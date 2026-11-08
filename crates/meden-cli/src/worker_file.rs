//! Trusted filesystem worker, executed inside the private namespace.

use anyhow::{Context, Result};
use meden_agent::tool::{Args, ToolError};
use serde_json::Value;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

const OUTPUT_LIMIT: usize = 24_000;
#[derive(Clone, Copy)]
enum Operation {
    Read,
    List,
    Write,
    Edit,
}
struct FileOperation {
    root: PathBuf,
    operation: Operation,
}

pub fn run() -> Result<()> {
    let request: Value = serde_json::from_reader(std::io::stdin().lock())?;
    if request.get("operation").and_then(Value::as_str) == Some("snapshot") {
        let args: Args = serde_json::from_value(
            request
                .get("arguments")
                .cloned()
                .context("missing arguments")?,
        )?;
        let path = FileOperation {
            root: PathBuf::from("/workspace"),
            operation: Operation::Read,
        }
        .resolve(&args)?;
        let mut bytes = Vec::new();
        anyhow::ensure!(
            fs::metadata(&path)?.is_file(),
            "snapshot source must be a regular file"
        );
        fs::File::open(path)?
            .take(1_048_577)
            .read_to_end(&mut bytes)?;
        anyhow::ensure!(bytes.len() <= 1_048_576, "export exceeds 1 MiB");
        serde_json::to_writer(std::io::stdout().lock(), &bytes)?;
        return Ok(());
    }
    let operation = match request.get("operation").and_then(Value::as_str) {
        Some("read_file") => Operation::Read,
        Some("list_files") => Operation::List,
        Some("write_file") => Operation::Write,
        Some("edit_file") => Operation::Edit,
        _ => anyhow::bail!("unknown file operation"),
    };
    let args: Args = serde_json::from_value(
        request
            .get("arguments")
            .cloned()
            .context("missing arguments")?,
    )?;
    let result = FileOperation {
        root: PathBuf::from("/workspace"),
        operation,
    }
    .execute(&args)?;
    serde_json::to_writer(std::io::stdout().lock(), &Value::String(result))?;
    Ok(())
}

impl FileOperation {
    fn resolve(&self, args: &Args) -> Result<PathBuf, ToolError> {
        let requested = PathBuf::from(text(args, "path")?);
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
            parent.canonicalize().map_err(failed)?.join(name)
        };
        if !path.starts_with(&self.root) {
            return Err(ToolError::Failed(
                "path leaves the selected workspace".into(),
            ));
        }
        Ok(path)
    }

    fn execute(&self, args: &Args) -> Result<String, ToolError> {
        let path = self.resolve(args)?;
        match self.operation {
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
                Ok(output)
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
                Ok(output)
            }
            Operation::Write => {
                atomic_write(&path, text(args, "content")?)?;
                Ok("File written.".into())
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
                Ok("File edited.".into())
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

    fn execute(root: &Path, operation: Operation, args: Value) -> Result<String, ToolError> {
        let args = serde_json::from_value(args).expect("arguments");
        FileOperation {
            root: root.to_owned(),
            operation,
        }
        .execute(&args)
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
}
