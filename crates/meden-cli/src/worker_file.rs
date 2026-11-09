//! Trusted filesystem worker, executed inside the private namespace.

use anyhow::Result;
use meden_agent::tool::{Args, ToolError};
use meden_core::file::{Operation, Request};
use serde_json::Value;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

const OUTPUT_LIMIT: usize = 24_000;
struct FileOperation {
    root: PathBuf,
    operation: Operation,
}

pub fn run() -> Result<()> {
    let request: Request<Args> = serde_json::from_reader(std::io::stdin().lock())?;
    let (operation, path, payload) = request.into_parts();
    let path = String::from(path);
    let result = FileOperation {
        root: PathBuf::from("/workspace"),
        operation,
    }
    .execute(Path::new(&path), &payload)?;
    serde_json::to_writer(std::io::stdout().lock(), &result)?;
    Ok(())
}

impl FileOperation {
    #[allow(clippy::unnecessary_debug_formatting)] // Keep untrusted paths quoted and escaped.
    fn resolve(&self, requested: &Path) -> Result<PathBuf, ToolError> {
        let requested = requested.to_path_buf();
        let candidate = if requested.is_absolute() {
            requested
        } else {
            self.root.join(requested)
        };
        let path = if candidate.exists() || candidate.is_symlink() {
            candidate.canonicalize().map_err(failed)?
        } else {
            let parent = candidate
                .parent()
                .ok_or_else(|| ToolError::Failed("path has no parent".into()))?;
            let name = candidate
                .file_name()
                .ok_or_else(|| ToolError::Failed("path has no filename".into()))?;
            parent
                .canonicalize()
                .map_err(|error| {
                    ToolError::Failed(format!(
                        "cannot resolve parent directory {parent:?}: {error}. \
                         File tools do not create directories; use bash for directory operations."
                    ))
                })?
                .join(name)
        };
        if !path.starts_with(&self.root) {
            return Err(ToolError::Failed(
                "path leaves the selected workspace".into(),
            ));
        }
        if matches!(self.operation, Operation::Write) {
            if path == self.root {
                return Err(ToolError::Failed(
                    "cannot replace the workspace directory with a file".into(),
                ));
            }
            let parent = path
                .parent()
                .ok_or_else(|| ToolError::Failed("path has no parent".into()))?;
            if !fs::metadata(parent).map_err(failed)?.is_dir() {
                return Err(ToolError::Failed(format!(
                    "parent path {parent:?} is not a directory. \
                     Writing empty content creates a regular file, not a directory. \
                     Inspect the existing entry; use bash for directory operations."
                )));
            }
        }
        Ok(path)
    }

    fn execute(&self, requested: &Path, args: &Args) -> Result<Value, ToolError> {
        let path = self.resolve(requested)?;
        match self.operation {
            Operation::Snapshot => {
                if !fs::metadata(&path).map_err(failed)?.is_file() {
                    return Err(ToolError::Failed(
                        "snapshot source must be a regular file".into(),
                    ));
                }
                let mut bytes = Vec::new();
                fs::File::open(path)
                    .map_err(failed)?
                    .take(1_048_577)
                    .read_to_end(&mut bytes)
                    .map_err(failed)?;
                if bytes.len() > 1_048_576 {
                    return Err(ToolError::Failed("export exceeds 1 MiB".into()));
                }
                serde_json::to_value(bytes).map_err(failed)
            }
            Operation::Read => {
                let offset = integer(args, "offset", 1)?;
                let limit = integer(args, "limit", 200)?.min(2000);
                if offset == 0 || limit == 0 {
                    return Err(ToolError::Failed(
                        "offset and limit must be positive".into(),
                    ));
                }
                let file = fs::File::open(path).map_err(failed)?;
                let mut output = String::new();
                for (index, line) in BufReader::new(file)
                    .lines()
                    .enumerate()
                    .skip(offset.saturating_sub(1))
                    .take(limit)
                {
                    let line = line.map_err(failed)?;
                    if output.len().saturating_add(line.len()) > OUTPUT_LIMIT {
                        output.push_str("\n[output truncated; request a smaller line range]");
                        break;
                    }
                    let _ = writeln!(output, "{}: {line}", index.saturating_add(1));
                }
                Ok(Value::String(output))
            }
            Operation::List => {
                let mut entries = fs::read_dir(path)
                    .map_err(failed)?
                    .take(501)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(failed)?;
                entries.sort_by_key(std::fs::DirEntry::file_name);
                let mut output = String::new();
                for entry in entries.iter().take(500) {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let suffix = if entry.file_type().map_err(failed)?.is_dir() {
                        "/"
                    } else {
                        ""
                    };
                    let _ = writeln!(output, "{name}{suffix}");
                    if output.len() > OUTPUT_LIMIT {
                        break;
                    }
                }
                if entries.len() > 500 {
                    output.push_str("[more entries omitted]\n");
                }
                Ok(Value::String(output))
            }
            Operation::Write => {
                atomic_write(&path, text(args, "content")?)?;
                Ok(Value::String("File written.".into()))
            }
            Operation::Edit => {
                let original = fs::read_to_string(&path).map_err(failed)?;
                let old = text(args, "old_text")?;
                if old.is_empty() || original.matches(old).count() != 1 {
                    return Err(ToolError::Failed(
                        "old_text must match exactly once; file was not changed".into(),
                    ));
                }
                let new = text(args, "new_text")?;
                if old == new {
                    return Err(ToolError::Failed(
                        "new_text is identical to old_text; file was not changed".into(),
                    ));
                }
                atomic_write(&path, &original.replacen(old, new, 1))?;
                Ok(Value::String("File edited.".into()))
            }
        }
    }
}

fn failed(error: impl std::fmt::Display) -> ToolError {
    ToolError::Failed(error.to_string())
}

fn text<'a>(args: &'a Args, name: &str) -> Result<&'a str, ToolError> {
    args.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Failed(format!("{name} must be text")))
}

fn integer(args: &Args, name: &str, default: usize) -> Result<usize, ToolError> {
    match args.get(name) {
        None => Ok(default),
        Some(value) if value.as_u64().is_some() => {
            usize::try_from(value.as_u64().unwrap_or(0)).map_err(failed)
        }
        Some(_) => Err(ToolError::Failed(format!("{name} must be an integer"))),
    }
}

fn atomic_write(path: &Path, content: &str) -> Result<(), ToolError> {
    let parent = path
        .parent()
        .ok_or_else(|| ToolError::Failed("path has no parent".into()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(failed)?;
    temporary.write_all(content.as_bytes()).map_err(failed)?;
    if let Ok(metadata) = fs::metadata(path) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())
            .map_err(failed)?;
    }
    temporary.persist(path).map_err(failed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use serde_json::json;

    fn execute(root: &Path, operation: Operation, args: Value) -> Result<Value, ToolError> {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .expect("path")
            .to_owned();
        let mut payload: Args = serde_json::from_value(args).expect("arguments");
        payload.remove("path");
        FileOperation {
            root: root.to_owned(),
            operation,
        }
        .execute(Path::new(&path), &payload)
    }

    #[test]
    fn real_files_are_edited_only_for_one_exact_match() {
        let root = tempfile::tempdir().expect("workspace");
        execute(
            root.path(),
            Operation::Write,
            json!({"path":"note", "content":"hello\nhello\n"}),
        )
        .expect("write");
        assert!(execute(
            root.path(),
            Operation::Edit,
            json!({"path":"note", "old_text":"hello", "new_text":"bad"})
        )
        .is_err());
        assert_eq!(
            fs::read_to_string(root.path().join("note")).expect("read"),
            "hello\nhello\n"
        );
        execute(
            root.path(),
            Operation::Edit,
            json!({"path":"note", "old_text":"hello\nhello", "new_text":"updated"}),
        )
        .expect("edit");
        assert_eq!(
            execute(root.path(), Operation::Read, json!({"path":"note"})).expect("read"),
            "1: updated\n"
        );
        assert!(execute(
            root.path(),
            Operation::Edit,
            json!({"path":"note", "old_text":"updated", "new_text":"updated"})
        )
        .is_err_and(|e| e.to_string().contains("identical")));
        assert_eq!(
            fs::read_to_string(root.path().join("note")).expect("unchanged file"),
            "updated\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn outside_paths_and_symlinked_parents_are_rejected() {
        let root = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).expect("symlink");
        for path in [
            outside.path().join("secret"),
            root.path().join("link/secret"),
        ] {
            assert!(execute(
                root.path(),
                Operation::Write,
                json!({"path":path, "content":"bad"})
            )
            .is_err());
        }
        assert!(!outside.path().join("secret").exists());
    }

    #[test]
    fn invalid_parents_are_reported_without_creating_or_replacing_entries() {
        let root = tempfile::tempdir().expect("workspace");
        assert!(execute(
            root.path(),
            Operation::Write,
            json!({"path":".", "content":"not a directory"})
        )
        .is_err_and(|e| e.to_string().contains("workspace directory")));
        assert!(root.path().is_dir());
        let write = || {
            execute(
                root.path(),
                Operation::Write,
                json!({"path":"src/kernel.rs", "content":"candidate"}),
            )
        };
        assert!(write().is_err_and(|e| e.to_string().contains("parent directory")));
        assert!(!root.path().join("src").exists());
        fs::write(root.path().join("src"), "existing data").expect("existing file");
        assert!(write().is_err_and(|e| e.to_string().contains("not a directory")));
        assert_eq!(
            fs::read_to_string(root.path().join("src")).expect("unchanged"),
            "existing data"
        );
        fs::remove_file(root.path().join("src")).expect("test setup");
        fs::create_dir(root.path().join("src")).expect("parent directory");
        write().expect("write under existing directory");
        assert_eq!(
            fs::read_to_string(root.path().join("src/kernel.rs")).expect("candidate"),
            "candidate"
        );
    }
}
