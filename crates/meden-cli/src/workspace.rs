//! Filesystem tools shared by the interactive agent and headless checks.
//! Paths are resolved against the selected workspace and checked physically,
//! including existing symlink targets. This is not an OS process sandbox.

use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use meden_agent::tool::{Args, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::{Capability, Policy, Requirement};
use serde_json::json;
use serde_json::Value;

const OUTPUT_LIMIT: usize = 24_000;

#[derive(Clone, Copy)]
enum Operation {
    Read,
    List,
    Write,
    Edit,
}

struct FileTool {
    root: PathBuf,
    sig: ToolSig,
    operation: Operation,
}

/// Build real workspace tools and a matching read/write capability policy.
pub fn environment(root: &Path) -> Result<(Registry, Policy)> {
    let root = root
        .canonicalize()
        .with_context(|| format!("opening workspace {}", root.display()))?;
    anyhow::ensure!(root.is_dir(), "workspace must be a directory");
    let mut registry = Registry::new();
    for (name, operation, description, properties, required) in [
        ("read_file", Operation::Read, "Read a UTF-8 file in the workspace. Offset is a 1-based line number; limit defaults to 200 lines. Use further offsets for long files.",
         json!({"path":{"type":"string"},"offset":{"type":"integer","minimum":1},"limit":{"type":"integer","minimum":1,"maximum":2000}}), vec!["path"]),
        ("list_files", Operation::List, "List a workspace directory, with file types. Use path '.' for the root.",
         json!({"path":{"type":"string"}}), vec!["path"]),
        ("write_file", Operation::Write, "Create or replace a UTF-8 file in the workspace. Supply its complete content. Parent directories must already exist.",
         json!({"path":{"type":"string"},"content":{"type":"string"}}), vec!["path","content"]),
        ("edit_file", Operation::Edit, "Replace exactly one occurrence of old_text with new_text in a UTF-8 workspace file. Read the file first; ambiguous or missing matches fail without writing.",
         json!({"path":{"type":"string"},"old_text":{"type":"string"},"new_text":{"type":"string"}}), vec!["path","old_text","new_text"]),
    ] {
        let sig = ToolSig::new(name, description, json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}));
        registry.register(Box::new(FileTool { root: root.clone(), sig, operation }));
    }
    let root_scope = root.to_string_lossy();
    let grants = [
        format!("fs.read[{root_scope}]"),
        format!("fs.read[{root_scope}/*]"),
        format!("fs.write[{root_scope}/*]"),
    ];
    let labels: Vec<_> = grants.iter().map(String::as_str).collect();
    let policy = Policy::from_labels("workspace", &labels, &[])?;
    Ok((registry, policy))
}

impl FileTool {
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
                atomic_write(&path, &original.replacen(old, text(args, "new_text")?, 1))?;
                Ok("File edited.".into())
            }
        }
    }
}

impl Tool for FileTool {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }
    fn requirements(&self, args: &Args) -> Vec<Requirement> {
        let capability = match self.operation {
            Operation::Read | Operation::List => Capability::FsRead,
            Operation::Write | Operation::Edit => Capability::FsWrite,
        };
        let scope = self
            .resolve(args)
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        vec![Requirement::with_evidence(
            capability,
            scope.as_deref(),
            &self.sig.name,
        )]
    }
    fn call(&self, args: &Args) -> Result<Value, ToolError> {
        self.execute(args).map(Value::String)
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

    fn args(json: &serde_json::Value) -> Args {
        json.as_object()
            .expect("object")
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    #[test]
    fn real_files_can_be_written_read_and_edited_without_ambiguous_replacement() {
        let dir = tempfile::tempdir().expect("workspace");
        let (registry, policy) = environment(dir.path()).expect("environment");
        let write = registry.get("write_file").expect("write tool");
        let input = args(&json!({"path":"note.txt","content":"hello\nhello\n"}));
        assert!(meden_core::check(&write.requirements(&input), &policy).allowed);
        write.call(&input).expect("write");
        let edit = registry.get("edit_file").expect("edit tool");
        assert!(edit
            .call(&args(
                &json!({"path":"note.txt","old_text":"hello","new_text":"oops"})
            ))
            .is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join("note.txt")).expect("read"),
            "hello\nhello\n"
        );
        edit.call(&args(
            &json!({"path":"note.txt","old_text":"hello\nhello","new_text":"updated"}),
        ))
        .expect("unique edit");
        let result = registry
            .get("read_file")
            .expect("read tool")
            .call(&args(&json!({"path":"note.txt"})))
            .expect("read");
        assert_eq!(result.as_str(), Some("1: updated\n"));
    }

    #[test]
    fn traversal_is_refused_by_policy_and_tool_implementation() {
        let dir = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        let (registry, policy) = environment(dir.path()).expect("environment");
        let input = args(&json!({"path":outside.path().join("outside.txt"),"content":"bad"}));
        let write = registry.get("write_file").expect("write tool");
        assert!(!meden_core::check(&write.requirements(&input), &policy).allowed);
        assert!(write.call(&input).is_err());
        assert!(!outside.path().join("outside.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_parents_cannot_write_outside_the_workspace() {
        let dir = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).expect("symlink");
        let (registry, policy) = environment(dir.path()).expect("environment");
        let write = registry.get("write_file").expect("write tool");
        let input = args(&json!({"path":"link/outside.txt","content":"bad"}));
        assert!(!meden_core::check(&write.requirements(&input), &policy).allowed);
        assert!(write.call(&input).is_err());
        assert!(!outside.path().join("outside.txt").exists());
    }
}
