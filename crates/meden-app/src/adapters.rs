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
    registry.register("lean.frozen", lean::Lean::new(atlas.clone()))?;
    registry.register("lean.lake", lake::Lake::new(atlas))?;
    Ok((registry, selected))
}
