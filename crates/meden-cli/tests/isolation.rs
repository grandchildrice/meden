#![cfg(target_os = "linux")]
#![allow(clippy::expect_used)]
//! Observe the real CLI worker executable inside the production workspace.

use std::fs;
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

use meden_runtime::process::{self, Control, Limits, Output, Termination};
use meden_runtime::workspace::Workspace;
use serde_json::{json, Value};

struct Session(Workspace);
impl Drop for Session {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.0.path());
    }
}
fn import(source: &Path) -> Session {
    Session(Workspace::import(source, Path::new(env!("CARGO_BIN_EXE_meden"))).expect("import"))
}
fn run(command: &mut std::process::Command, input: &[u8]) -> Output {
    process::run_with_input(
        command,
        input,
        &Limits {
            timeout: Duration::from_secs(15),
            capture_bytes: 160_000,
            output_bytes: 1_000_000,
        },
        &Control::new(Arc::new(AtomicBool::new(false))),
    )
    .expect("isolated process")
}
fn success(output: &Output) -> bool {
    matches!(output.termination, Termination::Exited(code) if code.success())
}
fn file(workspace: &Workspace, name: &str, args: &Value) -> Output {
    let input = serde_json::to_vec(&json!({"operation": name, "arguments":args})).expect("request");
    run(&mut workspace.file_command().expect("file command"), &input)
}

#[test]
fn real_file_workers_share_private_edits_with_bash_and_break_host_hardlinks() {
    let source = tempfile::tempdir().expect("source");
    let outside = tempfile::tempdir().expect("host files");
    fs::write(source.path().join("note"), "draft\n").expect("source file");
    let host = outside.path().join("host-file");
    fs::write(&host, "HOST-CANARY").expect("host file");
    fs::hard_link(&host, source.path().join("shared")).expect("hard link");
    let session = import(source.path());
    let output = file(
        &session.0,
        "edit_file",
        &json!({"path":"note", "old_text":"draft", "new_text":"ready"}),
    );
    assert!(success(&output), "{}", output.stderr);
    let output = run(&mut session.0.shell_command().expect("Bash"), b"set -e; values=(one two); [[ ${values[1]} == two ]]; test \"$(cat note)\" = ready; printf private > shared; printf built > result");
    assert!(success(&output), "{}", output.stderr);
    let output = file(&session.0, "read_file", &json!({"path":"result"}));
    assert!(success(&output), "{}", output.stderr);
    assert_eq!(
        serde_json::from_str::<String>(&output.stdout).expect("result"),
        "1: built\n"
    );
    assert_eq!(
        fs::read_to_string(source.path().join("note")).expect("original"),
        "draft\n"
    );
    assert_eq!(
        fs::read_to_string(host).expect("host canary"),
        "HOST-CANARY"
    );
    assert!(!source.path().join("result").exists());
}

#[test]
fn host_paths_env_network_and_detached_children_do_not_escape_the_actual_shell() {
    let source = tempfile::tempdir().expect("source");
    let outside = tempfile::tempdir().expect("host files");
    let secret = outside.path().join("secret");
    fs::write(&secret, "CONFIDENTIAL-CANARY").expect("secret");
    std::os::unix::fs::symlink(outside.path(), source.path().join("outside")).expect("symlink");
    let receiver = TcpListener::bind("127.0.0.1:0").expect("receiver");
    receiver.set_nonblocking(true).expect("nonblocking");
    let port = receiver.local_addr().expect("address").port();
    let session = import(source.path());
    assert!(!success(&file(
        &session.0,
        "read_file",
        &json!({"path":"outside/secret"})
    )));
    // Deliberately make host capabilities inheritable. The launcher must close
    // them even when a host library forgot FD_CLOEXEC.
    let host_file = fs::File::open(&secret).expect("host capability");
    for fd in [host_file.as_raw_fd(), receiver.as_raw_fd()] {
        let borrowed = if fd == host_file.as_raw_fd() {
            std::os::fd::AsFd::as_fd(&host_file)
        } else {
            std::os::fd::AsFd::as_fd(&receiver)
        };
        nix::fcntl::fcntl(
            borrowed,
            nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::empty()),
        )
        .expect("inheritable capability");
    }
    let file_meta = host_file.metadata().expect("host metadata");
    let socket_meta =
        fs::metadata(format!("/proc/self/fd/{}", receiver.as_raw_fd())).expect("socket metadata");
    let script = format!(
        r#"python3 - <<'PY'
import os, pathlib, socket, subprocess, time
assert 'HOST_CREDENTIAL' not in os.environ
assert not pathlib.Path({host:?}).exists()
assert not pathlib.Path('/workspace/outside/secret').exists()
for fd, device, inode in [{file_cap}, {socket_cap}]:
    try:
        stat = os.fstat(fd)
    except OSError:
        continue
    assert (stat.st_dev, stat.st_ino) != (device, inode), 'inherited host capability'
for host in ['127.0.0.1', '::1', '192.0.2.1']:
    try:
        s = socket.create_connection((host, {port}), timeout=.1)
    except OSError:
        pass
    else:
        s.sendall(b'CONFIDENTIAL-CANARY')
        raise AssertionError('network escaped')
subprocess.Popen(['python3', '-c', "import time,pathlib; pathlib.Path('child-ready').write_text('ready'); time.sleep(.4); pathlib.Path('escaped').write_text('bad')"], start_new_session=True, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    if pathlib.Path('child-ready').exists():
        break
    time.sleep(.01)
assert pathlib.Path('child-ready').exists()
print('confined')
PY
"#,
        host = secret.to_string_lossy(),
        file_cap = format_args!(
            "({}, {}, {})",
            host_file.as_raw_fd(),
            file_meta.dev(),
            file_meta.ino()
        ),
        socket_cap = format_args!(
            "({}, {}, {})",
            receiver.as_raw_fd(),
            socket_meta.dev(),
            socket_meta.ino()
        )
    );
    let mut command = session.0.shell_command().expect("Bash");
    command.env("HOST_CREDENTIAL", "launcher-only-canary");
    let output = run(&mut command, script.as_bytes());
    assert!(success(&output), "{}", output.stderr);
    assert_eq!(output.stdout.trim(), "confined");
    std::thread::sleep(Duration::from_millis(600));
    assert!(!session.0.path().join("escaped").exists());
    assert!(receiver.accept().is_err());
}

#[test]
fn source_sockets_are_rejected_and_missing_isolation_never_falls_back() {
    let source = tempfile::tempdir().expect("source");
    let socket = source.path().join("host.sock");
    let _receiver = std::os::unix::net::UnixListener::bind(socket).expect("socket");
    assert!(Workspace::import(source.path(), Path::new(env!("CARGO_BIN_EXE_meden"))).is_err());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_meden"))
        .args(["tui", "--provider", "configured", "--workspace"])
        .arg(source.path())
        .env("MEDEN_LLM_BASE_URL", "http://127.0.0.1:1/v1")
        .env("MEDEN_LLM_MODEL", "fixture")
        .env("MEDEN_BWRAP", "/missing-meden-isolation-backend")
        .output()
        .expect("startup");
    assert!(!output.status.success());
}

#[test]
fn public_exec_retains_private_results_and_checks_them_in_another_offline_copy() {
    let directory = tempfile::tempdir().expect("fixture");
    let source = directory.path().join("source");
    fs::create_dir(&source).expect("source");
    fs::write(source.join("calc.py"), "def add(a,b): return a-b\n").expect("source file");
    let execute = |root: &Path, input: &[u8]| {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_meden"));
        command
            .args(["exec", "--workspace"])
            .arg(root)
            .env("XDG_DATA_HOME", directory.path().join("data"));
        let output = run(&mut command, input);
        assert!(success(&output), "{}", output.stderr);
        serde_json::from_str::<Value>(&output.stdout).expect("result record")
    };
    let first = execute(&source, b"printf 'def add(a,b): return a+b\\n' > calc.py; python3 -c 'from calc import add; assert add(2,3)==5'");
    let first_path = Path::new(
        first
            .get("workspace")
            .and_then(Value::as_str)
            .expect("private path"),
    );
    assert_eq!(
        fs::read_to_string(source.join("calc.py")).expect("original"),
        "def add(a,b): return a-b\n"
    );
    assert!(fs::read_to_string(first_path.join("calc.py"))
        .expect("private result")
        .contains("a+b"));
    let checked = execute(
        first_path,
        b"python3 -c 'from calc import add; assert add(-4,3)==-1'",
    );
    assert_eq!(
        checked.get("result").and_then(|r| r.get("exit_code")),
        Some(&json!(0))
    );
    assert_ne!(checked.get("workspace"), first.get("workspace"));
}
