//! GUI selection of the same immutable, one-use exports used by the public TUI.
use std::sync::Arc;

use anyhow::{Context, Result};
use meden_runtime::{process::Control, workspace::Workspace};
use serde::Serialize;

use super::{Command, View};
use crate::exports::{Exports, Reference, Summary};

const PAGE_BYTES: usize = 2048;

#[derive(Clone, Serialize)]
pub(super) struct Review {
    serial: u64,
    summary: Summary,
    offset: usize,
    body_hex: String,
}

#[derive(Clone, Serialize)]
pub(super) struct Outcome {
    summary: Summary,
    http_status: Option<u16>,
}

pub(super) struct Delivery {
    pub queue: Arc<Exports>,
    serial: u64,
    review: Option<Review>,
    outcome: Option<Outcome>,
}

impl Delivery {
    pub fn new(project: &Arc<crate::project::Project>) -> Self {
        Self {
            queue: Arc::new(Exports::new(Arc::clone(project))),
            serial: 0,
            review: None,
            outcome: None,
        }
    }

    pub fn begin(&mut self, command: &Command) {
        if !matches!(
            command,
            Command::SendExport { .. } | Command::RejectExport { .. } | Command::CloseExport { .. }
        ) {
            self.review = None;
        }
    }

    pub fn observe(&self, view: &mut View) -> Result<()> {
        view.export_review.clone_from(&self.review);
        view.export_outcome.clone_from(&self.outcome);
        view.exports = self.queue.list()?;
        Ok(())
    }

    pub fn perform(
        &mut self,
        command: Command,
        workspace: &Workspace,
        control: &Control,
    ) -> Result<()> {
        match command {
            Command::ReviewExport { reference, offset } => {
                let entry = self.queue.review(&reference)?;
                let summary = entry.summary()?;
                anyhow::ensure!(
                    offset == 0 || offset < summary.bytes,
                    "review offset outside payload"
                );
                self.serial = self
                    .serial
                    .checked_add(1)
                    .context("export review sequence exhausted")?;
                self.review = Some(Review {
                    serial: self.serial,
                    summary,
                    offset,
                    body_hex: entry.hex(offset, PAGE_BYTES)?,
                });
            }
            Command::SendExport { reference, serial } => {
                self.consume(&reference, serial)?;
                let (summary, http_status) = self.queue.send(&reference, workspace, control)?;
                self.outcome = Some(Outcome {
                    summary,
                    http_status,
                });
            }
            Command::RejectExport { reference, serial } => {
                self.consume(&reference, serial)?;
                self.outcome = Some(Outcome {
                    summary: self.queue.reject(&reference)?,
                    http_status: None,
                });
            }
            Command::CloseExport { reference, serial } => {
                self.consume(&reference, serial)?;
            }
            _ => anyhow::bail!("Unexpected delivery command"),
        }
        Ok(())
    }

    fn consume(&mut self, reference: &Reference, serial: u64) -> Result<()> {
        let reviewed = self
            .review
            .take()
            .context("Open the exact export review before acting")?;
        anyhow::ensure!(
            reviewed.serial == serial
                && reviewed.summary.reference.session == reference.session
                && reviewed.summary.reference.id == reference.id,
            "The selected export review changed; open and review it again"
        );
        Ok(())
    }
}
