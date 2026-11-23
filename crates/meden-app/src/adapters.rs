//! Host-selected implementations of application role ports. Source formats,
//! compiler resources, rendering and execution stay outside the core.
use meden_runtime::{process::Control, workspace::Workspace};
pub mod informath;
pub mod lake;
pub mod lean;
pub mod source;

pub type Observer = std::sync::Arc<
    dyn Fn(&str, std::time::Duration, Option<&meden_runtime::process::OutputProgress>)
        + Send
        + Sync,
>;

pub struct Context<'a> {
    pub workspace: &'a Workspace,
    pub control: &'a Control,
    pub observer: Option<Observer>,
}

/// Compose native implementations from trusted host configuration. Selection
/// happens before dispatch; model/workspace data cannot register an executable.
pub fn compilers(
    atlas: crate::atlas::Configuration,
) -> anyhow::Result<(crate::compiler::Registry, &'static str)> {
    let selected = if lake::enabled()? {
        "lean.lake"
    } else {
        "lean.frozen"
    };
    let mut registry = crate::compiler::Registry::default();
    if selected == "lean.frozen" {
        registry.register(selected, lean::Lean::new(atlas))?;
    } else {
        registry.register(selected, lake::Lake::new(atlas))?;
    }
    Ok((registry, selected))
}

fn compiler_definition(
    id: &str,
    implementation: &[u8],
    atlas: &crate::atlas::Configuration,
    configuration: &serde_json::Value,
) -> anyhow::Result<meden_runtime::plugin::Descriptor> {
    let implementation = [
        implementation,
        include_bytes!("analysis/collect.rs"),
        include_bytes!("analysis/layout.rs"),
        include_bytes!("analysis/observe.rs"),
        include_bytes!("analysis/sources.rs"),
        include_bytes!("compiler.rs"),
        include_bytes!("adapters.rs"),
        include_bytes!("atlas/mod.rs"),
        include_bytes!("atlas/project.rs"),
        include_bytes!("atlas/lake.rs"),
        include_bytes!("atlas/graph.rs"),
    ]
    .concat();
    Ok(meden_runtime::plugin::Descriptor::new(
        meden_runtime::plugin::Definition {
            id,
            role: meden_runtime::plugin::Role::Compiler,
            version: "1",
            formats: vec!["lean-module-v1".into()],
            implementation: &implementation,
            configuration: serde_json::json!({"method":configuration,"resources_sha256":atlas.current_resources()?}),
            input: crate::compiler::input_schema()?,
            output: crate::compiler::output_schema()?,
        },
    )?)
}
