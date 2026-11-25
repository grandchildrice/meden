//! Host-selected implementations of application role ports. Source formats,
//! compiler resources, rendering and execution stay outside the core.
use meden_runtime::{process::Control, workspace::Workspace};
pub mod informath;
pub mod lake;
pub mod lean;
pub mod plausible;
pub mod preparation;
pub mod source;
#[path = "adapters/source/repair.rs"]
mod source_repair;
#[path = "adapters/source/semantic.rs"]
mod source_semantic;
#[path = "adapters/source/view.rs"]
mod source_view;

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
) -> anyhow::Result<(crate::compiler::Registry<lean::Snapshot>, &'static str)> {
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
pub fn searchers(
    atlas: crate::atlas::Configuration,
) -> anyhow::Result<(
    crate::search::Registry<lean::Snapshot, lean::search::Report>,
    &'static str,
)> {
    let mut registry = crate::search::Registry::default();
    registry.register(plausible::Plausible::new(atlas)?)?;
    Ok((registry, "lean.plausible"))
}
