//! Opt-in shell execution. The workspace is a working directory, not a
//! sandbox: enabling this tool grants the current account's process access.

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use meden_agent::tool::{Args, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::{Capability, Grant, Policy, Requirement};
use meden_runtime::process::{self, Control, Limits, Termination};
use serde_json::json;
use serde_json::Value;

struct Shell {
    root: std::path::PathBuf,
    sig: ToolSig,
    control: Arc<Control>,
}

/// Add shell execution and its unscoped grant when the operator opts in.
pub fn enable(
    registry: &mut Registry,
    policy: &mut Policy,
    root: &Path,
    control: Arc<Control>,
) -> Result<()> {
    let sig = ToolSig::new("bash", "Run a shell command in the workspace (Bash on Unix, cmd on Windows). This is not a sandbox: the command has the account's filesystem and network permissions. Inspect failures before claiming success.", json!({"type":"object","properties":{
        "command":{"type":"string"},"timeout_seconds":{"type":"integer","minimum":1,"maximum":300}
    },"required":["command"],"additionalProperties":false}));
    registry.register(Box::new(Shell {
        root: root.canonicalize()?,
        sig,
        control,
    }));
    policy.grants.push(Grant::parse("shell.exec")?);
    Ok(())
}

impl Tool for Shell {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }
    fn requirements(&self, _: &Args) -> Vec<Requirement> {
        vec![Requirement::with_evidence(
            Capability::ShellExec,
            None::<&str>,
            "bash.command",
        )]
    }
    fn call(&self, args: &Args) -> Result<Value, ToolError> {
        let command = args
            .get("command")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| ToolError::Failed("command must be nonempty text".into()))?;
        let seconds = match args.get("timeout_seconds") {
            None => 60,
            Some(value) if value.as_u64().is_some_and(|n| (1..=300).contains(&n)) => {
                value.as_u64().unwrap_or(60)
            }
            Some(_) => {
                return Err(ToolError::Failed(
                    "timeout_seconds must be an integer from 1 to 300".into(),
                ))
            }
        };
        #[cfg(windows)]
        let mut child = {
            let mut child = Command::new("cmd");
            child.args(["/D", "/S", "/C", command]);
            child
        };
        #[cfg(not(windows))]
        let mut child = {
            let mut child = Command::new("bash");
            child.args(["--noprofile", "--norc", "-c", command]);
            child
        };
        child.current_dir(&self.root);
        let output = process::run(
            &mut child,
            &Limits {
                timeout: Duration::from_secs(seconds),
                capture_bytes: 24_000,
                output_bytes: 16_777_216,
            },
            &self.control,
        )
        .map_err(|e| ToolError::Failed(e.to_string()))?;
        let (success, code, reason) = match output.termination {
            Termination::Exited(status) => (status.success(), status.code(), "exited"),
            Termination::TimedOut => (false, None, "timed_out"),
            Termination::Cancelled => (false, None, "cancelled"),
            Termination::OutputLimit => (false, None, "output_limit"),
        };
        let result = json!({"success":success,"exit_code":code,"status":reason,
            "stdout":output.stdout,"stderr":output.stderr,"output_truncated":output.truncated});
        if !success {
            return Err(ToolError::Failed(result.to_string()));
        }
        Ok(result)
    }
}

#[cfg(all(test, unix))]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn shell_is_absent_until_enabled_and_failing_commands_are_errors() {
        let root = tempfile::tempdir().expect("workspace");
        let (mut registry, mut policy) =
            crate::workspace::environment(root.path()).expect("environment");
        assert!(registry.get("bash").is_none());
        enable(
            &mut registry,
            &mut policy,
            root.path(),
            Arc::new(Control::new(Arc::new(AtomicBool::new(false)))),
        )
        .expect("enable");
        let tool = registry.get("bash").expect("bash");
        let bash_syntax = [(
            "command".to_owned(),
            json!("set -o pipefail; values=(one two); [[ ${values[1]} == two ]]"),
        )]
        .into_iter()
        .collect();
        assert!(
            tool.call(&bash_syntax).is_ok(),
            "the bash tool must support Bash syntax"
        );
        assert!(meden_core::check(&tool.requirements(&Args::new()), &policy).allowed);
        let args = [("command".to_owned(), json!("printf failure >&2; exit 7"))]
            .into_iter()
            .collect();
        let error = tool.call(&args).expect_err("failed command").to_string();
        assert!(error.contains("failure"));
        assert!(error.contains("\"exit_code\":7"));
        policy.denies.push(Capability::ShellExec);
        assert!(!meden_core::check(&tool.requirements(&args), &policy).allowed);
    }
}
