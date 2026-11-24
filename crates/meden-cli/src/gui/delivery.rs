//! Interpret GUI commands through the shared original delivery-review owner.
use super::{Command, View};
use anyhow::Result;
use meden_app::delivery::Delivery;
use meden_runtime::{process::Control, workspace::Workspace};

pub(super) fn begin(delivery: &mut Delivery, command: &Command) {
    if !matches!(
        command,
        Command::SendExport { .. } | Command::RejectExport { .. } | Command::CloseExport { .. }
    ) {
        delivery.clear_review();
    }
}
pub(super) fn observe(delivery: &Delivery, view: &mut View) -> Result<()> {
    let snapshot = delivery.snapshot()?;
    view.exports = snapshot.exports;
    view.export_review = snapshot.review;
    view.export_outcome = snapshot.outcome;
    Ok(())
}
pub(super) fn perform(
    delivery: &mut Delivery,
    command: Command,
    workspace: &Workspace,
    control: &Control,
) -> Result<()> {
    match command {
        Command::ReviewExport { reference, offset } => {
            delivery.review(&reference.original(), usize::try_from(offset)?)
        }
        Command::SendExport { reference, serial } => {
            delivery.send(&reference.original(), serial, workspace, control)
        }
        Command::RejectExport { reference, serial } => {
            delivery.reject(&reference.original(), serial)
        }
        Command::CloseExport { reference, serial } => delivery.close(&reference.original(), serial),
        _ => anyhow::bail!("Unexpected delivery command"),
    }
}
