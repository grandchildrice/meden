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
                    view.source_snapshot
                        .clone()
                        .context("Read sources before requesting a repair")?,
                    view.analysis
                        .clone()
                        .context("Select a retained Finding first")?,
                )
            };
            anyhow::ensure!(
                record.finding_id.as_deref() == Some(finding_id.as_str()),
                "Repair Finding selection changed"
            );
            lock(view).activity = "修正差分を準備しています".into();
            lock(view).repair_review = Some(session.prepare_repair(repairs, &snapshot, &record)?);
        }
        Command::ApplyRepair { reference } => {
            analysis::apply_repair(view, session, repairs, &reference)?;
        }
        Command::RejectRepair { reference } => {
            anyhow::ensure!(
                lock(view)
                    .repair_review
                    .as_ref()
                    .is_some_and(|review| review.reference == reference),
                "Repair review selection changed"
            );
            lock(view).repair_review = None;
            session.reject_repair(repairs, &reference)?;
            lock(view).activity = "修正を拒否しました（ファイルは変更していません）".into();
        }
        _ => anyhow::bail!("not a repair command"),
    }
    Ok(())
}
