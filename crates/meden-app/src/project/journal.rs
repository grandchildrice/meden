//! Host-owned contract decisions, replayed through the actual controller.
//! No proof coverage, running jobs, processor grants or exports are restored.

use std::path::Path;

use anyhow::{Context, Result};
use meden_core::evidence::Contracts;
use meden_runtime::verification::Contract;
use serde::{Deserialize, Serialize};

use super::{hash, obligation, Origin, Reference, State};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Event {
    Propose { contract: Box<Contract> },
    Review { revision: u64, contract: String },
    Accept { revision: u64, contract: String },
    Reject { revision: u64, contract: String },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    workspace: Vec<u8>,
    events: Vec<Event>,
}

impl Event {
    pub(super) fn review(reference: &Reference) -> Self {
        Self::Review {
            revision: reference.revision,
            contract: reference.contract.clone(),
        }
    }
    pub(super) fn accept(reference: &Reference) -> Self {
        Self::Accept {
            revision: reference.revision,
            contract: reference.contract.clone(),
        }
    }
    pub(super) fn reject(reference: &Reference) -> Self {
        Self::Reject {
            revision: reference.revision,
            contract: reference.contract.clone(),
        }
    }

    fn apply(&self, controller: &mut Contracts, session: &[u8]) -> Result<()> {
        let (revision, identity) = match self {
            Self::Propose { contract } => {
                contract.validate()?;
                anyhow::ensure!(
                    controller.propose(
                        Origin::Controller,
                        contract.canonical()?,
                        vec![obligation(contract)?]
                    ),
                    "saved proposal refused"
                );
                return Ok(());
            }
            Self::Review { revision, contract }
            | Self::Accept { revision, contract }
            | Self::Reject { revision, contract } => (*revision, contract),
        };
        let displayed = controller
            .displayed()
            .context("saved contract decision has no proposal")?;
        anyhow::ensure!(
            displayed.revision == revision && hash(&displayed.subject.contract) == *identity,
            "saved contract decision does not match the displayed meaning"
        );
        let accepted = match self {
            Self::Review { .. } => controller.review(Origin::Host, session, revision),
            Self::Accept { .. } => controller.accept(Origin::Host, session, revision),
            Self::Reject { .. } => controller.reject(Origin::Host, session, revision),
            Self::Propose { .. } => false,
        };
        anyhow::ensure!(accepted, "saved contract decision refused");
        Ok(())
    }
}

pub(super) fn replay(events: &[Event], session: &str) -> Result<Contracts> {
    let mut controller = Contracts::new(session.as_bytes().to_vec());
    for event in events {
        event.apply(&mut controller, session.as_bytes())?;
    }
    Ok(controller)
}

pub(super) fn load(path: &Path, root: &Path, session: &str) -> Result<(Contracts, Vec<Event>)> {
    let Some(saved) = crate::storage::read::<Saved>(path)? else {
        return Ok((Contracts::new(session.as_bytes().to_vec()), Vec::new()));
    };
    anyhow::ensure!(
        saved.version == 1 && saved.workspace == root.as_os_str().as_encoded_bytes(),
        "saved contract journal belongs to another repository or version"
    );
    Ok((replay(&saved.events, session)?, saved.events))
}

pub(super) fn record(
    path: &Path,
    root: &Path,
    session: &str,
    state: &mut State,
    event: Event,
) -> Result<()> {
    let mut events = state.events.clone();
    events.push(event);
    // Validate and persist the decision before the live controller commits it.
    // Jobs/edits do not alter proposal generations or host review/accept phases.
    replay(&events, session)?;
    crate::storage::write(
        path,
        &Saved {
            version: 1,
            workspace: root.as_os_str().as_encoded_bytes().to_vec(),
            events: events.clone(),
        },
    )?;
    state.events = events;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted_events() -> Result<Vec<Event>> {
        let contract = Contract::parse(include_str!(
            "../../../../verification/examples/counter.contract.json"
        ))?;
        let identity = contract.identity()?;
        Ok(vec![
            Event::Propose {
                contract: Box::new(contract),
            },
            Event::Review {
                revision: 1,
                contract: identity.clone(),
            },
            Event::Accept {
                revision: 1,
                contract: identity,
            },
        ])
    }

    #[test]
    fn reopening_keeps_meaning_but_never_jobs_evidence_or_the_old_session() -> Result<()> {
        let root = tempfile::tempdir()?;
        let events = accepted_events()?;
        let mut old = replay(&events, "old-session")?;
        let ticket = old
            .start(Origin::Controller, 1, b"old inputs".to_vec())
            .context("ticket")?;
        let path = root.path().join("contracts.json");
        crate::storage::write(
            &path,
            &Saved {
                version: 1,
                workspace: root.path().as_os_str().as_encoded_bytes().to_vec(),
                events,
            },
        )?;
        let (mut restored, _) = load(&path, root.path(), "new-session")?;
        let accepted = restored.accepted().context("accepted meaning")?;
        assert_eq!(accepted.revision, 1);
        assert_eq!(
            accepted.subject.contract,
            old.accepted().context("old meaning")?.subject.contract
        );
        assert_eq!(accepted.subject.session, b"new-session");
        assert!(!restored.is_active() && !restored.eligible(1, b"old inputs".to_vec()));
        assert!(!restored.abort(Origin::Controller, &ticket));
        Ok(())
    }

    #[test]
    fn rejecting_then_reproposing_keeps_monotonic_revisions_after_restart() -> Result<()> {
        let mut events = accepted_events()?;
        let mut replacement = serde_json::to_value(Contract::parse(include_str!(
            "../../../../verification/examples/counter.contract.json"
        ))?)?;
        *replacement.get_mut("description").context("description")? =
            "Changed reviewed meaning".into();
        let replacement = Contract::parse(&serde_json::to_string(&replacement)?)?;
        events.push(Event::Propose {
            contract: Box::new(replacement.clone()),
        });
        events.push(Event::Reject {
            revision: 2,
            contract: replacement.identity()?,
        });
        events.push(Event::Propose {
            contract: Box::new(replacement),
        });
        let restored = replay(&events, "new-session")?;
        assert_eq!(restored.accepted().context("retained")?.revision, 1);
        assert_eq!(restored.displayed().context("proposal")?.revision, 3);
        assert!(restored.pending() && !restored.is_accepted());
        Ok(())
    }

    #[test]
    fn failed_persistence_or_unreviewed_acceptance_does_not_commit_a_decision() -> Result<()> {
        let root = tempfile::tempdir()?;
        let project = super::super::Project::new(root.path());
        let mut state = project.state.lock().map_err(|_| anyhow::anyhow!("state"))?;
        let contract = Contract::parse(include_str!(
            "../../../../verification/examples/counter.contract.json"
        ))?;
        let blocked = root.path().join("not-a-directory");
        std::fs::write(&blocked, "block")?;
        assert!(record(
            &blocked.join("contracts.json"),
            root.path(),
            &project.session,
            &mut state,
            Event::Propose {
                contract: Box::new(contract.clone())
            }
        )
        .is_err());
        assert!(state.events.is_empty() && state.contracts.displayed().is_none());
        let path = root.path().join("contracts.json");
        record(
            &path,
            root.path(),
            &project.session,
            &mut state,
            Event::Propose {
                contract: Box::new(contract.clone()),
            },
        )?;
        let bytes = std::fs::read(&path)?;
        assert!(record(
            &path,
            root.path(),
            &project.session,
            &mut state,
            Event::Accept {
                revision: 1,
                contract: contract.identity()?
            }
        )
        .is_err());
        assert_eq!(std::fs::read(&path)?, bytes);
        Ok(())
    }
}
