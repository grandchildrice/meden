//! Host-owned contract decisions, replayed through the actual controller.
//! No proof coverage, running jobs, processor grants or exports are restored.

use std::path::Path;

use crate::verification::{Binding, Contract};
use anyhow::{Context, Result};
use meden_core::evidence::Contracts;
use meden_runtime::verification::artifact::Class;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{obligation, Obligation, Origin, Reference, State};
use crate::identity::{ContractId, RequirementRevision, SessionId};

/// Keep the originally reviewed checker and strength across plugin upgrades.
/// Replaying meaning must not silently adopt a new verification method.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Required {
    statement: Vec<u8>,
    class: Class,
    checker: Vec<u8>,
}
impl Required {
    fn obligation(&self) -> Obligation {
        Obligation {
            statement: self.statement.clone(),
            kind: self.class.core(),
            checker: self.checker.clone(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Event {
    Propose {
        contract: Box<Value>,
        required: Required,
        #[serde(default)]
        binding: Option<Box<Binding>>,
    },
    Review {
        revision: RequirementRevision,
        contract: ContractId,
    },
    Accept {
        revision: RequirementRevision,
        contract: ContractId,
    },
    Reject {
        revision: RequirementRevision,
        contract: ContractId,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    workspace: Vec<u8>,
    #[serde(default)]
    selected: Option<String>,
    events: Vec<Event>,
}

impl Event {
    pub(super) fn propose(contract: &Contract) -> Result<Self> {
        let original = obligation(contract)?;
        Ok(Self::Propose {
            contract: Box::new(serde_json::to_value(contract)?),
            binding: contract.binding().cloned().map(Box::new),
            required: Required {
                statement: original.statement,
                class: contract.description().class,
                checker: original.checker,
            },
        })
    }
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
            Self::Propose {
                contract,
                required,
                binding,
            } => {
                anyhow::ensure!(
                    serde_json::from_slice::<Value>(&required.statement)? == **contract
                        && !required.checker.is_empty()
                        && required.checker.len() <= 256,
                    "saved verification obligation is malformed"
                );
                Contract::recorded(
                    &required.statement,
                    required.class,
                    &required.checker,
                    binding.as_deref().cloned(),
                )?;
                anyhow::ensure!(
                    controller.propose(
                        Origin::Controller,
                        required.statement.clone(),
                        vec![required.obligation()]
                    ),
                    "saved proposal refused"
                );
                return Ok(());
            }
            Self::Review { revision, contract }
            | Self::Accept { revision, contract }
            | Self::Reject { revision, contract } => (revision.value(), contract),
        };
        let displayed = controller
            .displayed()
            .context("saved contract decision has no proposal")?;
        anyhow::ensure!(
            displayed.revision == revision
                && ContractId::from_canonical(&displayed.subject.contract) == *identity,
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

pub(super) fn replay(events: &[Event], session: &SessionId) -> Result<Contracts> {
    let mut controller = Contracts::new(session.as_bytes().to_vec());
    for event in events {
        event.apply(&mut controller, session.as_bytes())?;
    }
    Ok(controller)
}

pub(super) fn load(
    path: &Path,
    root: &Path,
    session: &SessionId,
) -> Result<(Contracts, Vec<Event>, Option<String>)> {
    let Some(saved) = crate::storage::read::<Saved>(path)? else {
        return Ok((
            Contracts::new(session.as_bytes().to_vec()),
            Vec::new(),
            None,
        ));
    };
    anyhow::ensure!(
        saved.version == 2 && saved.workspace == root.as_os_str().as_encoded_bytes(),
        "saved contract journal belongs to another repository or version"
    );
    Ok((
        replay(&saved.events, session)?,
        saved.events,
        saved.selected,
    ))
}

/// Recover the exact original proposal observation, never parse by format again.
pub(super) fn contract(
    view: &meden_core::evidence::ContractView,
    events: &[Event],
) -> Result<Contract> {
    for event in events.iter().rev() {
        if let Event::Propose {
            required, binding, ..
        } = event
        {
            if required.statement == view.subject.contract
                && view.obligations == vec![required.obligation()]
            {
                return Ok(Contract::recorded(
                    &required.statement,
                    required.class,
                    &required.checker,
                    binding.as_deref().cloned(),
                )?);
            }
        }
    }
    anyhow::bail!("Original contract proposal is missing from its journal")
}

pub(super) fn save(
    path: &Path,
    root: &Path,
    selected: Option<&str>,
    events: &[Event],
) -> Result<()> {
    crate::storage::write(
        path,
        &Saved {
            version: 2,
            workspace: root.as_os_str().as_encoded_bytes().to_vec(),
            selected: selected.map(str::to_owned),
            events: events.to_vec(),
        },
    )
}

pub(super) fn record(
    path: &Path,
    root: &Path,
    session: &SessionId,
    selected: Option<&str>,
    state: &mut State,
    event: Event,
) -> Result<()> {
    let mut events = state.events.clone();
    events.push(event);
    // Validate and persist the decision before the live controller commits it.
    // Jobs/edits do not alter proposal generations or host review/accept phases.
    replay(&events, session)?;
    save(path, root, selected, &events)?;
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
        let identity = ContractId::from_canonical(&contract.canonical()?);
        Ok(vec![
            Event::propose(&contract)?,
            Event::Review {
                revision: RequirementRevision::new(1)?,
                contract: identity.clone(),
            },
            Event::Accept {
                revision: RequirementRevision::new(1)?,
                contract: identity,
            },
        ])
    }

    #[test]
    fn replay_preserves_the_reviewed_method_instead_of_recomputing_it() -> Result<()> {
        let mut events = accepted_events()?;
        if let Some(Event::Propose {
            required, binding, ..
        }) = events.first_mut()
        {
            required.checker = b"original reviewed implementation".to_vec();
            required.class = Class::Bounded;
            *binding = None;
        }
        let restored = replay(&events, &SessionId::parse("new-session")?)?;
        let accepted = restored.accepted().context("retained meaning")?;
        assert_eq!(
            accepted.obligations.first().map(|o| o.checker.as_slice()),
            Some(b"original reviewed implementation".as_slice())
        );
        assert_eq!(
            accepted.obligations.first().map(|o| o.kind),
            Some(Class::Bounded.core())
        );
        assert!(!restored.is_active());
        Ok(())
    }

    #[test]
    fn reopening_keeps_meaning_but_never_jobs_evidence_or_the_old_session() -> Result<()> {
        let root = tempfile::tempdir()?;
        let events = accepted_events()?;
        let mut old = replay(&events, &SessionId::parse("old-session")?)?;
        let ticket = old
            .start(Origin::Controller, 1, b"old inputs".to_vec())
            .context("ticket")?;
        let path = root.path().join("contracts.json");
        crate::storage::write(
            &path,
            &Saved {
                version: 2,
                workspace: root.path().as_os_str().as_encoded_bytes().to_vec(),
                selected: None,
                events,
            },
        )?;
        let (mut restored, _, _) = load(&path, root.path(), &SessionId::parse("new-session")?)?;
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
        events.push(Event::propose(&replacement)?);
        events.push(Event::Reject {
            revision: RequirementRevision::new(2)?,
            contract: ContractId::from_canonical(&replacement.canonical()?),
        });
        events.push(Event::propose(&replacement)?);
        let restored = replay(&events, &SessionId::parse("new-session")?)?;
        assert_eq!(restored.accepted().context("retained")?.revision, 1);
        assert_eq!(restored.displayed().context("proposal")?.revision, 3);
        assert!(restored.pending() && !restored.is_accepted());
        Ok(())
    }

    #[test]
    fn failed_persistence_or_unreviewed_acceptance_does_not_commit_a_decision() -> Result<()> {
        let root = tempfile::tempdir()?;
        let project = super::super::Project::new(root.path()).expect("valid fixture workspace");
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
            None,
            &mut state,
            Event::propose(&contract)?
        )
        .is_err());
        assert!(state.events.is_empty() && state.contracts.displayed().is_none());
        let path = root.path().join("contracts.json");
        record(
            &path,
            root.path(),
            &project.session,
            None,
            &mut state,
            Event::propose(&contract)?,
        )?;
        let bytes = std::fs::read(&path)?;
        assert!(record(
            &path,
            root.path(),
            &project.session,
            None,
            &mut state,
            Event::Accept {
                revision: RequirementRevision::new(1)?,
                contract: ContractId::from_canonical(&contract.canonical()?)
            }
        )
        .is_err());
        assert_eq!(std::fs::read(&path)?, bytes);
        Ok(())
    }
}
