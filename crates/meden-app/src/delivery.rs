//! Host review selection over the same immutable export queue as the TUI and
//! protocol. The original project/kernel owns authorization and physical send.
use crate::{
    exports::{Exports, Reference, Summary},
    view::{delivery as view, sequence::Sequence},
};
use anyhow::{Context, Result};
use meden_runtime::{process::Control, workspace::Workspace};
use std::sync::Arc;

struct Review {
    serial: u64,
    summary: Summary,
    offset: usize,
    body_hex: String,
}
struct Outcome {
    summary: Summary,
    http_status: Option<u16>,
}
/// Own the current selected review; display data cannot replace this selection.
pub struct Delivery {
    /// Shared original proposal queue. Model registration exposes proposals only.
    pub queue: Arc<Exports>,
    serial: u64,
    review: Option<Review>,
    outcome: Option<Outcome>,
}
impl Delivery {
    /// Bind one controller to the original project and session queue.
    pub fn new(project: &Arc<crate::project::Project>) -> Self {
        Self {
            queue: Arc::new(Exports::new(Arc::clone(project))),
            serial: 0,
            review: None,
            outcome: None,
        }
    }
    /// Another host operation retires readiness without approving an export.
    pub fn clear_review(&mut self) {
        self.review = None;
    }
    /// Review original bytes, retaining the exact original request and counter.
    pub fn review(&mut self, reference: &Reference, offset: usize) -> Result<()> {
        self.clear_review();
        let entry = self.queue.review(reference)?;
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
            body_hex: entry.hex(offset, 2048)?,
        });
        Ok(())
    }
    /// Dispatch only after consuming the same original selected review.
    pub fn send(
        &mut self,
        reference: &Reference,
        serial: Sequence,
        workspace: &Workspace,
        control: &Control,
    ) -> Result<()> {
        self.consume(reference, serial)?;
        let (summary, http_status) = self.queue.send(reference, workspace, control)?;
        self.outcome = Some(Outcome {
            summary,
            http_status,
        });
        Ok(())
    }
    /// Host refusal consumes the selected review before invoking the old kernel.
    pub fn reject(&mut self, reference: &Reference, serial: Sequence) -> Result<()> {
        self.consume(reference, serial)?;
        self.outcome = Some(Outcome {
            summary: self.queue.reject(reference)?,
            http_status: None,
        });
        Ok(())
    }
    /// Closing retires the original selected review without sending.
    pub fn close(&mut self, reference: &Reference, serial: Sequence) -> Result<()> {
        self.consume(reference, serial)
    }
    fn consume(&mut self, reference: &Reference, serial: Sequence) -> Result<()> {
        let reviewed = self
            .review
            .take()
            .context("Open the exact export review before acting")?;
        anyhow::ensure!(
            reviewed.serial == serial.value()
                && reviewed.summary.reference.session == reference.session
                && reviewed.summary.reference.id == reference.id,
            "The selected export review changed; open and review it again"
        );
        Ok(())
    }
    /// Project original observations; no workspace reads or transport dispatch.
    pub fn snapshot(&self) -> Result<view::Snapshot> {
        Ok(view::Snapshot {
            exports: self
                .queue
                .list()?
                .iter()
                .map(view::Summary::new)
                .collect::<Result<_>>()?,
            review: self
                .review
                .as_ref()
                .map(|r| view::Review::new(r.serial, &r.summary, r.offset, &r.body_hex))
                .transpose()?,
            outcome: self
                .outcome
                .as_ref()
                .map(|o| view::Outcome::new(&o.summary, o.http_status))
                .transpose()?,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    // Selection comparison only: these private observations do not manufacture
    // a proposal or exercise physical delivery. Public journeys use real queues.
    fn selected(serial: u64) -> Delivery {
        let project = Arc::new(
            crate::project::Project::new(std::path::Path::new("/sessions/sequence"))
                .expect("valid fixture workspace"),
        );
        let mut delivery = Delivery::new(&project);
        delivery.review = Some(Review {
            serial,
            summary: Summary {
                reference: Reference {
                    session: crate::identity::SessionId::parse("original")
                        .expect("valid fixture session"),
                    id: 0,
                },
                source: "empty.bin".into(),
                contract: None,
                target: "http://127.0.0.1:1234/exact".into(),
                bytes: 0,
                payload_sha256: "a".repeat(64),
                state: crate::exports::State::Reviewed,
            },
            offset: 0,
            body_hex: String::new(),
        });
        delivery
    }
    #[test]
    fn full_u64_selection_is_exact_and_every_attempt_consumes_it() -> Result<()> {
        let original = Reference {
            session: crate::identity::SessionId::parse("original")?,
            id: 0,
        };
        let mut delivery = selected(u64::MAX);
        assert_eq!(
            serde_json::to_value(delivery.snapshot()?.review)?.get("serial"),
            Some(&serde_json::json!(u64::MAX.to_string()))
        );
        assert!(delivery
            .consume(&original, Sequence::new(u64::MAX - 1)?)
            .is_err());
        assert!(delivery.review.is_none());
        assert!(delivery
            .consume(&original, Sequence::new(u64::MAX)?)
            .is_err());
        let mut delivery = selected(u64::MAX);
        delivery.consume(&original, Sequence::new(u64::MAX)?)?;
        assert!(delivery.review.is_none());
        for reference in [
            Reference {
                session: crate::identity::SessionId::parse("replacement")?,
                id: 0,
            },
            Reference {
                session: crate::identity::SessionId::parse("original")?,
                id: 1,
            },
        ] {
            let mut delivery = selected(u64::MAX);
            assert!(delivery
                .consume(&reference, Sequence::new(u64::MAX)?)
                .is_err());
            assert!(delivery.review.is_none());
        }
        Ok(())
    }
}
