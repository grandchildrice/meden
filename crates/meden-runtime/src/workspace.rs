//! Repository workspaces mounted into offline workers. Explicit snapshot jobs
//! may still import a private copy; ordinary development opens the given tree.

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Host-owned per-repository storage, outside the model's writable tree.
/// `root` is the canonical repository path selected by the host.
pub fn state_directory(root: &Path) -> io::Result<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .filter(|path| path.is_absolute())
        .ok_or_else(|| io::Error::other("no absolute local storage directory configured"))?;
    let key = format!("{:x}", Sha256::digest(root.as_os_str().as_encoded_bytes()));
    Ok(data.join("meden/projects").join(key))
}

/// The selected repository, exposed only at `/workspace` inside each worker.
pub struct Workspace {
    root: PathBuf,
    session: Vec<u8>,
    #[cfg(target_os = "linux")]
    _lease: Option<nix::fcntl::Flock<fs::File>>,
    #[cfg(target_os = "linux")]
    _session_directory: Option<tempfile::TempDir>,
    #[cfg(target_os = "linux")]
    sandbox: crate::sandbox::Sandbox,
}

impl Workspace {
    /// Develop directly in the host-selected repository while retaining the
    /// offline sandbox. Metadata and permissions remain outside that mount.
    #[cfg(target_os = "linux")]
    pub fn open(root: &Path, worker: &Path) -> io::Result<Self> {
        use std::os::unix::fs::PermissionsExt as _;
        fs::create_dir_all(root)?;
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::other("workspace must be a directory"));
        }
        let storage = state_directory(&root)?;
        fs::create_dir_all(&storage)?;
        let storage = storage.canonicalize()?;
        if storage.starts_with(&root) || root.starts_with(&storage) {
            return Err(io::Error::other(
                "the repository and Meden's host state must be separate directories",
            ));
        }
        fs::set_permissions(&storage, fs::Permissions::from_mode(0o700))?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(storage.join("workspace.lock"))?;
        let lease = nix::fcntl::Flock::lock(lock, nix::fcntl::FlockArg::LockExclusiveNonblock)
            .map_err(|(_, error)| {
                io::Error::other(format!(
                    "This repository is already open in another Meden session: {error}"
                ))
            })?;
        let directory = tempfile::Builder::new()
            .prefix("run-")
            .tempdir_in(&storage)?;
        let session = directory.path().as_os_str().as_encoded_bytes().to_vec();
        let sandbox = crate::sandbox::Sandbox::with_worker(&root, worker)?;
        Ok(Self {
            root,
            session,
            sandbox,
            _lease: Some(lease),
            _session_directory: Some(directory),
        })
    }

    /// Unsupported platforms never fall back to unrestricted host execution.
    #[cfg(not(target_os = "linux"))]
    pub fn open(_root: &Path, _worker: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "isolated workspaces require Linux",
        ))
    }

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
        let root = temporary.keep();
        Ok(Self {
            session: root.as_os_str().as_encoded_bytes().to_vec(),
            root,
            sandbox,
            _lease: None,
            _session_directory: None,
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

    /// Actual host directory where file and Bash changes are written.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Fresh process/session binding. Reopening a repository never renews an
    /// earlier processor grant, check job or export approval.
    pub fn session_id(&self) -> &[u8] {
        &self.session
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
