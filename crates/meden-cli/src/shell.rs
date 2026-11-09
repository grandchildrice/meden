//! Offline Bash execution in the same private workspace as file tools.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use meden_agent::tool::{Args, Prepared, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::{Capability, Grant, Policy, Requirement};
use meden_runtime::process::{self, Control, Limits, Termination};
use meden_runtime::workspace::Workspace;
use serde_json::json;
use serde_json::Value;

struct Shell {
    workspace: Arc<Workspace>,
    sig: ToolSig,
    control: Arc<Control>,
}

/// Register offline Bash with the common workspace tool environment.
pub fn register(
    registry: &mut Registry,
    policy: &mut Policy,
    workspace: Arc<Workspace>,
    control: Arc<Control>,
) -> Result<()> {
    let sig = ToolSig::new("bash", "Run Bash offline in /workspace. Host files, credentials and network services are unavailable. Inspect failures before claiming success.", json!({"type":"object","properties":{
        "command":{"type":"string"},"timeout_seconds":{"type":"integer","minimum":1,"maximum":300}
    },"required":["command"],"additionalProperties":false}));
    registry.register(Box::new(Shell {
        workspace,
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
    fn prepare(&self, args: Args) -> Result<Prepared<'_>, ToolError> {
        let requirements = vec![Requirement::with_evidence(
            Capability::ShellExec,
            None::<&str>,
            "bash.command",
        )];
        Ok(Prepared::new((args, requirements), |args, _| {
            self.execute(&args)
        }))
    }
}

impl Shell {
    fn execute(&self, args: &Args) -> Result<Value, ToolError> {
        let command = args
            .get("command")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| ToolError::failed("command must be nonempty text".into()))?;
        let seconds = match args.get("timeout_seconds") {
            None => 60,
            Some(value) if value.as_u64().is_some_and(|n| (1..=300).contains(&n)) => {
                value.as_u64().unwrap_or(60)
            }
            Some(_) => {
                return Err(ToolError::failed(
                    "timeout_seconds must be an integer from 1 to 300".into(),
                ))
            }
        };
        let mut child = self
            .workspace
            .shell_command()
            .map_err(|e| ToolError::failed(e.to_string()))?;
        let output = process::run_with_input(
            &mut child,
            command.as_bytes(),
            &Limits {
                timeout: Duration::from_secs(seconds),
                capture_bytes: 24_000,
                output_bytes: 16_777_216,
            },
            &self.control,
        )
        .map_err(|e| ToolError::failed(e.to_string()))?;
        let (success, code, reason) = match output.termination {
            Termination::Exited(status) => (status.success(), status.code(), "exited"),
            Termination::TimedOut => (false, None, "timed_out"),
            Termination::Cancelled => (false, None, "cancelled"),
            Termination::OutputLimit => (false, None, "output_limit"),
        };
        let result = json!({"success":success,"exit_code":code,"status":reason,
            "stdout":output.stdout,"stderr":output.stderr,"output_truncated":output.truncated});
        if !success {
            return Err(ToolError::failed(result.to_string()));
        }
        Ok(result)
    }
}
