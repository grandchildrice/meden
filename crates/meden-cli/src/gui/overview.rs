//! Read-only workspace inventory through the policy-bound file worker. This is
//! a lexical navigation index, never compiler evidence or contract acceptance.
use super::{lock, View};
use anyhow::{Context, Result};
use base64::Engine as _;
use meden_agent::tool::Args;
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
pub(super) struct Status {
    pub state: &'static str,
    pub scanned: usize,
    pub total: usize,
    pub revision: Option<String>,
    pub error: Option<String>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            state: "loading",
            scanned: 0,
            total: 0,
            revision: None,
            error: None,
        }
    }
}
#[derive(Serialize)]
pub(super) struct File {
    path: String,
    sha256: String,
    bytes: usize,
    declarations: usize,
    theorems: usize,
    imports: Vec<String>,
}
#[derive(Serialize)]
pub(super) struct Snapshot {
    pub revision: String,
    files: Vec<File>,
    excluded: Vec<&'static str>,
}
#[derive(Deserialize)]
struct Entry {
    name: String,
    kind: String,
}
const EXCLUDED: &[&str] = &["node_modules", "target", "dist", "__pycache__"];

pub(super) fn refresh(view: &Mutex<View>, workspace: &Workspace, control: &Control) -> Result<()> {
    {
        let mut view = lock(view);
        view.overview = Status::default();
        view.overview_snapshot = None;
        view.activity = "ワークスペース全体のファイルと契約・証明を読み込み中".into();
    }
    let result = capture(view, workspace, control);
    let mut view = lock(view);
    match result {
        Ok(snapshot) => {
            view.overview.state = "ready";
            view.overview.scanned = snapshot.files.len();
            view.overview.revision = Some(snapshot.revision.clone());
            view.overview_snapshot = Some(Arc::new(snapshot));
            view.activity = "ワークスペース全体を読み込みました".into();
            Ok(())
        }
        Err(error) => {
            view.overview.state = "failed";
            view.overview.error = Some(format!("{error:#}"));
            Err(error)
        }
    }
}

fn capture(view: &Mutex<View>, workspace: &Workspace, control: &Control) -> Result<Snapshot> {
    let started = Instant::now();
    let mut pending = vec![(String::new(), 0usize)];
    let mut paths = Vec::new();
    let mut directories = 0usize;
    let mut entry_count = 0usize;
    while !pending.is_empty() {
        let batch = pending
            .drain(pending.len().saturating_sub(128)..)
            .collect::<Vec<_>>();
        directories = directories.saturating_add(batch.len());
        anyhow::ensure!(
            directories <= 65_536 && batch.iter().all(|(_, depth)| *depth <= 32),
            "Workspace index exceeds directory/depth limits"
        );
        let values = observe_batch(
            workspace,
            control,
            Operation::List,
            &batch
                .iter()
                .map(|(path, _)| path.clone())
                .collect::<Vec<_>>(),
            &json!({"entries":true,"max_entries":10000}),
            started,
        )?;
        for ((directory, depth), value) in batch.into_iter().zip(values) {
            let entries: Vec<Entry> = serde_json::from_value(value)?;
            entry_count = entry_count.saturating_add(entries.len());
            anyhow::ensure!(
                entry_count <= 1_000_000,
                "Workspace index exceeds one million directory entries"
            );
            for entry in entries {
                anyhow::ensure!(
                    !entry.name.is_empty()
                        && Path::new(&entry.name).components().count() == 1
                        && Path::new(&entry.name)
                            .components()
                            .all(|c| matches!(c, std::path::Component::Normal(_))),
                    "Invalid workspace inventory entry"
                );
                if entry.name.starts_with('.') || EXCLUDED.contains(&entry.name.as_str()) {
                    continue;
                }
                let path = if directory.is_empty() {
                    entry.name
                } else {
                    format!("{directory}/{}", entry.name)
                };
                if entry.kind == "directory" {
                    pending.push((path, depth.saturating_add(1)));
                } else if entry.kind == "file"
                    && matches!(
                        Path::new(&path).extension().and_then(|x| x.to_str()),
                        Some("lean" | "rs")
                    )
                {
                    paths.push(path);
                    anyhow::ensure!(
                        paths.len() <= 10_000,
                        "Workspace index exceeds 10000 source files"
                    );
                }
                // Inventory symlinks and special files are never followed.
            }
        }
    }
    paths.sort();
    lock(view).overview.total = paths.len();
    let files = source_files(view, workspace, control, &paths, started)?;
    let revision = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&files)?));
    Ok(Snapshot {
        revision,
        files,
        excluded: EXCLUDED.to_vec(),
    })
}

fn source_files(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    started: Instant,
) -> Result<Vec<File>> {
    let cursor = AtomicUsize::new(0);
    let bytes = AtomicUsize::new(0);
    let files = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| -> Result<()> {
        let handles = (0..4)
            .map(|_| {
                scope.spawn(|| -> Result<()> {
                    loop {
                        let index = cursor.fetch_add(8, Ordering::SeqCst);
                        if index >= paths.len() {
                            break;
                        }
                        let batch = paths
                            .get(index..index.saturating_add(8).min(paths.len()))
                            .context("Invalid source batch")?;
                        let values = observe_batch(
                            workspace,
                            control,
                            Operation::Snapshot,
                            batch,
                            &json!({"encoding":"base64","max_bytes":1_048_576}),
                            started,
                        )?;
                        for (path, value) in batch.iter().zip(values) {
                            let source = base64::engine::general_purpose::STANDARD.decode(
                                value
                                    .as_str()
                                    .context("Invalid workspace source response")?,
                            )?;
                            anyhow::ensure!(
                                bytes
                                    .fetch_add(source.len(), Ordering::SeqCst)
                                    .saturating_add(source.len())
                                    <= 134_217_728,
                                "Workspace sources exceed 128 MiB"
                            );
                            let text = std::str::from_utf8(&source)
                                .context("Workspace source is not UTF-8")?;
                            let (declarations, theorems, imports) = headers(
                                text,
                                Path::new(path).extension().is_some_and(|ext| ext == "lean"),
                            );
                            lock(&files).insert(
                                path.clone(),
                                File {
                                    path: path.clone(),
                                    sha256: format!("{:x}", Sha256::digest(&source)),
                                    bytes: source.len(),
                                    declarations,
                                    theorems,
                                    imports,
                                },
                            );
                            let mut view = lock(view);
                            view.overview.scanned = view.overview.scanned.saturating_add(1);
                        }
                    }
                    Ok(())
                })
            })
            .collect::<Vec<_>>();
        let mut failure = None;
        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| anyhow::anyhow!("Workspace index worker stopped"))?;
            if let Err(error) = result {
                failure.get_or_insert(error);
            }
        }
        failure.map_or(Ok(()), Err)
    })?;
    let files = files
        .into_inner()
        .map_err(|_| anyhow::anyhow!("Workspace index unavailable"))?
        .into_values()
        .collect::<Vec<_>>();
    anyhow::ensure!(files.len() == paths.len(), "Incomplete workspace index");
    Ok(files)
}

fn observe_batch(
    workspace: &Workspace,
    control: &Control,
    operation: Operation,
    paths: &[String],
    payload: &Value,
    started: Instant,
) -> Result<Vec<Value>> {
    anyhow::ensure!(
        started.elapsed() < Duration::from_secs(300),
        "Workspace index exceeded 300 seconds"
    );
    let requests = paths
        .iter()
        .map(|path| -> Result<_> {
            let path = crate::workspace::logical_path(path)
                .context("Workspace index path leaves the selected root")?;
            Ok(Request::new(
                operation,
                path.to_str()
                    .context("Workspace index path is not UTF-8")?
                    .into(),
                serde_json::from_value::<Args>(payload.clone())?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    meden_runtime::file::read_batch(
        workspace,
        control,
        &requests,
        &crate::workspace::file_policy()?,
        &Limits {
            timeout: Duration::from_secs(30),
            capture_bytes: 16_777_216,
            output_bytes: 16_777_216,
        },
    )
    .map_err(anyhow::Error::new)
}

// Mask nested comments and string literals before counting declarations or
// reading Lean import lines. Names and dependencies are lexical candidates.
fn headers(source: &str, lean: bool) -> (usize, usize, Vec<String>) {
    let text = mask(source, lean);
    let mut declarations = 0usize;
    let mut theorems = 0usize;
    let mut imports = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace().peekable();
        while words.peek().is_some_and(|word| {
            matches!(
                *word,
                "public"
                    | "private"
                    | "protected"
                    | "noncomputable"
                    | "unsafe"
                    | "partial"
                    | "async"
            ) || word.starts_with("pub")
        }) {
            words.next();
        }
        let Some(kind) = words.next() else {
            continue;
        };
        if lean && kind == "import" {
            imports.extend(words.map(str::to_owned));
        }
        if matches!(
            kind,
            "theorem"
                | "lemma"
                | "def"
                | "structure"
                | "class"
                | "inductive"
                | "abbrev"
                | "opaque"
                | "axiom"
                | "constant"
                | "fn"
                | "struct"
                | "enum"
                | "trait"
                | "type"
        ) {
            declarations = declarations.saturating_add(1);
            if matches!(kind, "theorem" | "lemma") {
                theorems = theorems.saturating_add(1);
            }
        }
    }
    (declarations, theorems, imports)
}

fn mask(source: &str, lean: bool) -> String {
    let mut chars = source.chars().peekable();
    let mut output = String::new();
    let mut depth = 0usize;
    let mut line = false;
    let mut quoted = false;
    while let Some(c) = chars.next() {
        let next = chars.peek().copied();
        if line {
            output.push(if c == '\n' { '\n' } else { ' ' });
            if c == '\n' {
                line = false;
            }
        } else if depth > 0 {
            if (c, next)
                == if lean {
                    ('/', Some('-'))
                } else {
                    ('/', Some('*'))
                }
            {
                depth = depth.saturating_add(1);
                chars.next();
                output.push_str("  ");
            } else if (c, next)
                == if lean {
                    ('-', Some('/'))
                } else {
                    ('*', Some('/'))
                }
            {
                depth = depth.saturating_sub(1);
                chars.next();
                output.push_str("  ");
            } else {
                output.push(if c == '\n' { '\n' } else { ' ' });
            }
        } else if quoted {
            output.push(if c == '\n' { '\n' } else { ' ' });
            if c == '\\' {
                if let Some(escaped) = chars.next() {
                    output.push(if escaped == '\n' { '\n' } else { ' ' });
                }
            } else if c == '"' {
                quoted = false;
            }
        } else if (c, next)
            == if lean {
                ('-', Some('-'))
            } else {
                ('/', Some('/'))
            }
        {
            line = true;
            chars.next();
            output.push_str("  ");
        } else if (c, next)
            == if lean {
                ('/', Some('-'))
            } else {
                ('/', Some('*'))
            }
        {
            depth = 1;
            chars.next();
            output.push_str("  ");
        } else if c == '"' {
            quoted = true;
            output.push(' ');
        } else {
            output.push(c);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_navigation_does_not_count_imports_or_theorems_in_comments_and_literals() {
        let source = "/- import Fake\n/- theorem fake : False -/\n-/\npublic import Real.Module\n-- theorem fake : False\ndef text := \"theorem fake : False\"\npublic theorem actual (n : Nat) : n = n := rfl\n";
        assert_eq!(
            headers(source, true),
            (2, 1, vec!["Real.Module".to_owned()])
        );
    }
}
