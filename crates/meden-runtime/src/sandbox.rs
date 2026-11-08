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

/// A fixed host-selected toolchain and one writable workspace. Child commands
/// cannot choose additional mounts, inherit host configuration or share the
/// host network namespace. Toolchains are trusted public resources.
pub struct Sandbox {
    bwrap: PathBuf,
    root: PathBuf,
    tools: Vec<(String, PathBuf)>,
    store_paths: BTreeSet<PathBuf>,
}

fn find(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|path| path.join(program))
        .find(|path| path.is_file())
}

impl Sandbox {
    /// Prepare an offline execution configuration. Missing isolation support
    /// is an error; there is no unrestricted host fallback.
    pub fn new(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::other("workspace must be a directory"));
        }
        let bwrap = std::env::var_os("MEDEN_BWRAP")
            .map(PathBuf::from)
            .or_else(|| find("bwrap"))
            .ok_or_else(|| io::Error::other("bubblewrap is required for isolated execution"))?
            .canonicalize()?;
        let tools: Vec<_> = TOOLS
            .iter()
            .filter_map(|name| {
                find(name)
                    .and_then(|path| path.canonicalize().ok())
                    .map(|path| ((*name).to_owned(), path))
            })
            .collect();
        if !tools.iter().any(|(name, _)| name == "bash") {
            return Err(io::Error::other("Bash is required for isolated execution"));
        }
        let roots: BTreeSet<_> = tools
            .iter()
            .filter_map(|(_, path)| {
                path.ancestors()
                    .find(|p| p.parent() == Some(Path::new("/nix/store")))
                    .map(Path::to_owned)
            })
            .collect();
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
        })
    }

    /// Build a command in the private namespace with no inherited secrets,
    /// proxies, host sockets, host process tree or outbound network route.
    pub fn command(&self, executable: &str, args: &[impl AsRef<OsStr>]) -> Command {
        let mut command = Command::new(&self.bwrap);
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
            for path in ["/usr", "/bin", "/lib", "/lib64"] {
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
        command
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::process::{self, Control, Limits, Termination};
    use std::net::TcpListener;
    use std::sync::{atomic::AtomicBool, Arc};

    #[test]
    fn real_process_cannot_reach_host_files_or_network_and_can_edit_workspace() {
        let root = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("host directory");
        let secret = outside.path().join("secret");
        std::fs::write(&secret, "CONFIDENTIAL-CANARY").expect("host file");
        let listener = TcpListener::bind("127.0.0.1:0").expect("receiver");
        listener.set_nonblocking(true).expect("nonblocking");
        let port = listener.local_addr().expect("address").port();
        let script = format!(
            r#"
import os, socket, pathlib, subprocess
assert 'HOST_CREDENTIAL' not in os.environ
assert not pathlib.Path({host_path:?}).exists()
for host in ['127.0.0.1', '::1', '192.0.2.1']:
    try:
        s = socket.create_connection((host, {port}), timeout=.15)
    except OSError:
        pass
    else:
        s.sendall(b'CONFIDENTIAL-CANARY')
        raise AssertionError('network escaped')
assert subprocess.run(['python3', '-c', "import socket; socket.create_connection(('127.0.0.1',{port}),timeout=.1)"], capture_output=True).returncode != 0
pathlib.Path('edited.txt').write_text('edited offline')
print('isolated')
"#,
            host_path = secret.to_string_lossy()
        );
        let sandbox = Sandbox::new(root.path()).expect("isolation configuration");
        let mut command = sandbox.command("python3", &["-c", &script]);
        command.env("HOST_CREDENTIAL", "secret inherited by the launcher only");
        let output = process::run(
            &mut command,
            &Limits {
                timeout: std::time::Duration::from_secs(15),
                capture_bytes: 24000,
                output_bytes: 1_000_000,
            },
            &Control::new(Arc::new(AtomicBool::new(false))),
        )
        .expect("sandbox execution");
        assert!(
            matches!(output.termination, Termination::Exited(code) if code.success()),
            "{}",
            output.stderr
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("edited.txt")).expect("effect"),
            "edited offline"
        );
        assert!(
            listener.accept().is_err(),
            "receiver must observe zero connections"
        );
        assert_eq!(
            std::fs::read_to_string(secret).expect("host file"),
            "CONFIDENTIAL-CANARY"
        );
    }
}
