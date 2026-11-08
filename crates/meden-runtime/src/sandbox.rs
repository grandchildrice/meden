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

fn store_root(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|p| p.parent() == Some(Path::new("/nix/store")))
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
        Self::configure(root, worker.canonicalize()?, &[])
    }

    /// A separate offline model process. Only prepared executable/weights and
    /// a private scratch/socket directory are mounted; never the coding copy.
    pub fn inference_command(
        root: &Path,
        worker: &Path,
        engine: &Path,
        model: &Path,
    ) -> io::Result<Command> {
        let engine = engine.canonicalize()?;
        let model = model.canonicalize()?;
        if !engine.is_file() || !model.is_file() {
            return Err(io::Error::other(
                "local engine and weights must be regular files",
            ));
        }
        let sandbox = Self::configure(root, worker.canonicalize()?, &[&engine])?;
        Ok(sandbox.build(
            "/local-engine",
            &[
                "--model",
                "/model.gguf",
                "--host",
                "/workspace/llama.sock",
                "--offline",
                "--no-webui",
                "--no-ui-mcp-proxy",
                "--jinja",
                "--n-gpu-layers",
                "0",
                "--alias",
                "meden-local",
                "--ctx-size",
                "16384",
                "--reasoning-budget",
                "0",
            ],
            None,
            &[(&engine, "/local-engine"), (&model, "/model.gguf")],
        ))
    }

    fn configure(root: &Path, worker: PathBuf, assets: &[&Path]) -> io::Result<Self> {
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
            .filter_map(|(_, path)| store_root(path).map(Path::to_owned))
            .collect();
        for asset in assets {
            if let Some(root) = store_root(asset) {
                roots.insert(root.to_owned());
            }
        }
        let mut worker_libraries = Vec::new();
        let mut worker_loader = None;
        if !roots.is_empty() {
            // The trusted worker may use another libc than the toolchain.
            // Resolve its public library objects, never expose nix-ld's
            // whole system profile or the whole Nix store.
            for binary in std::iter::once(worker.as_path()).chain(assets.iter().copied()) {
                let output = Command::new(
                    find("ldd").ok_or_else(|| io::Error::other("ldd required for Nix worker"))?,
                )
                .arg(binary)
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
                    if let Some(root) = store_root(&path) {
                        roots.insert(root.to_owned());
                        if let Some(parent) = path.parent().filter(|_| binary == worker) {
                            worker_libraries.push(parent.to_owned());
                        }
                        if binary == worker && word.ends_with("/ld-linux-x86-64.so.2") {
                            worker_loader = Some(path);
                        }
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
        self.build(executable, args, None, &[])
    }

    pub(crate) fn for_verification(
        root: &Path,
        worker: &Path,
        bundle: &crate::verification::Bundle,
    ) -> io::Result<Self> {
        let executables = bundle.executables();
        Self::configure(
            root,
            worker.canonicalize()?,
            &executables.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
        )
    }

    pub(crate) fn verification_command(&self, bundle: &crate::verification::Bundle) -> Command {
        let bindings = bundle.bindings();
        self.build(
            "/bin/bash",
            &["--noprofile", "--norc"],
            None,
            &bindings
                .iter()
                .map(|(p, n)| (p.as_path(), n.as_str()))
                .collect::<Vec<_>>(),
        )
    }

    pub(crate) fn verification_syntax_command(&self) -> io::Result<Command> {
        self.worker_invocation("worker-ledger-syntax", None)
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
        Ok(self.build("/bin/env", &args, source, &[]))
    }

    fn build(
        &self,
        executable: &str,
        args: &[impl AsRef<OsStr>],
        source: Option<&Path>,
        bindings: &[(&Path, &str)],
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
        // Debian compiler alternatives can point into /etc, which stays hidden.
        // Expose the already selected public objects through private aliases.
        command.args(["--dir", "/tools"]);
        for (name, path) in &self.tools {
            command
                .arg("--symlink")
                .arg(path)
                .arg(format!("/tools/{name}"));
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
        for (source, destination) in bindings {
            command.arg("--ro-bind").arg(source).arg(destination);
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
                "/tools:/bin:/usr/bin",
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
