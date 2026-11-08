//! Managed non-interactive processes. File-backed output avoids pipe
//! deadlocks; cancellation and timeout also terminate the Unix process group.
//! This manages process lifetime, not command permissions or OS isolation.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Enter an executable from a fresh single-threaded Linux launcher, closing
/// every inherited capability except stdin/stdout/stderr. Never call this in
/// the session process: closing there would race its other threads.
#[cfg(target_os = "linux")]
pub fn exec_clean(arguments: &[std::ffi::OsString]) -> io::Result<()> {
    use std::os::unix::process::CommandExt as _;
    if std::fs::read_dir("/proc/self/task")?.count() != 1 {
        return Err(io::Error::other("launcher must be single-threaded"));
    }
    let descriptors = std::fs::read_dir("/proc/self/fd")?
        .map(|entry| {
            entry?
                .file_name()
                .to_string_lossy()
                .parse::<i32>()
                .map_err(io::Error::other)
        })
        .collect::<io::Result<Vec<_>>>()?;
    for fd in descriptors.into_iter().filter(|fd| *fd > 2) {
        match nix::unistd::close(fd) {
            Ok(()) | Err(nix::errno::Errno::EBADF) => {}
            Err(error) => return Err(io::Error::from(error)),
        }
    }
    let (program, args) = arguments
        .split_first()
        .ok_or_else(|| io::Error::other("missing executable"))?;
    Err(Command::new(program).args(args).env_clear().exec())
}

/// Unsupported platforms have no unconfined launcher fallback.
#[cfg(not(target_os = "linux"))]
pub fn exec_clean(_arguments: &[std::ffi::OsString]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "isolated launch requires Linux",
    ))
}

/// Shared cancellation and ownership of spawned process groups.
pub struct Control {
    cancelled: Arc<AtomicBool>,
    active: Mutex<BTreeSet<u32>>,
}

impl Control {
    /// Cancellation flag shared with the conversation loop. Reset only
    /// between turns, after the previous turn's processes have finished.
    pub fn flag(&self) -> &AtomicBool {
        &self.cancelled
    }

    /// Use the same cancellation flag as the surrounding agent session.
    pub fn new(cancelled: Arc<AtomicBool>) -> Self {
        Self {
            cancelled,
            active: Mutex::new(BTreeSet::new()),
        }
    }

    /// Stop owned processes synchronously. The spawn/register critical
    /// section prevents a process from appearing after cancellation returns.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(active) = self.active.lock() {
            for pid in active.iter().copied() {
                kill_group(pid);
            }
        }
    }
}

/// Why the process stopped.
#[derive(Debug)]
pub enum Termination {
    /// Normal OS completion, including a nonzero exit code or signal.
    Exited(ExitStatus),
    /// The time budget elapsed.
    TimedOut,
    /// The user cancelled.
    Cancelled,
    /// Output exceeded the on-disk capture budget.
    OutputLimit,
}

/// Captured command output, bounded in memory independently for each stream.
#[derive(Debug)]
pub struct Output {
    /// Exit status or stop reason.
    pub termination: Termination,
    /// Standard output, decoded with replacement for invalid UTF-8.
    pub stdout: String,
    /// Standard error, decoded with replacement for invalid UTF-8.
    pub stderr: String,
    /// At least one stream exceeded the returned text limit.
    pub truncated: bool,
}

/// Execution and capture budgets.
pub struct Limits {
    /// Wall time, including processes waiting for input or background jobs.
    pub timeout: Duration,
    /// Maximum bytes returned from each output stream.
    pub capture_bytes: usize,
    /// Stop when combined capture files exceed this budget. Checked while
    /// running; a fast writer can overshoot between checks.
    pub output_bytes: u64,
}

struct Running<'a> {
    child: Child,
    control: &'a Control,
    armed: bool,
}
impl Running<'_> {
    fn stop(&mut self) {
        if !self.armed {
            return;
        }
        let mut active = self.control.active.lock().ok();
        kill_group(self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(active) = active.as_mut() {
            active.remove(&self.child.id());
        }
        self.armed = false;
    }
}
impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A persistent process group with no inherited input/output channels. Used
/// for isolated inference; its only data channel is a dedicated Unix socket.
pub struct Background {
    child: Option<Child>,
    control: Arc<Control>,
}

impl Background {
    /// Start under the same spawn/cancel critical section as tool workers.
    /// No server log is retained or forwarded to the terminal/model.
    pub fn spawn(command: &mut Command, control: Arc<Control>) -> io::Result<Self> {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            command.process_group(0);
        }
        let child = {
            let mut active = control
                .active
                .lock()
                .map_err(|_| io::Error::other("process registry poisoned"))?;
            if control.cancelled.load(Ordering::SeqCst) {
                return Err(io::Error::other("process cancelled"));
            }
            let child = command.spawn()?;
            active.insert(child.id());
            child
        };
        Ok(Self {
            child: Some(child),
            control,
        })
    }

    /// Inspect the owned child; never silently replace a failed model server.
    pub fn is_running(&mut self) -> io::Result<bool> {
        self.child
            .as_mut()
            .map_or(Ok(false), |child| Ok(child.try_wait()?.is_none()))
    }
}

impl Drop for Background {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let mut active = self.control.active.lock().ok();
            kill_group(child.id());
            let _ = child.kill();
            let _ = child.wait();
            if let Some(active) = active.as_mut() {
                active.remove(&child.id());
            }
        }
    }
}

#[cfg(unix)]
fn kill_group(pid: u32) {
    if let Ok(pid) = i32::try_from(pid) {
        let _ = nix::sys::signal::killpg(
            nix::unistd::Pid::from_raw(pid),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
}

#[cfg(windows)]
fn kill_group(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(any(unix, windows)))]
fn kill_group(_: u32) {}

/// Run without interactive stdin. On Unix a fresh process group is killed
/// on completion too, so ordinary background jobs cannot outlive their tool.
/// Commands which deliberately escape that group require an OS sandbox.
pub fn run(command: &mut Command, limits: &Limits, control: &Control) -> io::Result<Output> {
    run_inner(command, Stdio::null(), limits, control)
}

/// Supply confidential input through a private anonymous file, never argv
/// or environment variables. Capture and cancellation have the same limits.
pub fn run_with_input(
    command: &mut Command,
    input: &[u8],
    limits: &Limits,
    control: &Control,
) -> io::Result<Output> {
    let mut file = tempfile::tempfile()?;
    file.write_all(input)?;
    file.rewind()?;
    run_inner(command, file.into(), limits, control)
}

fn run_inner(
    command: &mut Command,
    stdin: Stdio,
    limits: &Limits,
    control: &Control,
) -> io::Result<Output> {
    let directory = tempfile::tempdir()?;
    let mut stdout = File::options()
        .create_new(true)
        .read(true)
        .write(true)
        .open(directory.path().join("stdout"))?;
    let mut stderr = File::options()
        .create_new(true)
        .read(true)
        .write(true)
        .open(directory.path().join("stderr"))?;
    command
        .stdin(stdin)
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let child = {
        let mut active = control
            .active
            .lock()
            .map_err(|_| io::Error::other("process registry poisoned"))?;
        if control.cancelled.load(Ordering::SeqCst) || limits.timeout.is_zero() {
            return Ok(Output {
                termination: if limits.timeout.is_zero() {
                    Termination::TimedOut
                } else {
                    Termination::Cancelled
                },
                stdout: String::new(),
                stderr: String::new(),
                truncated: false,
            });
        }
        let child = command.spawn()?;
        active.insert(child.id());
        child
    };
    let mut running = Running {
        child,
        control,
        armed: true,
    };
    let started = Instant::now();
    let termination = loop {
        if control.cancelled.load(Ordering::SeqCst) {
            break Termination::Cancelled;
        }
        if started.elapsed() >= limits.timeout {
            break Termination::TimedOut;
        }
        if stdout
            .metadata()?
            .len()
            .saturating_add(stderr.metadata()?.len())
            > limits.output_bytes
        {
            break Termination::OutputLimit;
        }
        if let Some(status) = running.child.try_wait()? {
            break Termination::Exited(status);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    running.stop();
    let (stdout, out_cut) = capture(&mut stdout, limits.capture_bytes)?;
    let (stderr, err_cut) = capture(&mut stderr, limits.capture_bytes)?;
    Ok(Output {
        termination,
        stdout,
        stderr,
        truncated: out_cut || err_cut,
    })
}

fn capture(file: &mut File, limit: usize) -> io::Result<(String, bool)> {
    file.seek(SeekFrom::Start(0))?;
    let size = u64::try_from(limit).unwrap_or(u64::MAX);
    let mut bytes = Vec::new();
    file.take(size).read_to_end(&mut bytes)?;
    Ok((
        String::from_utf8_lossy(&bytes).into_owned(),
        file.metadata()?.len() > size,
    ))
}

#[cfg(all(test, unix))]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    fn limits() -> Limits {
        Limits {
            timeout: Duration::from_secs(5),
            capture_bytes: 4096,
            output_bytes: 4_194_304,
        }
    }
    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }
    fn control() -> Control {
        Control::new(Arc::new(AtomicBool::new(false)))
    }

    #[test]
    fn both_large_streams_are_captured_without_pipe_deadlock() {
        let output = run(
            &mut shell("head -c 200000 /dev/zero; head -c 200000 /dev/zero >&2"),
            &limits(),
            &control(),
        )
        .expect("run");
        assert!(matches!(output.termination, Termination::Exited(status) if status.success()));
        assert_eq!(output.stdout.len(), 4096);
        assert_eq!(output.stderr.len(), 4096);
        assert!(output.truncated);
    }

    #[test]
    fn nonzero_exit_keeps_diagnostic_output() {
        let output = run(
            &mut shell("printf failure >&2; exit 7"),
            &limits(),
            &control(),
        )
        .expect("run");
        assert!(
            matches!(output.termination, Termination::Exited(status) if status.code() == Some(7))
        );
        assert_eq!(output.stderr, "failure");
    }

    #[test]
    fn timeout_kills_background_children_before_they_can_write() {
        let directory = tempfile::tempdir().expect("directory");
        let marker = directory.path().join("leaked");
        let mut command = shell("(sleep 0.4; printf leaked > \"$MARKER\") & wait");
        command.env("MARKER", &marker);
        let limits = Limits {
            timeout: Duration::from_millis(70),
            ..limits()
        };
        let output = run(&mut command, &limits, &control()).expect("run");
        assert!(matches!(output.termination, Termination::TimedOut));
        std::thread::sleep(Duration::from_millis(500));
        assert!(!marker.exists());
    }

    #[test]
    fn background_children_do_not_survive_successful_parent_exit() {
        let directory = tempfile::tempdir().expect("directory");
        let marker = directory.path().join("leaked");
        let mut command = shell("(sleep 0.4; printf leaked > \"$MARKER\") & exit 0");
        command.env("MARKER", &marker);
        let output = run(&mut command, &limits(), &control()).expect("run");
        assert!(matches!(output.termination, Termination::Exited(status) if status.success()));
        std::thread::sleep(Duration::from_millis(500));
        assert!(!marker.exists());
    }

    #[test]
    fn cancellation_stops_a_live_command_and_prevents_a_later_spawn() {
        let control = Arc::new(control());
        let thread_control = Arc::clone(&control);
        let thread =
            std::thread::spawn(move || run(&mut shell("sleep 30"), &limits(), &thread_control));
        let deadline = Instant::now();
        while control.active.lock().expect("registry").is_empty()
            && deadline.elapsed() < Duration::from_secs(2)
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!control.active.lock().expect("registry").is_empty());
        control.cancel();
        let output = thread.join().expect("worker").expect("output");
        assert!(matches!(output.termination, Termination::Cancelled));
        let output = run(
            &mut Command::new("definitely-not-installed"),
            &limits(),
            &control,
        )
        .expect("cancel before spawn");
        assert!(matches!(output.termination, Termination::Cancelled));
    }

    #[test]
    fn runaway_output_is_stopped() {
        let limits = Limits {
            output_bytes: 65_536,
            ..limits()
        };
        let output = run(&mut shell("yes output"), &limits, &control()).expect("run");
        assert!(matches!(output.termination, Termination::OutputLimit));
        assert!(output.truncated);
    }
}
