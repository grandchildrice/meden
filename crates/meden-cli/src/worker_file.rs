//! Trusted filesystem worker, executed inside the private namespace.

use anyhow::Result;
use meden_agent::tool::{Args, ToolError};
use meden_core::file::{Operation, Request};
use serde_json::Value;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
mod pinned;
#[cfg(target_os = "linux")]
use pinned::Access;

const OUTPUT_LIMIT: usize = 24_000;
struct FileOperation {
    root: PathBuf,
    operation: Operation,
}

pub fn run() -> Result<()> {
    let (request, rules): (Request<Args>, meden_core::Policy) =
        serde_json::from_reader(std::io::stdin().lock())?;
    let (operation, path, payload) = request.into_parts();
    let path = String::from(path);
    let result = FileOperation {
        root: PathBuf::from("/workspace"),
        operation,
    }
    .execute(Path::new(&path), &payload, &rules)?;
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

    #[cfg(target_os = "linux")]
    fn execute(
        &self,
        requested: &Path,
        args: &Args,
        rules: &meden_core::Policy,
    ) -> Result<Value, ToolError> {
        let (request, requirements) = Request::new(
            self.operation,
            requested
                .to_str()
                .ok_or_else(|| ToolError::Failed("requested path is not UTF-8".into()))?
                .into(),
            args,
        )
        .prepare();
        if !meden_core::allowed(&requirements, rules) {
            return Err(ToolError::Failed(
                "requested file effects are not permitted".into(),
            ));
        }
        let path = self.resolve(requested)?;
        let canonical = path
            .to_str()
            .ok_or_else(|| ToolError::Failed("resolved path is not UTF-8".into()))?
            .into();
        let access = Access::open(&self.root, &path, self.operation)?;
        let (request, _, access) = request
            .bind(canonical, access)
            .consume(rules)
            .ok_or_else(|| ToolError::Failed("resolved file effects are not permitted".into()))?;
        let (operation, _, args) = request.into_parts();
        Self {
            root: self.root.clone(),
            operation,
        }
        .execute_access(access, args)
    }

    #[cfg(not(target_os = "linux"))]
    fn execute(
        &self,
        _requested: &Path,
        _args: &Args,
        _rules: &meden_core::Policy,
    ) -> Result<Value, ToolError> {
        Err(ToolError::Failed(
            "isolated file access requires Linux".into(),
        ))
    }

    #[cfg(target_os = "linux")]
    fn execute_access(&self, access: Access, args: &Args) -> Result<Value, ToolError> {
        match (self.operation, access) {
            (Operation::Snapshot, Access::File(file)) => {
                let mut bytes = Vec::new();
                pinned::readable(&file)?
                    .take(1_048_577)
                    .read_to_end(&mut bytes)
                    .map_err(failed)?;
                if bytes.len() > 1_048_576 {
                    return Err(ToolError::Failed("export exceeds 1 MiB".into()));
                }
                serde_json::to_value(bytes).map_err(failed)
            }
            (Operation::Read, Access::File(file)) => {
                let offset = integer(args, "offset", 1)?;
                let limit = integer(args, "limit", 200)?.min(2000);
                if offset == 0 || limit == 0 {
                    return Err(ToolError::Failed(
                        "offset and limit must be positive".into(),
                    ));
                }
                let mut output = String::new();
                let file = pinned::readable(&file)?;
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
            (Operation::List, Access::Directory(directory)) => {
                let mut entries = fs::read_dir(pinned::descriptor_path(&directory))
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
            (
                Operation::Write,
                Access::Replacement {
                    parent,
                    name,
                    original,
                },
            ) => {
                pinned::atomic_write(&parent, &name, original.as_ref(), text(args, "content")?)?;
                Ok(Value::String("File written.".into()))
            }
            (
                Operation::Edit,
                Access::Replacement {
                    parent,
                    name,
                    original: Some(file),
                },
            ) => {
                let content = edited(&file, args)?;
                pinned::atomic_write(&parent, &name, Some(&file), &content)?;
                Ok(Value::String("File edited.".into()))
            }
            _ => Err(ToolError::Failed(
                "file operation does not match pinned access".into(),
            )),
        }
    }
}

#[cfg(target_os = "linux")]
fn edited(file: &fs::File, args: &Args) -> Result<String, ToolError> {
    let mut original = String::new();
    pinned::readable(file)?
        .read_to_string(&mut original)
        .map_err(failed)?;
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
    Ok(original.replacen(old, new, 1))
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

#[cfg(all(test, target_os = "linux"))]
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
        .execute(
            Path::new(&path),
            &payload,
            &meden_core::Policy::from_labels("native worker", &["fs.read", "fs.write"], &[])
                .expect("test policy"),
        )
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

    #[test]
    fn internal_relative_and_absolute_links_keep_their_file_semantics() {
        let root = tempfile::tempdir().expect("workspace");
        fs::create_dir(root.path().join("real")).expect("directory");
        fs::write(root.path().join("real/note"), "original").expect("file");
        std::os::unix::fs::symlink("real", root.path().join("relative")).expect("relative link");
        std::os::unix::fs::symlink(root.path().join("real/note"), root.path().join("absolute"))
            .expect("absolute link");
        assert_eq!(
            execute(
                root.path(),
                Operation::Read,
                json!({"path":"relative/note"})
            )
            .expect("read"),
            "1: original\n"
        );
        assert_eq!(
            execute(root.path(), Operation::Snapshot, json!({"path":"absolute"}))
                .expect("snapshot"),
            json!(b"original")
        );
        execute(
            root.path(),
            Operation::Write,
            json!({"path":"absolute","content":"changed"}),
        )
        .expect("replace linked target");
        execute(
            root.path(),
            Operation::Edit,
            json!({"path":"relative/note","old_text":"changed","new_text":"edited"}),
        )
        .expect("edit linked target");
        assert!(root.path().join("absolute").is_symlink());
        assert_eq!(
            fs::read_to_string(root.path().join("real/note")).expect("target"),
            "edited"
        );
        assert_eq!(
            execute(root.path(), Operation::List, json!({"path":"relative"})).expect("list"),
            "note\n"
        );
        execute(
            root.path(),
            Operation::Write,
            json!({"path":"relative/new","content":"created"}),
        )
        .expect("new file through linked parent");
        assert_eq!(
            fs::read_to_string(root.path().join("real/new")).expect("new target"),
            "created"
        );
    }

    #[test]
    fn resolved_parent_substitution_is_refused_before_any_content_access() {
        for operation in [
            Operation::Read,
            Operation::Snapshot,
            Operation::List,
            Operation::Write,
            Operation::Edit,
        ] {
            let root = tempfile::tempdir().expect("workspace");
            let outside = tempfile::tempdir().expect("outside");
            fs::create_dir(root.path().join("parent")).expect("parent");
            for directory in [root.path().join("parent"), outside.path().to_owned()] {
                if operation == Operation::List {
                    fs::create_dir(directory.join("entry")).expect("list target");
                } else {
                    fs::write(directory.join("entry"), "original").expect("file target");
                }
            }
            let worker = FileOperation {
                root: root.path().to_owned(),
                operation,
            };
            let resolved = worker
                .resolve(Path::new("parent/entry"))
                .expect("original resolution");
            fs::rename(root.path().join("parent"), root.path().join("held")).expect("move parent");
            std::os::unix::fs::symlink(outside.path(), root.path().join("parent"))
                .expect("substitute parent");
            assert!(
                Access::open(root.path(), &resolved, operation).is_err(),
                "{operation:?} must not follow a substituted parent"
            );
        }
    }

    #[test]
    fn resolved_leaf_substitution_is_refused_including_missing_write_targets() {
        for operation in [
            Operation::Read,
            Operation::Snapshot,
            Operation::List,
            Operation::Write,
            Operation::Edit,
        ] {
            for existed in [true, false] {
                if !existed && operation != Operation::Write {
                    continue;
                }
                let root = tempfile::tempdir().expect("workspace");
                let outside = tempfile::tempdir().expect("outside");
                let entry = root.path().join("entry");
                let secret = outside.path().join("secret");
                if operation == Operation::List {
                    fs::create_dir(&entry).expect("directory");
                    fs::create_dir(&secret).expect("outside directory");
                } else {
                    if existed {
                        fs::write(&entry, "original").expect("file");
                    }
                    fs::write(&secret, "private").expect("outside file");
                }
                let worker = FileOperation {
                    root: root.path().to_owned(),
                    operation,
                };
                let resolved = worker
                    .resolve(Path::new("entry"))
                    .expect("original resolution");
                if operation == Operation::List {
                    fs::remove_dir(&entry).expect("remove directory");
                } else if existed {
                    fs::remove_file(&entry).expect("remove file");
                }
                std::os::unix::fs::symlink(&secret, &entry).expect("substitute leaf");
                assert!(
                    Access::open(root.path(), &resolved, operation).is_err(),
                    "{operation:?} must not follow or treat a substituted symlink as missing"
                );
            }
        }
    }

    #[test]
    fn pinned_reads_lists_and_replacements_ignore_later_path_substitution() {
        for operation in [
            Operation::Read,
            Operation::Snapshot,
            Operation::List,
            Operation::Write,
            Operation::Edit,
        ] {
            let root = tempfile::tempdir().expect("workspace");
            let outside = tempfile::tempdir().expect("outside");
            fs::create_dir(root.path().join("parent")).expect("directory");
            fs::write(root.path().join("parent/note"), "original").expect("original");
            fs::write(outside.path().join("note"), "private").expect("secret");
            fs::write(outside.path().join("secret-name"), "private").expect("secret name");
            let worker = FileOperation {
                root: root.path().to_owned(),
                operation,
            };
            let requested = if operation == Operation::List {
                "parent"
            } else {
                "parent/note"
            };
            let resolved = worker
                .resolve(Path::new(requested))
                .expect("original resolution");
            let access = Access::open(root.path(), &resolved, operation).expect("pin original");
            fs::rename(root.path().join("parent"), root.path().join("held")).expect("move parent");
            std::os::unix::fs::symlink(outside.path(), root.path().join("parent"))
                .expect("substitute parent");
            if operation != Operation::List {
                fs::remove_file(root.path().join("held/note")).expect("unlink original target");
                std::os::unix::fs::symlink(
                    outside.path().join("note"),
                    root.path().join("held/note"),
                )
                .expect("substitute leaf after pin");
            }
            let args = serde_json::from_value(
                json!({"content":"replaced","old_text":"original","new_text":"edited"}),
            )
            .expect("arguments");
            let result = worker
                .execute_access(access, &args)
                .expect("execute original handles");
            match operation {
                Operation::Read => assert_eq!(result, "1: original\n"),
                Operation::Snapshot => assert_eq!(result, json!(b"original")),
                Operation::List => assert_eq!(result, "note\n"),
                Operation::Write | Operation::Edit => {
                    assert!(!root.path().join("held/note").is_symlink());
                    assert_eq!(
                        fs::read_to_string(root.path().join("held/note")).expect("original parent"),
                        if operation == Operation::Write {
                            "replaced"
                        } else {
                            "edited"
                        }
                    );
                }
            }
            assert_eq!(
                fs::read_to_string(outside.path().join("note")).expect("outside unchanged"),
                "private"
            );
            assert_eq!(
                fs::read_dir(outside.path())
                    .expect("outside entries")
                    .count(),
                2
            );
        }
    }

    #[test]
    fn special_entries_are_refused_without_blocking_or_replacing_them() {
        use nix::sys::stat::Mode;
        use nix::unistd::mkfifo;
        let root = tempfile::tempdir().expect("workspace");
        let fifo = root.path().join("pipe");
        mkfifo(&fifo, Mode::S_IRUSR | Mode::S_IWUSR).expect("FIFO");
        for operation in [
            Operation::Read,
            Operation::Snapshot,
            Operation::Write,
            Operation::Edit,
            Operation::List,
        ] {
            let worker = FileOperation {
                root: root.path().to_owned(),
                operation,
            };
            let resolved = worker.resolve(Path::new("pipe")).expect("FIFO resolution");
            assert!(
                Access::open(root.path(), &resolved, operation).is_err(),
                "{operation:?} must reject a FIFO"
            );
        }
        assert!(!fs::metadata(fifo).expect("entry preserved").is_file());
    }

    #[test]
    fn lookup_errors_cannot_be_treated_as_permission_to_create_a_file() {
        let root = tempfile::tempdir().expect("workspace");
        let worker = FileOperation {
            root: root.path().to_owned(),
            operation: Operation::Write,
        };
        let requested = "x".repeat(300);
        let resolved = worker
            .resolve(Path::new(&requested))
            .expect("parent exists");
        assert!(Access::open(root.path(), &resolved, Operation::Write).is_err());
        assert_eq!(
            fs::read_dir(root.path())
                .expect("workspace entries")
                .count(),
            0
        );
    }

    #[test]
    fn a_symlink_to_non_utf8_names_cannot_use_a_lossy_policy_scope() {
        use std::os::unix::ffi::OsStringExt;
        let root = tempfile::tempdir().expect("workspace");
        let name = std::ffi::OsString::from_vec(vec![b'n', 0xff]);
        fs::write(root.path().join(&name), "private").expect("non-UTF-8 file");
        std::os::unix::fs::symlink(&name, root.path().join("alias")).expect("alias");
        for operation in [
            Operation::Read,
            Operation::Snapshot,
            Operation::Write,
            Operation::Edit,
        ] {
            assert!(execute(
                root.path(),
                operation,
                json!({"path":"alias","content":"bad","old_text":"private","new_text":"bad"})
            )
            .is_err_and(|e| e.to_string().contains("not UTF-8")));
        }
        assert_eq!(
            fs::read_to_string(root.path().join(name)).expect("unchanged"),
            "private"
        );
    }
}
