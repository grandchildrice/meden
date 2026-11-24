//! Freeze a bounded static application through the existing offline file worker.
use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use base64::Engine as _;
use meden_agent::tool::Args;
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// One immutable resource from the complete captured static bundle.
pub struct Asset {
    bytes: Arc<[u8]>,
    mime: &'static str,
}
/// Complete captured content; only successful preparation constructs a bundle.
pub struct Bundle {
    root: String,
    entry: String,
    revision: String,
    files: BTreeMap<String, Asset>,
    byte_count: usize,
}
impl Asset {
    /// Original immutable captured bytes.
    pub fn bytes(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }
    /// Original selected static media type.
    pub fn mime(&self) -> &'static str {
        self.mime
    }
}
impl Bundle {
    /// Original logical HTML entry, used by the host to construct its URL.
    pub fn entry(&self) -> &str {
        &self.entry
    }
    /// Look up captured content without filesystem reads or worker dispatch.
    pub fn asset(&self, path: &str) -> Option<&Asset> {
        self.files.get(path)
    }
    pub(crate) fn description(&self, url: String) -> Result<crate::view::preview::Data> {
        anyhow::ensure!(
            self.root.len() <= 4096 && self.entry.len() <= 4096 && url.len() <= 16384,
            "Preview description exceeds its view bounds"
        );
        Ok(crate::view::preview::Data {
            frame: format!(
                "sha256:{:x}",
                Sha256::digest(serde_json::to_vec(&(
                    "meden.preview.frame.v1",
                    &self.revision,
                    &url
                ))?)
            ),
            root: self.root.clone(),
            entry: self.entry.clone(),
            revision: self.revision.clone(),
            url,
            file_count: u32::try_from(self.files.len())?,
            byte_count: u32::try_from(self.byte_count)?,
            errors: Vec::new(),
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    kind: Kind,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    File,
    Directory,
    Symlink,
    Other,
}

/// Capture all allowed static resources through the original policy-bound file
/// worker; a failure returns no bundle or successful prefix.
pub fn capture(
    workspace: &Workspace,
    control: &Control,
    root: &str,
    entry: &str,
) -> Result<Bundle> {
    let root = crate::workspace::logical_path(root)
        .context("preview root leaves /workspace")?
        .components()
        .collect::<std::path::PathBuf>();
    let policy = read_policy(&root)?;
    anyhow::ensure!(
        !entry.is_empty()
            && Path::new(entry)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "preview entry must be a relative file path"
    );
    anyhow::ensure!(
        mime(entry) == Some("text/html; charset=utf-8"),
        "preview entry must be an HTML file"
    );
    let started = Instant::now();
    let mut pending = vec![(String::new(), 0usize)];
    let mut directories = 0usize;
    let mut files = BTreeMap::new();
    let mut byte_count = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        directories = directories.saturating_add(1);
        anyhow::ensure!(
            directories <= 64 && depth <= 8,
            "preview exceeds directory/depth limits"
        );
        let path = root.join(&directory);
        let value = observe(
            workspace,
            control,
            &policy,
            Operation::List,
            &path,
            json!({"entries":true}),
            started,
        )?;
        let entries: Vec<Entry> = serde_json::from_value(value)?;
        for item in entries {
            anyhow::ensure!(
                !item.name.is_empty()
                    && Path::new(&item.name).components().count() == 1
                    && Path::new(&item.name)
                        .components()
                        .all(|c| matches!(c, Component::Normal(_))),
                "invalid preview inventory name"
            );
            if item.name.starts_with('.') || matches!(item.name.as_str(), "node_modules" | "target")
            {
                continue;
            }
            let relative = if directory.is_empty() {
                item.name
            } else {
                format!("{directory}/{}", item.name)
            };
            match item.kind {
                Kind::Directory => pending.push((relative, depth.saturating_add(1))),
                Kind::File => {
                    let Some(mime) = mime(&relative) else {
                        continue;
                    };
                    anyhow::ensure!(files.len() < 256, "preview exceeds 256 files");
                    let encoded = observe(
                        workspace,
                        control,
                        &policy,
                        Operation::Snapshot,
                        &root.join(&relative),
                        json!({"encoding":"base64","max_bytes":4_194_304}),
                        started,
                    )?;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(encoded.as_str().context("invalid preview file response")?)?;
                    anyhow::ensure!(bytes.len() <= 4_194_304, "preview file exceeds 4 MiB");
                    byte_count = byte_count
                        .checked_add(bytes.len())
                        .context("preview size overflow")?;
                    anyhow::ensure!(byte_count <= 16_777_216, "preview exceeds 16 MiB");
                    files.insert(
                        relative,
                        Asset {
                            bytes: bytes.into(),
                            mime,
                        },
                    );
                }
                Kind::Symlink | Kind::Other => {} // Never recurse through inventory links or special entries.
            }
        }
    }
    anyhow::ensure!(
        files.contains_key(entry),
        "preview entry is missing from regular static files: {entry}"
    );
    finish(&root, entry, files, byte_count)
}
fn read_policy(root: &Path) -> Result<meden_core::Policy> {
    let root = root.to_str().context("preview root is not UTF-8")?;
    anyhow::ensure!(
        !root.contains(['*', '?']),
        "preview root must not contain policy wildcard characters"
    );
    Ok(meden_core::Policy::from_labels(
        "host-selected preview folder",
        &[&format!("fs.read[{root}]"), &format!("fs.read[{root}/*]")],
        &[],
    )?)
}
fn finish(
    root: &Path,
    entry: &str,
    files: BTreeMap<String, Asset>,
    byte_count: usize,
) -> Result<Bundle> {
    let root = root
        .strip_prefix("/workspace")?
        .to_str()
        .context("preview root is not UTF-8")?
        .to_owned();
    let manifest = files
        .iter()
        .map(|(name, asset)| (name, format!("{:x}", Sha256::digest(&asset.bytes))))
        .collect::<BTreeMap<_, _>>();
    let revision = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(
            &json!({"root":root,"entry":entry,"files":manifest})
        )?)
    );
    Ok(Bundle {
        root: if root.is_empty() { ".".into() } else { root },
        entry: entry.into(),
        revision,
        files,
        byte_count,
    })
}
fn observe(
    workspace: &Workspace,
    control: &Control,
    policy: &meden_core::Policy,
    operation: Operation,
    path: &Path,
    args: Value,
    started: Instant,
) -> Result<Value> {
    let remaining = Duration::from_secs(90)
        .checked_sub(started.elapsed())
        .context("preview observation timed out")?;
    let args: Args = serde_json::from_value(args)?;
    let request = Request::new(
        operation,
        path.to_str().context("preview path is not UTF-8")?.into(),
        args,
    );
    meden_runtime::file::run(
        workspace,
        control,
        &request,
        policy,
        &Limits {
            timeout: remaining.min(Duration::from_secs(30)),
            capture_bytes: 6_000_000,
            output_bytes: 6_000_000,
        },
    )
    .map_err(|e| anyhow::anyhow!("Preview observation failed: {e}"))
}
fn mime(path: &str) -> Option<&'static str> {
    Some(
        match Path::new(path)
            .extension()?
            .to_str()?
            .to_ascii_lowercase()
            .as_str()
        {
            "html" | "htm" => "text/html; charset=utf-8",
            "js" | "mjs" => "text/javascript; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "json" | "map" => "application/json",
            "txt" => "text/plain; charset=utf-8",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "ico" => "image/x-icon",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "wasm" => "application/wasm",
            _ => return None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::preview::Previews;
    fn fixture() -> Result<Bundle> {
        let bytes: Arc<[u8]> = Arc::from(&b"<p>Original captured bytes</p>"[..]);
        let length = bytes.len();
        finish(
            Path::new("/workspace/public"),
            "index.html",
            BTreeMap::from([(
                "index.html".into(),
                Asset {
                    bytes,
                    mime: "text/html; charset=utf-8",
                },
            )]),
            length,
        )
    }
    fn field(value: &Value, path: &str) -> Result<Value> {
        value
            .pointer(path)
            .cloned()
            .with_context(|| format!("Missing original field: {path}"))
    }
    fn revision(
        reader: &crate::view::preview::Reader,
    ) -> Result<crate::view::publication::ArtifactId<crate::view::preview::Artifact>> {
        Ok(serde_json::from_value(field(
            &serde_json::to_value(reader)?,
            "/revision",
        )?)?)
    }
    #[test]
    fn original_bundle_frame_and_frozen_reads_survive_diagnostics_but_not_replacement() -> Result<()>
    {
        let bundle = fixture()?;
        let views = Previews::default();
        let reader = views.reader();
        assert_eq!(serde_json::to_value(&reader)?, json!({"state":"idle"}));
        views.install(&bundle, "http://original.localhost:1234/index.html".into())?;
        let first_revision = revision(&reader)?;
        let first = reader
            .bytes(&first_revision)
            .ok_or_else(|| anyhow::anyhow!("Missing view"))?;
        for _ in 0..100 {
            assert!(Arc::ptr_eq(
                &first,
                &reader
                    .bytes(&first_revision)
                    .ok_or_else(|| anyhow::anyhow!("Lost view"))?
            ));
        }
        let original: Value = serde_json::from_slice(&first)?;
        assert_eq!(field(&original, "/data/revision")?, bundle.revision);
        assert_eq!(field(&original, "/data/file_count")?, 1);
        assert_eq!(field(&original, "/data/byte_count")?, bundle.byte_count);
        assert_eq!(field(&original, "/data/root")?, "public");
        assert_eq!(field(&original, "/data/entry")?, bundle.entry());
        assert_eq!(
            bundle.asset("index.html").map(Asset::bytes),
            Some(Arc::from(&b"<p>Original captured bytes</p>"[..]))
        );
        views.missing("missing.js");
        let second_revision = revision(&reader)?;
        assert_ne!(second_revision, first_revision);
        assert!(reader.bytes(&first_revision).is_none());
        let observed: Value = serde_json::from_slice(
            &reader
                .bytes(&second_revision)
                .ok_or_else(|| anyhow::anyhow!("Missing diagnostic view"))?,
        )?;
        assert_eq!(
            field(&observed, "/data/frame")?,
            field(&original, "/data/frame")?
        );
        assert_eq!(
            field(&observed, "/data/url")?,
            field(&original, "/data/url")?
        );
        assert_eq!(
            field(&observed, "/data/errors")?,
            json!(["Missing preview asset: missing.js"])
        );
        views.install(
            &bundle,
            "http://replacement.localhost:1234/index.html".into(),
        )?;
        let replacement: Value = serde_json::from_slice(
            &reader
                .bytes(&revision(&reader)?)
                .ok_or_else(|| anyhow::anyhow!("Missing replacement"))?,
        )?;
        assert_ne!(
            field(&replacement, "/data/frame")?,
            field(&original, "/data/frame")?
        );
        assert_eq!(
            field(&replacement, "/data/revision")?,
            field(&original, "/data/revision")?
        );
        assert_eq!(field(&replacement, "/data/errors")?, json!([]));
        views.clear();
        views.missing("late.js");
        assert_eq!(serde_json::to_value(&reader)?, json!({"state":"idle"}));
        assert!(reader.bytes(&second_revision).is_none());
        Ok(())
    }
    #[test]
    fn diagnostic_bounds_and_failed_publication_retire_the_whole_view() -> Result<()> {
        let bundle = fixture()?;
        let views = Previews::default();
        let reader = views.reader();
        views.install(&bundle, "http://original.localhost:1234/index.html".into())?;
        for _ in 0..25 {
            views.missing(&"🧪".repeat(700));
        }
        let last = revision(&reader)?;
        let data: Value = serde_json::from_slice(
            &reader
                .bytes(&last)
                .ok_or_else(|| anyhow::anyhow!("Missing bounded view"))?,
        )?;
        let errors_value = field(&data, "/data/errors")?;
        let errors = errors_value
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing errors"))?;
        assert_eq!(errors.len(), 20);
        for error in errors {
            let text = error
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Nonliteral diagnostic"))?;
            assert!(text.chars().count() <= 600);
            assert!(text.len() <= 2400);
            assert!(text.ends_with("… (path shortened)"));
        }
        assert!(views.install(&bundle, "x".repeat(16385)).is_err());
        assert_eq!(
            field(&serde_json::to_value(&reader)?, "/state")?,
            "unavailable"
        );
        assert!(reader.bytes(&last).is_none());
        views.missing("late.js");
        assert_eq!(
            field(&serde_json::to_value(&reader)?, "/state")?,
            "unavailable"
        );
        views.install(&bundle, "http://new.localhost:1234/index.html".into())?;
        assert_eq!(field(&serde_json::to_value(&reader)?, "/state")?, "ready");
        Ok(())
    }
}
