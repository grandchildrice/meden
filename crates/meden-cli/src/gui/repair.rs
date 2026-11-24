//! Browser host adapter for the shared one-use repair queue.
use super::{analysis, lock, Command, View};
use anyhow::{Context, Result};
use std::sync::Mutex;

pub(super) fn perform(
    view: &Mutex<View>,
    session: &crate::session::Session,
    repairs: &mut crate::analysis::repair::Queue,
    command: Command,
) -> Result<()> {
    match command {
        Command::PrepareRepair { finding_id } => {
            session.abandon_repairs(repairs)?;
            lock(view).repair_review = None;
            let (snapshot, record) = {
                let view = lock(view);
                (
                    view.sources
                        .current()
                        .cloned()
                        .context("Read sources before requesting a repair")?,
                    view.analyses
                        .data()
                        .current
                        .clone()
                        .context("Select a retained Finding first")?,
                )
            };
            anyhow::ensure!(
                record.finding_id.as_deref() == Some(finding_id.as_str()),
                "Repair Finding selection changed"
            );
            lock(view).activity = "Preparing patch".into();
            lock(view).repair_review = Some(meden_app::view::repairs::Review::new(
                session.prepare_repair(repairs, &snapshot, &record)?,
            )?);
        }
        Command::ApplyRepair { reference } => {
            analysis::apply_repair(view, session, repairs, &reference)?;
        }
        Command::RejectRepair { reference } => {
            anyhow::ensure!(
                lock(view)
                    .repair_review
                    .as_ref()
                    .is_some_and(|review| review.original().reference == reference),
                "Repair review selection changed"
            );
            lock(view).repair_review = None;
            session.reject_repair(repairs, &reference)?;
            lock(view).activity = "Patch rejected; files unchanged".into();
        }
        _ => anyhow::bail!("not a repair command"),
    }
    Ok(())
}

pub(super) fn begin(
    view: &std::sync::Mutex<super::View>,
    workspace: &meden_runtime::workspace::Workspace,
    repairs: &mut crate::analysis::repair::Queue,
    command: &super::Command,
) -> anyhow::Result<()> {
    if !matches!(
        command,
        super::Command::PrepareRepair { .. }
            | super::Command::ApplyRepair { .. }
            | super::Command::RejectRepair { .. }
    ) {
        repairs.abandon(workspace)?;
        super::lock(view).repair_review = None;
    }
    Ok(())
}
