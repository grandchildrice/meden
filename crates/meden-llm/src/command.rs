//! Talking to a model through a command-line tool.
//!
//! Several of the agents people already have installed will answer a single
//! prompt non-interactively: `claude -p`, `codex exec`, `hermes chat -q`,
//! `ollama run`. Using one as a planner costs no setup at all, which matters
//! when the point is to try this on the machine you are sitting at.
//!
//! They are requested to act as text generators, but run with their own
//! permissions. Independent actions of those external programs are outside
//! Meden's gate; the gate only rules on the plan they return. HTTP providers
//! keep model access separate from all local tool execution.

use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use meden_agent::planner::PlannerError;

/// Placeholder in an argument list for the prompt.
pub const PROMPT_PLACEHOLDER: &str = "{prompt}";
/// Placeholder in an argument list for a file the tool should write its
/// answer to.
pub const OUTPUT_PLACEHOLDER: &str = "{out}";

/// How to invoke a command-line tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    /// The program to run.
    pub program: String,
    /// Its arguments. [`PROMPT_PLACEHOLDER`] is replaced by the prompt and
    /// [`OUTPUT_PLACEHOLDER`] by a temporary file path.
    pub args: Vec<String>,
    /// Read the answer from the file named by [`OUTPUT_PLACEHOLDER`] rather
    /// than from standard output. Tools that narrate their work need this.
    pub answer_from_file: bool,
    /// How long to wait before giving up and killing the process.
    pub timeout: Duration,
}

impl CommandSpec {
    /// A tool that prints its answer on standard output.
    pub fn stdout(program: &str, args: &[&str]) -> Self {
        CommandSpec {
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            answer_from_file: false,
            timeout: Duration::from_secs(300),
        }
    }

    /// A tool that writes its answer to the file it is given.
    pub fn output_file(program: &str, args: &[&str]) -> Self {
        CommandSpec {
            answer_from_file: true,
            ..CommandSpec::stdout(program, args)
        }
    }

    /// This spec with a different timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The arguments with the placeholders filled in.
    pub fn resolved_args(&self, prompt: &str, out_path: &str) -> Vec<String> {
        self.args
            .iter()
            .map(|a| {
                a.replace(PROMPT_PLACEHOLDER, prompt)
                    .replace(OUTPUT_PLACEHOLDER, out_path)
            })
            .collect()
    }

    /// Run the tool and return what it answered.
    pub fn run(&self, prompt: &str) -> Result<String, PlannerError> {
        let dir = tempfile::tempdir()
            .map_err(|e| PlannerError::Unavailable(format!("no temporary directory: {e}")))?;
        let out_file = dir.path().join("answer.txt");
        let out_path = out_file.to_string_lossy().into_owned();
        let args = self.resolved_args(prompt, &out_path);

        let output = crate::process::run(
            Command::new(&self.program).args(&args),
            &crate::process::Limits {
                timeout: self.timeout,
                capture_bytes: 1_048_576,
                output_bytes: 16_777_216,
            },
            &crate::process::Control::new(Arc::new(AtomicBool::new(false))),
        )
        .map_err(|e| PlannerError::Unavailable(format!("could not run {}: {e}", self.program)))?;

        let (stdout, stderr) = (output.stdout, output.stderr);
        match output.termination {
            crate::process::Termination::Exited(status) if status.success() => {}
            crate::process::Termination::Exited(status) => {
                return Err(PlannerError::Unavailable(format!(
                    "{} exited with {status}{}",
                    self.program,
                    tail(&stderr)
                )))
            }
            reason => {
                return Err(PlannerError::Unavailable(format!(
                    "{} stopped: {reason:?}{}",
                    self.program,
                    tail(&stderr)
                )))
            }
        }
        if output.truncated && !self.answer_from_file {
            return Err(PlannerError::NotAPlan(format!(
                "{} output exceeded the capture limit",
                self.program
            )));
        }

        if self.answer_from_file {
            return read_answer(&out_file).ok_or_else(|| {
                PlannerError::NotAPlan(format!(
                    "{} wrote no usable answer (empty, invalid UTF-8, or over 1 MiB){}",
                    self.program,
                    tail(&stderr)
                ))
            });
        }
        if stdout.trim().is_empty() {
            return Err(PlannerError::NotAPlan(format!(
                "{} said nothing{}",
                self.program,
                tail(&stderr)
            )));
        }
        Ok(stdout)
    }
}

fn read_answer(path: &Path) -> Option<String> {
    let mut text = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(1_048_577)
        .read_to_string(&mut text)
        .ok()?;
    (!text.trim().is_empty() && text.len() <= 1_048_576).then_some(text)
}

/// The last line or two of a tool's complaint, for an error message.
fn tail(stderr: &str) -> String {
    let last: Vec<&str> = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .rev()
        .take(2)
        .collect();
    if last.is_empty() {
        String::new()
    } else {
        let mut lines = last;
        lines.reverse();
        format!(": {}", lines.join(" / "))
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use std::time::Instant;

    #[test]
    fn placeholders_are_filled_in() {
        let spec = CommandSpec::stdout("claude", &["-p", PROMPT_PLACEHOLDER]);
        assert_eq!(
            spec.resolved_args("hello", "/tmp/a"),
            vec!["-p".to_owned(), "hello".to_owned()]
        );

        let spec = CommandSpec::output_file(
            "codex",
            &["exec", "-o", OUTPUT_PLACEHOLDER, PROMPT_PLACEHOLDER],
        );
        assert_eq!(
            spec.resolved_args("hi", "/tmp/out"),
            vec![
                "exec".to_owned(),
                "-o".to_owned(),
                "/tmp/out".to_owned(),
                "hi".to_owned()
            ]
        );
        assert!(spec.answer_from_file);
    }

    #[test]
    fn a_tool_that_prints_is_read_from_stdout() {
        let spec = CommandSpec::stdout("echo", &[PROMPT_PLACEHOLDER]);
        assert_eq!(
            spec.run("hello").map(|s| s.trim().to_owned()),
            Ok("hello".to_owned())
        );
    }

    #[test]
    fn a_tool_that_writes_a_file_is_read_from_it() {
        // `sh -c 'printf hi > "$0"' <out>` writes to the output file.
        let spec = CommandSpec::output_file(
            "sh",
            &["-c", "printf 'from the file' > \"$0\"", OUTPUT_PLACEHOLDER],
        );
        assert_eq!(spec.run("ignored").ok(), Some("from the file".to_owned()));
    }

    #[test]
    fn a_missing_program_is_reported_not_panicked() {
        let spec = CommandSpec::stdout("definitely-not-installed-xyz", &[]);
        assert!(matches!(spec.run("x"), Err(PlannerError::Unavailable(_))));
    }

    #[test]
    fn a_silent_tool_is_an_error_and_quotes_its_complaint() {
        let spec = CommandSpec::stdout("sh", &["-c", "echo 'went wrong' >&2"]);
        match spec.run("x") {
            Err(PlannerError::NotAPlan(msg)) => assert!(msg.contains("went wrong"), "{msg}"),
            other => panic!("expected a NotAPlan error, got {other:?}"),
        }
    }

    #[test]
    fn a_tool_that_hangs_is_killed() {
        let spec = CommandSpec::stdout("sleep", &["30"]).with_timeout(Duration::from_millis(200));
        let start = Instant::now();
        assert!(matches!(spec.run("x"), Err(PlannerError::Unavailable(_))));
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_tool_that_writes_nothing_to_its_file_is_an_error() {
        let spec = CommandSpec::output_file("true", &[OUTPUT_PLACEHOLDER]);
        assert!(matches!(spec.run("x"), Err(PlannerError::NotAPlan(_))));
    }

    #[test]
    fn output_from_a_failed_command_is_not_a_valid_answer() {
        let spec = CommandSpec::stdout("sh", &["-c", "printf partial; exit 7"]);
        assert!(matches!(spec.run("x"), Err(PlannerError::Unavailable(_))));
    }
}
