//! Linux worker containment. Only host documentation fetches retain networking.
//! Bubblewrap and OS semantics are trusted; no Lean isolation proof is claimed.

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
const RUST_TOOLS: &[&str] = &["cargo", "rustc", "rustdoc"];

#[derive(Clone, Copy)]
enum Network {
    Offline,
    Pages,
}

/// Host-selected public toolchains and a private writable root. Coding,
/// checking and inference stay offline; only the dedicated documentation
/// command retains networking. Mounts and environment are fixed by the host.
pub struct Sandbox {
    bwrap: PathBuf,
    root: PathBuf,
    tools: Vec<(String, PathBuf)>,
    system_tools: bool,
    store_paths: BTreeSet<PathBuf>,
    tool_bindings: BTreeSet<PathBuf>,
    worker: PathBuf,
    worker_libraries: Vec<PathBuf>,
    worker_loader: Option<PathBuf>,
}

fn find(program: &str, approved: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter_map(|path| path.join(program).canonicalize().ok())
        .find(|path| path.is_file() && approved(path))
}

fn store_root(path: &Path) -> Option<&Path> {
    path.ancestors()
        .find(|p| p.parent() == Some(Path::new("/nix/store")))
}

fn public_path(path: &Path) -> bool {
    store_root(path).is_some() || SYSTEM_ROOTS.iter().any(|root| path.starts_with(root))
}

fn find_public(program: &str) -> Option<PathBuf> {
    find(program, public_path)
}

// Preparation runs before the namespace exists. Never execute a repository
// PATH entry, including through a helper's interpreter or child lookup.
fn host_command(program: &Path, name: &str) -> io::Result<Command> {
    use std::os::unix::process::CommandExt as _;
    let paths: Vec<_> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .filter_map(|path| path.canonicalize().ok())
        .filter(|path| path.is_dir() && public_path(path))
        .collect();
    let mut command = Command::new(program);
    command
        .arg0(name)
        .current_dir("/")
        .env(
            "PATH",
            std::env::join_paths(paths).map_err(io::Error::other)?,
        )
        .env_remove("BASH_ENV")
        .env_remove("ENV");
    Ok(command)
}

fn public_tools() -> (Vec<(String, PathBuf)>, Option<PathBuf>) {
    let mut tools: Vec<_> = TOOLS
        .iter()
        .filter_map(|name| {
            // A Python virtualenv/uv executable can shadow the public
            // Nix toolchain without being present in its mounted closure.
            // Pick a visible public object; never expose a host cache.
            find_public(name).map(|path| ((*name).to_owned(), path))
        })
        .collect();
    let mut manager = None;
    if let Some(rustup) = rustup_proxy() {
        tools.retain(|(name, _)| !RUST_TOOLS.contains(&name.as_str()));
        if let Some(prepared) = prepared_rust(&rustup) {
            tools.extend(prepared);
            manager = Some(rustup);
        }
    }
    (tools, manager)
}

// Do not execute a repository-supplied `cargo`/`rustup` from PATH. A proxy
// must be the same installed object as an approved public/host Cargo binary.
fn rustup_proxy() -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt as _;
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    let installed = cargo_home
        .as_ref()
        .and_then(|home| home.join("bin/rustup").canonicalize().ok());
    let rustup = find("rustup", |path| {
        public_path(path) || installed.as_deref() == Some(path)
    })?;
    let installed_cargo = cargo_home.and_then(|home| home.join("bin/cargo").canonicalize().ok());
    let proxy = find("cargo", |path| {
        public_path(path) || installed_cargo.as_deref() == Some(path)
    })?
    .metadata()
    .ok()?;
    let manager = rustup.metadata().ok()?;
    (proxy.dev() == manager.dev() && proxy.ino() == manager.ino()).then_some(rustup)
}

// Snapshot already-installed executables before confinement. No workspace
// toolchain override or rustup settings/cache is exposed to the child.
fn prepared_rust(rustup: &Path) -> Option<Vec<(String, PathBuf)>> {
    let toolchain = std::env::var_os("RUSTUP_TOOLCHAIN").or_else(|| {
        let output = host_command(rustup, "rustup")
            .ok()?
            .arg("default")
            .env("RUSTUP_AUTO_INSTALL", "0")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        Some(
            String::from_utf8(output.stdout)
                .ok()?
                .split_whitespace()
                .next()?
                .into(),
        )
    })?;
    RUST_TOOLS
        .iter()
        .map(|name| {
            let path = if Path::new(&toolchain).is_absolute() {
                Path::new(&toolchain).join("bin").join(name)
            } else {
                let output = host_command(rustup, "rustup")
                    .ok()?
                    .args(["which", "--toolchain"])
                    .arg(&toolchain)
                    .arg(name)
                    .env("RUSTUP_AUTO_INSTALL", "0")
                    .output()
                    .ok()?;
                if !output.status.success() {
                    return None;
                }
                PathBuf::from(String::from_utf8(output.stdout).ok()?.trim_end())
            }
            .canonicalize()
            .ok()?;
            path.is_file().then(|| ((*name).to_owned(), path))
        })
        .collect()
}

fn rust_bindings(tools: &[(String, PathBuf)]) -> BTreeSet<PathBuf> {
    let mut bindings = BTreeSet::new();
    for (name, path) in tools {
        if !RUST_TOOLS.contains(&name.as_str())
            || SYSTEM_ROOTS.iter().any(|root| path.starts_with(root))
        {
            continue;
        }
        bindings.insert(path.clone());
        if let Some(lib) = path
            .parent()
            .and_then(Path::parent)
            .map(|root| root.join("lib"))
            .filter(|path| path.is_dir())
            .and_then(|path| path.canonicalize().ok())
        {
            bindings.insert(lib);
        }
    }
    bindings
}

fn resolve_libraries<'a>(
    worker: &'a Path,
    assets: impl Iterator<Item = &'a Path>,
    roots: &mut BTreeSet<PathBuf>,
) -> io::Result<(Vec<PathBuf>, Option<PathBuf>)> {
    let mut worker_libraries = Vec::new();
    let mut worker_loader = None;
    if !roots.is_empty() {
        // The trusted worker may use another libc than the toolchain.
        // Resolve its public library objects, never expose nix-ld's
        // whole system profile or the whole Nix store.
        for binary in std::iter::once(worker).chain(assets) {
            let output = host_command(
                &find_public("ldd")
                    .ok_or_else(|| io::Error::other("public ldd required for Nix worker"))?,
                "ldd",
            )?
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
    Ok((worker_libraries, worker_loader))
}

// llama.cpp discovers siblings from the first split's filename. Keep that
// naming scheme in the private namespace without exposing the host directory.
// Parse before canonicalization: prepared cache entries may be symlinks to
// blobs whose names carry no split information.
fn model_bindings(model: &Path) -> io::Result<Vec<(PathBuf, String)>> {
    let split = model
        .file_name()
        .and_then(OsStr::to_str)
        .and_then(|name| name.strip_suffix(".gguf"))
        .and_then(|name| name.rsplit_once("-of-"))
        .and_then(|(name, count)| {
            name.rsplit_once('-')
                .map(|(prefix, index)| (prefix, index, count))
        })
        .filter(|(_, index, count)| {
            index.len() == 5
                && count.len() == 5
                && index.bytes().all(|c| c.is_ascii_digit())
                && count.bytes().all(|c| c.is_ascii_digit())
        });
    let paths = if let Some((prefix, index, count)) = split {
        // GGUF's split.count is a uint16. Refuse a later shard or impossible
        // count before starting any model process.
        let count = count
            .parse::<u16>()
            .ok()
            .filter(|count| *count > 0 && index == "00001")
            .ok_or_else(|| {
                io::Error::other("split weights require the first shard and a valid count")
            })?;
        (1..=count)
            .map(|index| {
                (
                    model.with_file_name(format!("{prefix}-{index:05}-of-{count:05}.gguf")),
                    format!("/model-{index:05}-of-{count:05}.gguf"),
                )
            })
            .collect::<Vec<_>>()
    } else {
        vec![(model.to_owned(), "/model.gguf".to_owned())]
    };
    paths
        .into_iter()
        .map(|(path, destination)| {
            let path = path.canonicalize().map_err(|error| {
                io::Error::other(format!("cannot prepare local weights: {error}"))
            })?;
            if !path.is_file() {
                return Err(io::Error::other("local weights must be regular files"));
            }
            Ok((path, destination))
        })
        .collect()
}

impl Sandbox {
    /// Prepare an offline execution configuration. Missing isolation support
    /// is an error; there is no unrestricted host fallback.
    pub fn with_worker(root: &Path, worker: &Path) -> io::Result<Self> {
        Self::configure(root, worker.canonicalize()?, &[], BTreeSet::new())
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
        if !engine.is_file() {
            return Err(io::Error::other("local engine must be a regular file"));
        }
        let models = model_bindings(model)?;
        let model = models
            .first()
            .ok_or_else(|| io::Error::other("local weights missing"))?;
        let bindings: Vec<_> = std::iter::once((engine.as_path(), "/local-engine"))
            .chain(
                models
                    .iter()
                    .map(|(path, name)| (path.as_path(), name.as_str())),
            )
            .collect();
        let sandbox = Self::configure(root, worker.canonicalize()?, &[&engine], BTreeSet::new())?;
        Ok(sandbox.build(
            "/local-engine",
            &[
                "--model",
                &model.1,
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
            &bindings,
            Network::Offline,
        ))
    }

    /// The caller supplies a private scratch directory; the dedicated worker
    /// receives only its URL batch on stdin. Networking is retained for HTTPS,
    /// with every resolved connection address checked by the worker.
    pub fn prefetch_command(root: &Path, worker: &Path) -> io::Result<Command> {
        Self::configure(root, worker.canonicalize()?, &[], BTreeSet::new())?.worker_invocation(
            "worker-prefetch",
            None,
            Network::Pages,
        )
    }

    fn configure(
        root: &Path,
        worker: PathBuf,
        assets: &[&Path],
        mut tool_bindings: BTreeSet<PathBuf>,
    ) -> io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::other("workspace must be a directory"));
        }
        let bwrap = std::env::var_os("MEDEN_BWRAP")
            .map(PathBuf::from)
            .or_else(|| find_public("bwrap"))
            .ok_or_else(|| {
                io::Error::other(
                    "bubblewrap is required for isolated execution: no public bwrap found in PATH; set MEDEN_BWRAP to the absolute path of a trusted Bubblewrap executable",
                )
            })?
            .canonicalize()?;
        let (tools, manager) = public_tools();
        let system_tools = tools
            .iter()
            .any(|(_, path)| SYSTEM_ROOTS.iter().any(|root| path.starts_with(root)));
        tool_bindings.extend(rust_bindings(&tools));
        if let Some(manager) = manager.filter(|path| store_root(path).is_some()) {
            tool_bindings.insert(manager);
        }
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
        for path in &tool_bindings {
            if let Some(root) = store_root(path) {
                roots.insert(root.to_owned());
            }
        }
        tool_bindings.retain(|path| store_root(path).is_none());
        let (worker_libraries, worker_loader) = resolve_libraries(
            &worker,
            assets.iter().copied().chain(
                tools
                    .iter()
                    .filter(|(_, path)| store_root(path).is_none())
                    .map(|(_, path)| path.as_path()),
            ),
            &mut roots,
        )?;
        let mut store_paths = BTreeSet::new();
        if !roots.is_empty() {
            // Bind only the toolchain closure, never the whole Nix store or
            // the system profile (which can include host configuration).
            let output = host_command(
                &find_public("nix-store")
                    .ok_or_else(|| io::Error::other("nix-store required for Nix toolchains"))?,
                "nix-store",
            )?
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
            system_tools,
            store_paths,
            tool_bindings,
            worker,
            worker_libraries,
            worker_loader,
        })
    }

    /// Build a command in the private namespace with no inherited secrets,
    /// proxies, host sockets, host process tree or outbound network route.
    pub fn command(&self, executable: &str, args: &[impl AsRef<OsStr>]) -> Command {
        self.build(executable, args, None, &[], Network::Offline)
    }

    pub(crate) fn for_verification(
        root: &Path,
        worker: &Path,
        bundle: &crate::verification::Bundle,
    ) -> io::Result<Self> {
        let executables = bundle.executables();
        // Prepared Rust wrappers can refer to their own sysroot by its original
        // absolute path. Retain only the selected binaries/lib there, just as
        // for the independently selected host toolchain; never its parent.
        // Keep a symlinked lib's original alias as well as its canonical target,
        // which supplies the Nix closure and any absolute dependency references.
        let rust = ["rustc", "cargo"]
            .map(|name| bundle.rust.join("bin").join(name))
            .into_iter()
            .chain([
                bundle.rust.join("lib"),
                bundle.rust.join("lib").canonicalize()?,
            ])
            .collect();
        Self::configure(
            root,
            worker.canonicalize()?,
            &executables.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
            rust,
        )
    }

    pub(crate) fn verification_command(&self, bundle: &crate::verification::Bundle) -> Command {
        let bindings = bundle.bindings();
        self.build(
            "/tools/bash",
            &["--noprofile", "--norc"],
            None,
            &bindings
                .iter()
                .map(|(p, n)| (p.as_path(), n.as_str()))
                .collect::<Vec<_>>(),
            Network::Offline,
        )
    }

    pub(crate) fn verification_source_command(&self) -> io::Result<Command> {
        self.worker_invocation("worker-kernel-syntax", None, Network::Offline)
    }

    /// Invoke the trusted file worker inside the same containment as shell tools.
    pub fn worker_command(&self) -> io::Result<Command> {
        self.worker_invocation("worker-file", None, Network::Offline)
    }

    /// Only the trusted importer receives a read-only view of the original.
    pub fn import_command(&self, source: &Path) -> io::Result<Command> {
        self.worker_invocation("worker-import", Some(source), Network::Offline)
    }

    fn worker_invocation(
        &self,
        operation: &str,
        source: Option<&Path>,
        network: Network,
    ) -> io::Result<Command> {
        let mut args = Vec::new();
        if !self.worker_libraries.is_empty() {
            let paths = std::env::join_paths(&self.worker_libraries).map_err(io::Error::other)?;
            let mut assignment = std::ffi::OsString::from("LD_LIBRARY_PATH=");
            assignment.push(paths);
            args.push(assignment);
        }
        if let Some(loader) = &self.worker_loader {
            args.push(loader.as_os_str().to_owned());
        }
        args.push("/meden-worker".into());
        args.push(operation.into());
        let resolver = if matches!(network, Network::Pages) {
            Some(Path::new("/etc/resolv.conf").canonicalize()?)
        } else {
            None
        };
        let bindings: Vec<_> = resolver
            .as_deref()
            .map(|path| (path, "/etc/resolv.conf"))
            .into_iter()
            .collect();
        Ok(self.build("/tools/env", &args, source, &bindings, network))
    }

    fn build(
        &self,
        executable: &str,
        args: &[impl AsRef<OsStr>],
        source: Option<&Path>,
        bindings: &[(&Path, &str)],
        network: Network,
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
        if matches!(network, Network::Pages) {
            command.args(["--share-net", "--dir", "/etc"]);
        }
        self.mount_toolchains(&mut command);
        command
            .arg("--ro-bind")
            .arg(&self.worker)
            .arg("/meden-worker");
        if let Some(loader) = self.worker_loader.as_ref().filter(|_| !self.system_tools) {
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

    fn mount_toolchains(&self, command: &mut Command) {
        if self.system_tools {
            for path in SYSTEM_ROOTS {
                if Path::new(path).exists() {
                    command.args(["--ro-bind", path, path]);
                }
            }
        }
        if !self.store_paths.is_empty() {
            command.args(["--dir", "/nix", "--dir", "/nix/store"]);
            for path in &self.store_paths {
                command.arg("--ro-bind").arg(path).arg(path);
            }
        }
        if !self.system_tools {
            command.args(["--dir", "/bin"]);
            for (name, path) in &self.tools {
                command
                    .arg("--symlink")
                    .arg(path)
                    .arg(format!("/bin/{name}"));
            }
            command.args([
                "--symlink",
                "/bin/bash",
                "/bin/sh",
                "--dir",
                "/usr",
                "--dir",
                "/usr/bin",
                "--symlink",
                "/tools/env",
                "/usr/bin/env",
            ]);
        }
        for path in &self.tool_bindings {
            command.arg("--ro-bind").arg(path).arg(path);
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
    }
}
