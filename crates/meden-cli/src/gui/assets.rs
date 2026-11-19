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

pub(super) struct Asset {
    pub bytes: Arc<[u8]>,
    pub mime: &'static str,
}
pub(super) struct Bundle {
    pub root: String,
    pub entry: String,
    pub revision: String,
    pub files: BTreeMap<String, Asset>,
    pub byte_count: usize,
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

pub(super) fn capture(
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
pub(super) fn mime(path: &str) -> Option<&'static str> {
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
