//! Logical workspace tool router. File access occurs only in the worker.
use anyhow::Result;
use meden_agent::tool::{Args, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::{Capability, Policy, Requirement};
use meden_runtime::process::{self, Control, Limits, Termination};
use meden_runtime::workspace::Workspace;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Copy)]
enum Operation {
    Read,
    List,
    Write,
    Edit,
}
struct FileTool {
    workspace: Arc<Workspace>,
    control: Arc<Control>,
    sig: ToolSig,
    operation: Operation,
}

pub fn environment(
    workspace: &Arc<Workspace>,
    control: &Arc<Control>,
) -> Result<(Registry, Policy)> {
    let mut registry = Registry::new();
    for (name, operation, description, properties, required) in [
        ("read_file", Operation::Read, "Read a UTF-8 file in the workspace. Offset is a 1-based line number; limit defaults to 200 lines. Use further offsets for long files.",
         json!({"path":{"type":"string"},"offset":{"type":"integer","minimum":1},"limit":{"type":"integer","minimum":1,"maximum":2000}}), vec!["path"]),
        ("list_files", Operation::List, "List a workspace directory, with file types. Use path '.' for the root.",
         json!({"path":{"type":"string"}}), vec!["path"]),
        ("write_file", Operation::Write, "Create or replace a UTF-8 file in the workspace. Supply its complete content. Parent directories must already exist.",
         json!({"path":{"type":"string"},"content":{"type":"string"}}), vec!["path","content"]),
        ("edit_file", Operation::Edit, "Replace exactly one occurrence of old_text with different new_text in a UTF-8 workspace file. Read the file first; identical replacements, ambiguous or missing matches fail without writing.",
         json!({"path":{"type":"string"},"old_text":{"type":"string"},"new_text":{"type":"string"}}), vec!["path","old_text","new_text"]),
    ] {
        let sig = ToolSig::new(name, description, json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}));
        registry.register(Box::new(FileTool { workspace: Arc::clone(workspace), control: Arc::clone(control), sig, operation }));
    }

    let mut policy = Policy::from_labels(
        "workspace",
        &[
            "fs.read[/workspace]",
            "fs.read[/workspace/*]",
            "fs.write[/workspace/*]",
        ],
        &[],
    )?;
    crate::shell::register(
        &mut registry,
        &mut policy,
        Arc::clone(workspace),
        Arc::clone(control),
    )?;
    Ok((registry, policy))
}

pub fn logical_path(args: &Args) -> Option<PathBuf> {
    let path = Path::new(args.get("path")?.as_str()?);
    let relative = if path.is_absolute() {
        path.strip_prefix("/workspace").ok()?
    } else {
        path
    };
    if relative.components().any(|p| {
        matches!(
            p,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(Path::new("/workspace").join(relative))
}

impl Tool for FileTool {
    fn signature(&self) -> &ToolSig {
        &self.sig
    }
    fn requirements(&self, args: &Args) -> Vec<Requirement> {
        let capability = match self.operation {
            Operation::Read | Operation::List => Capability::FsRead,
            Operation::Write | Operation::Edit => Capability::FsWrite,
        };
        let scope = logical_path(args).map(|p| p.to_string_lossy().into_owned());
        vec![Requirement::with_evidence(
            capability,
            scope.as_deref(),
            &self.sig.name,
        )]
    }
    fn call(&self, args: &Args) -> Result<Value, ToolError> {
        if logical_path(args).is_none() {
            return Err(ToolError::Failed("path leaves /workspace".into()));
        }
        let input = serde_json::to_vec(&json!({"operation":self.sig.name, "arguments":args}))
            .map_err(failed)?;
        let mut command = self.workspace.file_command().map_err(failed)?;
        let output = process::run_with_input(
            &mut command,
            &input,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 160_000,
                output_bytes: 1_000_000,
            },
            &self.control,
        )
        .map_err(failed)?;
        if !matches!(output.termination, Termination::Exited(code) if code.success())
            || output.truncated
        {
            return Err(ToolError::Failed(format!(
                "workspace worker {:?}: {}",
                output.termination, output.stderr
            )));
        }
        serde_json::from_str(&output.stdout).map_err(failed)
    }
}
fn failed(error: impl std::fmt::Display) -> ToolError {
    ToolError::Failed(error.to_string())
}
