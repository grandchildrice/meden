//! Offline Linux execution. Bubblewrap and the kernel are trusted components;
//! this implementation does not claim a Lean proof of their isolation.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const TOOLS: &[&str] = &[
    "bash", "python3", "git", "cargo", "rustc", "rustdoc", "node", "npm", "npx", "rg", "cat", "ls",
    "mkdir", "cp", "mv", "rm", "touch", "find", "sed", "awk", "grep", "head", "tail", "wc", "true",
    "false", "env", "sleep", "cc", "c++", "make", "uname", "tar", "gzip", "base64",
];
const SYSTEM_ROOTS: &[&str] = &["/usr", "/bin", "/lib", "/lib64"];

/// A fixed host-selected toolchain and one writable workspace. Child commands
/// cannot choose additional mounts, inherit host configuration or share the
/// host network namespace. Toolchains are trusted public resources.
pub struct Sandbox {
    bwrap: PathBuf,
    root: PathBuf,
    tools: Vec<(String, PathBuf)>,
    store_paths: BTreeSet<PathBuf>,
    worker: PathBuf,
    worker_libraries: Vec<PathBuf>,
    worker_loader: Option<PathBuf>,
}

fn find(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|path| path.join(program))
        .find(|path| path.is_file())
}

fn public_tools(nix_tools: bool) -> Vec<(String, PathBuf)> {
    let search = std::env::var_os("PATH").unwrap_or_default();
    TOOLS
        .iter()
        .filter_map(|name| {
            // A Python virtualenv/uv executable can shadow the public
            // Nix toolchain without being present in its mounted closure.
            // Pick a visible public object; never expose a host cache.
            std::env::split_paths(&search)
                .filter_map(|path| path.join(name).canonicalize().ok())
                .find(|path| {
                    path.is_file()
                        && if nix_tools {
                            path.starts_with("/nix/store")
                        } else {
                            SYSTEM_ROOTS.iter().any(|root| path.starts_with(root))
                        }
                })
                .map(|path| ((*name).to_owned(), path))
        })
        .collect()
}

impl Sandbox {
    /// Prepare an offline execution configuration. Missing isolation support
    /// is an error; there is no unrestricted host fallback.
    pub fn with_worker(root: &Path, worker: &Path) -> io::Result<Self> {
        Self::configure(root, worker.canonicalize()?)
    }

    fn configure(root: &Path, worker: PathBuf) -> io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::other("workspace must be a directory"));
        }
        let bwrap = std::env::var_os("MEDEN_BWRAP")
            .map(PathBuf::from)
            .or_else(|| find("bwrap"))
            .ok_or_else(|| io::Error::other("bubblewrap is required for isolated execution"))?
            .canonicalize()?;
        let nix_tools = find("bash")
            .and_then(|path| path.canonicalize().ok())
            .is_some_and(|path| path.starts_with("/nix/store"));
        let tools = public_tools(nix_tools);
        if !tools.iter().any(|(name, _)| name == "bash") {
            return Err(io::Error::other("Bash is required for isolated execution"));
        }
        let mut roots: BTreeSet<_> = tools
            .iter()
            .filter_map(|(_, path)| {
                path.ancestors()
                    .find(|p| p.parent() == Some(Path::new("/nix/store")))
                    .map(Path::to_owned)
            })
            .collect();
        let mut worker_libraries = Vec::new();
        let mut worker_loader = None;
        if !roots.is_empty() {
            // The trusted worker may use another libc than the toolchain.
            // Resolve its public library objects, never expose nix-ld's
            // whole system profile or the whole Nix store.
            let output = Command::new(
                find("ldd").ok_or_else(|| io::Error::other("ldd required for Nix worker"))?,
            )
            .arg(&worker)
            .output()?;
            if !output.status.success() {
                let message = String::from_utf8_lossy(&output.stderr);
                if !message.contains("not a dynamic executable")
                    && !message.contains("statically linked")
                {
                    return Err(io::Error::other("cannot resolve worker libraries"));
                }
            }
            for word in String::from_utf8_lossy(&output.stdout).split_whitespace() {
                if !word.starts_with('/') {
                    continue;
                }
                let path = Path::new(word).canonicalize()?;
                if let Some(root) = path
                    .ancestors()
                    .find(|p| p.parent() == Some(Path::new("/nix/store")))
                {
                    roots.insert(root.to_owned());
                    if let Some(parent) = path.parent() {
                        worker_libraries.push(parent.to_owned());
                    }
                    if word.ends_with("/ld-linux-x86-64.so.2") {
                        worker_loader = Some(path);
                    }
                }
            }
            worker_libraries.sort();
            worker_libraries.dedup();
        }
        let mut store_paths = BTreeSet::new();
        if !roots.is_empty() {
            // Bind only the toolchain closure, never the whole Nix store or
            // the system profile (which can include host configuration).
            let output = Command::new(
                find("nix-store")
                    .ok_or_else(|| io::Error::other("nix-store required for Nix toolchains"))?,
            )
            .args(["--query", "--requisites"])
            .args(&roots)
            .output()?;
            if !output.status.success() {
                return Err(io::Error::other("cannot resolve public toolchain closure"));
            }
            for path in String::from_utf8(output.stdout)
                .map_err(io::Error::other)?
                .lines()
            {
                let path = PathBuf::from(path);
                if path.parent() != Some(Path::new("/nix/store")) {
                    return Err(io::Error::other("unexpected toolchain closure path"));
                }
                store_paths.insert(path);
            }
        }
        Ok(Self {
            bwrap,
            root,
            tools,
            store_paths,
            worker,
            worker_libraries,
            worker_loader,
        })
    }

    /// Build a command in the private namespace with no inherited secrets,
    /// proxies, host sockets, host process tree or outbound network route.
    pub fn command(&self, executable: &str, args: &[impl AsRef<OsStr>]) -> Command {
        self.build(executable, args, None)
    }

    /// Invoke the trusted file worker inside the same containment as shell tools.
    pub fn worker_command(&self) -> io::Result<Command> {
        self.worker_invocation("worker-file", None)
    }

    /// Only the trusted importer receives a read-only view of the original.
    pub fn import_command(&self, source: &Path) -> io::Result<Command> {
        self.worker_invocation("worker-import", Some(source))
    }

    fn worker_invocation(&self, operation: &str, source: Option<&Path>) -> io::Result<Command> {
        let mut args = Vec::new();
        if !self.worker_libraries.is_empty() {
            let paths = std::env::join_paths(&self.worker_libraries).map_err(io::Error::other)?;
            let mut assignment = std::ffi::OsString::from("LD_LIBRARY_PATH=");
            assignment.push(paths);
            args.push(assignment);
        }
        args.push("/meden-worker".into());
        args.push(operation.into());
        Ok(self.build("/bin/env", &args, source))
    }

    fn build(
        &self,
        executable: &str,
        args: &[impl AsRef<OsStr>],
        source: Option<&Path>,
    ) -> Command {
        // The dedicated Rust launcher closes all inherited capabilities
        // before execing bwrap. Bubblewrap itself passes inherited fds on.
        let mut command = Command::new(&self.worker);
        command.arg("worker-launch").arg("--").arg(&self.bwrap);
        command.args([
            "--unshare-all",
            "--unshare-user",
            "--disable-userns",
            "--new-session",
            "--die-with-parent",
            "--cap-drop",
            "ALL",
            "--clearenv",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            "/home",
            "--dir",
            "/run",
            "--dir",
            "/workspace",
        ]);
        if self.store_paths.is_empty() {
            for path in SYSTEM_ROOTS {
                if Path::new(path).exists() {
                    command.args(["--ro-bind", path, path]);
                }
            }
        } else {
            command.args(["--dir", "/nix", "--dir", "/nix/store", "--dir", "/bin"]);
            for path in &self.store_paths {
                command.arg("--ro-bind").arg(path).arg(path);
            }
            for (name, path) in &self.tools {
                command
                    .arg("--symlink")
                    .arg(path)
                    .arg(format!("/bin/{name}"));
            }
            command.args(["--symlink", "/bin/bash", "/bin/sh"]);
        }
        command
            .arg("--ro-bind")
            .arg(&self.worker)
            .arg("/meden-worker");
        if let Some(loader) = &self.worker_loader {
            command
                .args(["--dir", "/lib64", "--symlink"])
                .arg(loader)
                .arg("/lib64/ld-linux-x86-64.so.2");
        }
        if let Some(source) = source {
            command.arg("--ro-bind").arg(source).arg("/source");
        }
        command
            .arg("--bind")
            .arg(&self.root)
            .arg("/workspace")
            .args([
                "--chdir",
                "/workspace",
                "--setenv",
                "PATH",
                "/bin:/usr/bin",
                "--setenv",
                "HOME",
                "/tmp",
                "--setenv",
                "LANG",
                "C.UTF-8",
                "--setenv",
                "TMPDIR",
                "/tmp",
                "--remount-ro",
                "/",
                "--",
            ])
            .arg(executable)
            .args(args)
            .env_clear();
        if !self.worker_libraries.is_empty() {
            // Only the host launcher sees this search path. exec_clean clears
            // it before bwrap and per-worker paths are set inside separately.
            if let Ok(paths) = std::env::join_paths(&self.worker_libraries) {
                command.env("LD_LIBRARY_PATH", paths);
            }
        }
        command
    }
}
