//! Logical workspace tool router. File access occurs only in the worker.
use anyhow::Result;
use meden_agent::tool::{Args, Prepared, Tool, ToolError, ToolSig};
use meden_agent::Registry;
use meden_core::file::{Operation, Request};
use meden_core::Policy;
use meden_runtime::process::{self, Control, Limits, Termination};
use meden_runtime::workspace::Workspace;
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

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
        ("write_file", Operation::Write, "Create or replace a regular UTF-8 FILE in the workspace. Supply its complete content. Even empty content creates a file, never a directory. Parent directories must already exist; create directories with bash using mkdir -p before writing nested files.",
         json!({"path":{"type":"string"},"content":{"type":"string"}}), vec!["path","content"]),
        ("edit_file", Operation::Edit, "Replace exactly one occurrence of old_text with different new_text in a UTF-8 workspace file. Read the file first; identical replacements, ambiguous or missing matches fail without writing.",
         json!({"path":{"type":"string"},"old_text":{"type":"string"},"new_text":{"type":"string"}}), vec!["path","old_text","new_text"]),
    ] {
        let sig = ToolSig::new(name, description, json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}));
        registry.register(Box::new(FileTool { workspace: Arc::clone(workspace), control: Arc::clone(control), sig, operation }));
    }

    let mut policy = file_policy()?;
    crate::shell::register(
        &mut registry,
        &mut policy,
        Arc::clone(workspace),
        Arc::clone(control),
    )?;
    Ok((registry, policy))
}

/// Host-selected file authority for the managed workspace. Model callbacks
/// receive their actual admitted policy instead of constructing this policy.
pub fn file_policy() -> Result<Policy> {
    Ok(Policy::from_labels(
        "workspace",
        &[
            "fs.read[/workspace]",
            "fs.read[/workspace/*]",
            "fs.write[/workspace/*]",
        ],
        &[],
    )?)
}

pub fn logical_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
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
    fn prepare(&self, mut args: Args) -> Result<Prepared<'_>, ToolError> {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .and_then(logical_path)
            .ok_or_else(|| ToolError::Failed("path leaves /workspace".into()))?;
        args.remove("path");
        let request = Request::new(self.operation, path.to_string_lossy().as_ref().into(), args);
        Ok(Prepared::new(request.prepare(), |request, rules| {
            self.execute(&request, rules)
        }))
    }
}

impl FileTool {
    fn execute(&self, request: &Request<Args>, rules: &Policy) -> Result<Value, ToolError> {
        let input = serde_json::to_vec(&(request, rules)).map_err(failed)?;
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
