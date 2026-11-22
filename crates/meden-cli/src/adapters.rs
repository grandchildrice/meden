//! Host-selected integrations. Core owns only the neutral Adapter interface;
//! source formats, compiler resources, rendering and execution stay here.
use meden_runtime::{process::Control, workspace::Workspace};
pub(crate) mod informath;
pub(crate) mod lake;
pub(crate) mod lean;
pub(crate) mod source;

pub(crate) struct Context<'a> {
    pub workspace: &'a Workspace,
    pub control: &'a Control,
}
