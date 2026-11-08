//! Private persistent workspaces. Original trees are visible only to the
//! trusted importer; shell and file workers receive the resulting copy.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One host-owned working copy, retained after the session for inspection.
pub struct Workspace {
    root: PathBuf,
    #[cfg(target_os = "linux")]
    sandbox: crate::sandbox::Sandbox,
}

impl Workspace {
    /// Copy regular files and literal symlinks in an offline namespace.
    /// Special files are rejected and hard links become independent files.
    /// The supplied worker executable is trusted host configuration.
    #[cfg(target_os = "linux")]
    pub fn import(source: &Path, worker: &Path) -> io::Result<Self> {
        use crate::process::{self, Control, Limits, Termination};
        use std::sync::{atomic::AtomicBool, Arc};
        use std::time::Duration;

        let source = source.canonicalize()?;
        if !source.is_dir() {
            return Err(io::Error::other("workspace source must be a directory"));
        }
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .ok_or_else(|| io::Error::other("no local workspace storage configured"))?;
        let storage = data.join("meden/workspaces");
        fs::create_dir_all(&storage)?;
        let storage = storage.canonicalize()?;
        if storage.starts_with(&source) {
            return Err(io::Error::other(
                "workspace source must not contain Meden's storage",
            ));
        }
        let temporary = tempfile::Builder::new()
            .prefix("session-")
            .tempdir_in(&storage)?;
        let sandbox = crate::sandbox::Sandbox::with_worker(temporary.path(), worker)?;
        let result = process::run(
            &mut sandbox.import_command(&source)?,
            &Limits {
                timeout: Duration::from_secs(120),
                capture_bytes: 24000,
                output_bytes: 1_000_000,
            },
            &Control::new(Arc::new(AtomicBool::new(false))),
        )?;
        if !matches!(result.termination, Termination::Exited(code) if code.success()) {
            return Err(io::Error::other(format!(
                "private workspace import {:?}: {}",
                result.termination, result.stderr
            )));
        }
        Ok(Self {
            root: temporary.keep(),
            sandbox,
        })
    }

    /// Unsupported platforms have no unrestricted fallback.
    #[cfg(not(target_os = "linux"))]
    pub fn import(_source: &Path, _worker: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "isolated workspaces require Linux",
        ))
    }

    /// Persistent result directory, outside the original source tree.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Build the trusted file worker command in the private namespace.
    pub fn file_command(&self) -> io::Result<Command> {
        #[cfg(target_os = "linux")]
        {
            self.sandbox.worker_command()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "isolation unavailable",
            ))
        }
    }

    /// Build an offline Bash worker. Its command text is supplied on stdin.
    pub fn shell_command(&self) -> io::Result<Command> {
        #[cfg(target_os = "linux")]
        {
            Ok(self
                .sandbox
                .command("bash", &["--noprofile", "--norc", "-s"]))
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "isolation unavailable",
            ))
        }
    }
}

/// Trusted worker operation. Run only in the prepared namespace, with fixed
/// `/source` and `/workspace` paths. No subprocess or user hook is executed.
pub fn import_tree(source: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            fs::create_dir(&target)?;
            import_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        } else if kind.is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(entry.path())?, target)?;
            #[cfg(not(unix))]
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "symlinks unsupported",
            ));
        } else {
            return Err(io::Error::other(
                "workspace contains a socket, device or FIFO",
            ));
        }
    }
    Ok(())
}
