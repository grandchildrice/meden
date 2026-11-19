//! Model access to the same confined, historical Lean work checks as the host UI.
use std::sync::Arc;

use meden_agent::tool::{Args, Prepared, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::{Capability, Policy, Requirement};
use meden_runtime::{process::Control, workspace::Workspace};
use serde_json::{json, Value};

struct LeanWork {
    workspace: Arc<Workspace>,
    control: Arc<Control>,
    project: Arc<crate::project::Project>,
    sig: ToolSig,
}

pub(crate) fn register(
    registry: &mut Registry,
    workspace: &Arc<Workspace>,
    control: &Arc<Control>,
    project: &Arc<crate::project::Project>,
) {
    registry.register(Box::new(LeanWork {
        workspace: Arc::clone(workspace),
        control: Arc::clone(control),
        project: Arc::clone(project),
        sig: ToolSig::new(
            "check_lean_work",
            "Compile an existing .meden work item's Lean module and freshly inspect its stored theorem names using the installed host Lean compiler and offline Lake. Use this for Lean compiler feedback; do not search for or install Lean in Bash. Supply only the stored work id, never a checker, command, receipt or replacement requirement. Repository Lake/Lean code executes with workspace read/write access in the checker sandbox. Compiler rejection is repairable; cancellation, unavailable resources and limits stop the turn. Results, including failures, are saved in .meden history. Success is a local historical check, not project_status.verified or native Rust contract acceptance.",
            json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":false}),
        ),
    }));
}

impl Tool for LeanWork {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }

    fn prepare(&self, args: Args) -> Result<Prepared<'_>, ToolError> {
        let id = args
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| crate::handoff::valid_name(id))
            .filter(|_| args.len() == 1)
            .ok_or_else(|| ToolError::failed("Supply only an existing work item's id".into()))?
            .to_owned();
        // Shell execution admits repository code with the same writable root as
        // Bash. File scopes govern the separate metadata operations; they do not
        // restrict code already admitted under shell.exec.
        let requirements = vec![
            Requirement::new(Capability::ShellExec, None),
            Requirement::new(Capability::ProcessSpawn, None),
            Requirement::new(Capability::FsRead, Some("/workspace/.meden/workspace.json")),
            Requirement::new(Capability::FsRead, Some("/workspace/lean-toolchain")),
            Requirement::new(
                Capability::FsWrite,
                Some("/workspace/.meden/workspace.json"),
            ),
        ];
        Ok(Prepared::new((id, requirements), |id, rules| {
            self.execute(&id, rules)
        }))
    }
}

impl LeanWork {
    fn execute(&self, id: &str, rules: &Policy) -> Result<Value, ToolError> {
        // Executable project configuration may mutate the source. A historical
        // Lean observation cannot preserve an older native Rust proof result.
        self.project.mark_dirty();
        let observation = crate::handoff::check_observed(&self.workspace, &self.control, id, rules)
            .map_err(|error| {
                if self
                    .control
                    .flag()
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    ToolError::cancelled()
                } else {
                    ToolError::unavailable(format!("Lean work checker unavailable: {error:#}"))
                }
            })?;
        let value = serde_json::to_value(&observation.record)
            .map_err(|error| ToolError::unavailable(error.to_string()))?;
        if observation.record.success {
            return Ok(value);
        }
        let text = value.to_string();
        Err(match observation.outcome {
            crate::lean_project::Outcome::Completed => ToolError::failed(text),
            crate::lean_project::Outcome::Cancelled => ToolError::cancelled(),
            crate::lean_project::Outcome::Inconclusive => ToolError::inconclusive(text),
            crate::lean_project::Outcome::Unavailable => ToolError::unavailable(text),
        })
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use meden_agent::conversation::{execute, ExecutionError, FunctionCall, ToolCall};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn missing_authority_refuses_before_any_history_or_checker_execution() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let workspace = Arc::new(Workspace::open(root.path(), &std::env::current_exe()?)?);
        let control = Arc::new(Control::new(Arc::new(AtomicBool::new(false))));
        let project = Arc::new(crate::project::Project::new(root.path()));
        let mut registry = Registry::new();
        register(&mut registry, &workspace, &control, &project);
        let call = ToolCall {
            id: "scoped-check".into(),
            function: FunctionCall {
                name: "check_lean_work".into(),
                arguments: json!({"id":"proof"}).to_string(),
            },
        };
        for denied in ["shell.exec", "process.spawn", "fs.read", "fs.write"] {
            let policy = Policy::from_labels(
                "test",
                &[
                    "shell.exec",
                    "process.spawn",
                    "fs.read[/workspace]",
                    "fs.read[/workspace/*]",
                    "fs.write[/workspace/*]",
                ],
                &[denied],
            )?;
            let result = execute(&call, &registry, &policy, &|_| {}, control.flag(), &|| None);
            assert!(
                matches!(result, Err(ExecutionError::Refused(_))),
                "{denied}: {result:?}"
            );
            assert!(!root.path().join(".meden").exists());
            assert!(!root.path().join(".lake").exists());
        }
        // A one-character file scope can match a literal '*' resource, but
        // cannot authorize the actual, longer metadata paths the checker uses.
        let narrow = Policy::from_labels(
            "single-character-paths",
            &[
                "shell.exec",
                "process.spawn",
                "fs.read[/workspace]",
                "fs.read[/workspace/?]",
                "fs.write[/workspace/?]",
            ],
            &[],
        )?;
        assert!(matches!(
            execute(&call, &registry, &narrow, &|_| {}, control.flag(), &|| None),
            Err(ExecutionError::Refused(_))
        ));
        assert!(!root.path().join(".meden").exists());
        assert!(!root.path().join(".lake").exists());
        Ok(())
    }
}
