//! Explicit host-selected Lake operations, confined like the other Lean workers.
//! A completed command is an observed check, not native project-proof authority.
use anyhow::Result;
use meden_runtime::{
    process::{self, Control, Limits, Termination},
    sandbox::Sandbox,
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

pub(crate) fn run(
    root: &Path,
    lean: &Path,
    arguments: &[String],
    seconds: u64,
) -> Result<serde_json::Value> {
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    run_controlled(root, lean, arguments, seconds, &control)
}

pub(crate) fn run_controlled(
    root: &Path,
    lean: &Path,
    arguments: &[String],
    seconds: u64,
    control: &Control,
) -> Result<serde_json::Value> {
    anyhow::ensure!(
        !arguments.is_empty() && (1..=7200).contains(&seconds),
        "Lake needs arguments and a timeout from 1 to 7200 seconds"
    );
    let resources = tempfile::tempdir()?;
    std::fs::write(
        resources.path().join("project.sh"),
        include_str!("lean_project.sh"),
    )?;
    std::fs::write(
        resources.path().join("project_hash.py"),
        include_str!("lean_project_hash.py"),
    )?;
    if arguments.first().is_some_and(|a| a == "--meden-inspect") {
        let module = arguments
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("Missing theorem module"))?;
        let names = arguments.get(2..).unwrap_or_default();
        anyhow::ensure!(
            std::iter::once(module)
                .chain(names)
                .all(|name| !name.is_empty()
                    && name.split('.').all(|p| !p.is_empty()
                        && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))),
            "Invalid declaration name"
        );
        std::fs::write(
            resources.path().join("inspect.lean"),
            include_str!("lean_project_inspect.lean"),
        )?;
        std::fs::write(
            resources.path().join("inspect.py"),
            include_str!("lean_project_inspect.py"),
        )?;
    }
    let mut command = Sandbox::lean_command(
        root,
        &std::env::current_exe()?,
        lean,
        resources.path(),
        None,
        "project.sh",
        arguments,
    )?;
    let output = process::run(
        &mut command,
        &Limits {
            timeout: Duration::from_secs(seconds),
            capture_bytes: 2_000_000,
            output_bytes: 32_000_000,
        },
        control,
    )?;
    let success = matches!(&output.termination, Termination::Exited(code) if code.success())
        && !output.truncated;
    Ok(
        serde_json::json!({"success":success,"termination":format!("{:?}", output.termination),"stdout":output.stdout,"stderr":output.stderr}),
    )
}
