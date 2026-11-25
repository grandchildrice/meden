//! Trusted TUI review of frozen repair inputs. Physical event provenance is an adapter.
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::prelude::*;
use std::sync::mpsc::Sender;

use super::{approval, App, Screen, Task, Update};
use crate::analysis::repair::Review;
use crate::session::Repairs;

impl App {
    pub(super) fn repair_applied(
        &mut self,
        (snapshot, record): (crate::analysis::prepared::Snapshot, crate::analysis::Record),
    ) {
        self.source_snapshot = Some(snapshot);
        self.analysis = Some(record);
    }

    pub(super) fn show_repair(&mut self, review: Review) {
        self.busy = false;
        self.turn_started = None;
        self.activity = None;
        self.review_area = Rect::default();
        self.review_rows = 0;
        self.scroll_back = 0;
        self.repair_review = Some(review);
        self.screen = Screen::ReviewingRepair;
    }
    pub(super) fn repair_key(&mut self, key: KeyEvent) {
        if self.screen != Screen::ReviewingRepair {
            return;
        }
        let Some(review) = self.repair_review.as_ref() else {
            return;
        };
        let task = if matches!(
            approval::route(
                &self.screen,
                &Event::Key(key),
                self.review_area.width,
                self.review_area.height,
                self.review_rows
            ),
            Some(approval::Choice::Repair)
        ) {
            Task::ApplyRepair(review.reference.clone())
        } else if key.modifiers.is_empty() && matches!(key.code, KeyCode::Esc | KeyCode::Char('n'))
        {
            Task::RejectRepair(review.reference.clone())
        } else {
            return;
        };
        self.repair_review = None;
        self.review_area = Rect::default();
        self.analysis_task(task);
    }
}
pub(super) fn execute(
    session: &crate::session::Session,
    queue: &mut Repairs,
    task: Task,
    tx: &Sender<Update>,
) -> bool {
    let result = (|| -> Result<Option<String>> {
        match task {
            Task::PrepareRepair { snapshot, record } => {
                let review = session.prepare_repair(queue, &snapshot, &record)?;
                tx.send(Update::RepairReview(Box::new(review)))?;
                Ok(None)
            }
            Task::RejectRepair(reference) => {
                session.reject_repair(queue, &reference)?;
                Ok(Some(
                    "Repair rejected. Source files were not changed.".into(),
                ))
            }
            Task::ApplyRepair(reference) => {
                let _ = tx.send(Update::ProjectDirty);
                let (snapshot, record) = session.apply_repair(queue, &reference, &|record| {
                    let _ = tx.send(Update::Analysis(Box::new(record.clone())));
                })?;
                let text = format!(
                    "{}\nRepair applied and reanalyzed. Proof evidence requires a fresh check.",
                    record.summary()
                );
                tx.send(Update::RepairApplied(Box::new((snapshot, record))))?;
                Ok(Some(text))
            }
            _ => unreachable!(),
        }
    })();
    let failed = result.is_err();
    let Some(text) = result.unwrap_or_else(|error|Some(format!("Repair stopped: {error:#}. Inspect the retained files/Finding before requesting another repair."))) else { return true; };
    tx.send(Update::AnalysisDone { text, failed }).is_ok()
}
