//! Latest original verification observation. These identities are not kernel
//! tickets, receipts or grants; only the project service owns the publisher.
use super::{
    job_id::JobId,
    publication::{ArtifactId, Envelope, Publication},
};
use crate::{
    identity::{ContractId, SessionId},
    project,
    verification::Contract,
};
use anyhow::{Context as _, Result};
use meden_runtime::verification::{artifact::Class, FailureKind};
use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[allow(missing_docs)]
pub(crate) mod recorded;
/// Original portable-work observations, never project proof authority.
pub use recorded::{Context as RecordedContext, Origin as RecordedOrigin, Phase as RecordedPhase};
pub(crate) mod tracker;

/// Original operation subject. A draft has no accepted revision or evidence.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Subject {
    /// Check a new contract before offering its meaning for host review.
    Proposal {
        /// Original project session, not a grant.
        #[schemars(with = "String", length(min = 1, max = 256))]
        session: SessionId,
        /// Complete canonical proposed contract identity.
        contract: ContractId,
    },
    /// Check implementation against the host-accepted original meaning.
    Accepted {
        /// Exact original accepted revision/session/contract.
        reference: super::evidence::Reference,
    },
}
/// Frozen original subject and host observation identity.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Context {
    job: JobId<Job>,
    subject: Subject,
    #[schemars(length(min = 1, max = 4096))]
    source_path: String,
    name: String,
    backend: String,
    class: Class,
    scope: String,
}
/// Original captured source and the complete prepared-input serialization hash.
/// A viewer cannot use these hashes to issue or complete an execution.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    source_sha256: String,
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    prepared_sha256: String,
}
/// Actual application boundary with exactly the inputs known there. A checking
/// stage without prepared inputs is not representable, including after failure.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Stage {
    /// Validate a draft through its originally registered verifier.
    CheckingProposal,
    /// Retain the checked draft for a separate host review.
    RecordingProposal,
    /// Read the accepted source under its frozen policy.
    ReadingSource,
    /// Prepare the originally registered checker and its inputs.
    PreparingChecker {
        /// Complete original source bytes, not metadata or a prefix.
        #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
        source_sha256: String,
    },
    /// Execute the already-issued check.
    Checking {
        /// Original complete prepared input binding.
        inputs: Inputs,
    },
    /// Observe the source again after checker completion.
    RefreshingSource {
        /// Original check input; never replaced by the freshness read.
        inputs: Inputs,
    },
    /// Stage outputs and ask the existing controller to admit them.
    RetainingEvidence {
        /// Original check input.
        inputs: Inputs,
    },
}
/// Input-bound progress and disjoint terminal observations. Diagnostics do not
/// select a phase, stop class or evidence strength.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Phase {
    /// A checked draft is available for review. This is not acceptance or
    /// implementation evidence, including when older accepted meaning remains.
    Proposed {
        /// Original newly displayed proposal reference.
        reference: super::evidence::Reference,
    },
    /// The host is executing this actual application stage.
    Running {
        /// Exact known input, frozen along with its operation boundary.
        stage: Stage,
    },
    /// Reported only after actual project evidence admission. This display
    /// observation cannot restore that receipt, acceptance or current freshness.
    Admitted {
        /// Original input binding for the actual admitted result.
        inputs: Inputs,
        /// The application's original strength-specific completion text.
        completion_label: String,
    },
    /// Original typed failure, without classifying diagnostic words.
    Failed {
        /// Last actual operation boundary.
        during: Stage,
        /// Original host classification.
        kind: FailureKind,
        /// Bounded literal diagnostic, explicitly marked if shortened.
        #[schemars(length(max = 8000))]
        message: String,
    },
    /// Failed execution with an independently observed host cancellation flag.
    Cancelled {
        /// Last actual operation boundary.
        during: Stage,
        /// Bounded original diagnostic.
        #[schemars(length(max = 8000))]
        message: String,
    },
}
impl Phase {
    fn stage(&self) -> Option<&Stage> {
        match self {
            Self::Running { stage } => Some(stage),
            Self::Proposed { .. }
            | Self::Admitted { .. }
            | Self::Failed { .. }
            | Self::Cancelled { .. } => None,
        }
    }
}
/// One latest complete observation. No successful history prefix is invented.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Data {
    /// Original project proposal or accepted implementation check.
    Project {
        /// Frozen original project meaning.
        context: Context,
        /// Host monotonic elapsed time.
        elapsed_ms: u32,
        /// Actual application boundary/result.
        phase: Phase,
    },
    /// A portable work check is historical and cannot carry admitted evidence.
    Recorded {
        /// Original requested work and host session.
        context: RecordedContext,
        /// Host monotonic elapsed time.
        elapsed_ms: u32,
        /// Actual historical-check boundary/result.
        phase: RecordedPhase,
    },
}
impl Data {
    fn job(&self) -> &JobId<Job> {
        match self {
            Self::Project { context, .. } => &context.job,
            Self::Recorded { context, .. } => &context.job,
        }
    }
    fn role(&self) -> Role {
        match self {
            Self::Project { .. } => Role::Project,
            Self::Recorded { context, .. } => match context.origin {
                RecordedOrigin::Host => Role::RecordedHost,
                RecordedOrigin::Model => Role::RecordedModel,
            },
        }
    }
    fn elapsed(&self) -> u32 {
        match self {
            Self::Project { elapsed_ms, .. } | Self::Recorded { elapsed_ms, .. } => *elapsed_ms,
        }
    }
    fn running(&self) -> bool {
        match self {
            Self::Project { phase, .. } => phase.stage().is_some(),
            Self::Recorded { phase, .. } => phase.stage().is_some(),
        }
    }
}
/// Complete generated envelope, independent of the accepted-evidence body.
pub type Snapshot = Envelope<Data>;
/// Frozen display destination; it grants no execution or evidence authority.
#[derive(Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Project proposal/accepted check invoked from the conversation.
    Project,
    /// Explicit host work check, shown in work history without activating chat.
    RecordedHost,
    /// Historical work check from an explicitly sent model turn.
    RecordedModel,
}
/// Small frequent status; the complete context and diagnostic are queried once.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Status {
    /// No verification has started in this host session; no body is requested.
    Idle,
    /// The complete latest observation is frozen and queryable.
    Ready {
        /// Original requested display destination, not inferred from diagnostics.
        role: Role,
        /// Original host/session/contract-bound job, independent of body revision.
        job: JobId<Job>,
        /// Exact content identity, never a job ticket or receipt.
        revision: ArtifactId<Artifact>,
    },
    /// No partial or superseded body is admitted.
    Unavailable {
        /// Original display destination when known; lock failure stays unknown.
        role: Option<Role>,
        /// Bounded observation/projection failure.
        #[schemars(length(max = 8000))]
        message: String,
    },
}

struct State {
    epoch: [u8; 32],
    sequence: u64,
    original: Option<Data>,
    status: Status,
    publication: Option<Publication<Artifact>>,
}
impl State {
    fn publish(&mut self) {
        let Some(original) = self.original.as_ref() else {
            self.status = Status::Idle;
            self.publication = None;
            return;
        };
        let _timing = meden_runtime::metrics::Span::start("view.verification_job.materialize");
        let result = match &self.publication {
            Some(old) => old.update(original),
            None => Publication::new(original),
        };
        match result {
            Ok(value) => {
                self.status = Status::Ready {
                    role: original.role(),
                    job: original.job().clone(),
                    revision: value.reference().revision.clone(),
                };
                self.publication = Some(value);
            }
            Err(error) => self.unavailable(&error.to_string()),
        }
    }
    fn unavailable(&mut self, message: &str) {
        self.unavailable_for(self.original.as_ref().map(Data::role), message);
    }
    fn unavailable_for(&mut self, role: Option<Role>, message: &str) {
        self.publication = None;
        self.status = Status::Unavailable {
            role,
            message: diagnostic(message),
        };
    }
    fn active(&self, observation: &Observation, elapsed: u32) -> bool {
        matches!(self.status, Status::Ready { .. })
            && self.original.as_ref().is_some_and(|data| {
                data.job() == &observation.job && elapsed >= data.elapsed() && data.running()
            })
    }
}
fn diagnostic(message: &str) -> String {
    super::publication::diagnostic(message, "\n… (diagnostic shortened)")
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// A non-deserializable producer handle. Replacement and terminal completion
/// retire it. It carries no kernel ticket and cannot cause an execution.
pub(crate) struct Observation {
    job: JobId<Job>,
}
/// The project service alone creates and advances these display observations.
pub(crate) struct Publisher(Arc<Mutex<State>>);
impl Publisher {
    pub(crate) fn new(session: &[u8]) -> Self {
        let mut state = State {
            epoch: Sha256::digest(session).into(),
            sequence: 0,
            original: None,
            status: Status::Idle,
            publication: None,
        };
        state.publish();
        Self(Arc::new(Mutex::new(state)))
    }
    pub(crate) fn reader(&self) -> Reader {
        Reader(Arc::clone(&self.0))
    }
    pub(crate) fn begin(
        &self,
        reference: &project::Reference,
        contract: &Contract,
    ) -> Option<Observation> {
        self.begin_subject(contract, Stage::ReadingSource, || {
            anyhow::ensure!(
                contract.identity()? == reference.contract.as_str(),
                "Verification contract identity changed"
            );
            Ok(Subject::Accepted {
                reference: super::evidence::Reference::new(reference),
            })
        })
    }
    pub(crate) fn begin_proposal(
        &self,
        session: &SessionId,
        contract: &Contract,
    ) -> Option<Observation> {
        self.begin_subject(contract, Stage::CheckingProposal, || {
            anyhow::ensure!(session.as_str().len() <= 256, "Invalid proposal session");
            Ok(Subject::Proposal {
                session: session.clone(),
                contract: ContractId::from_canonical(&contract.canonical()?),
            })
        })
    }
    fn begin_subject(
        &self,
        contract: &Contract,
        initial: Stage,
        subject: impl FnOnce() -> Result<Subject>,
    ) -> Option<Observation> {
        let mut state = self.0.lock().ok()?;
        let prepared = (|| -> Result<(u64, Context)> {
            let sequence = state
                .sequence
                .checked_add(1)
                .context("Verification observation sequence exhausted")?;
            let subject = subject()?;
            let description = contract.description();
            anyhow::ensure!(
                !description.source.is_empty() && description.source.len() <= 4096,
                "Verification resource exceeds view bounds"
            );
            let mut hash = Sha256::new();
            hash.update(b"meden.verification.job.v2\0");
            hash.update(state.epoch);
            hash.update(sequence.to_le_bytes());
            hash.update(serde_json::to_vec(&subject)?);
            let context = Context {
                job: JobId::from_digest(hash.finalize()),
                subject,
                source_path: description.source.clone(),
                name: description.name.clone(),
                backend: contract.backend().into(),
                class: description.class,
                scope: description.scope.clone(),
            };
            Ok((sequence, context))
        })();
        match prepared {
            Ok((sequence, context)) => {
                let observation = Observation {
                    job: context.job.clone(),
                };
                state.sequence = sequence;
                state.original = Some(Data::Project {
                    context,
                    elapsed_ms: 0,
                    phase: Phase::Running { stage: initial },
                });
                state.publish();
                Some(observation)
            }
            Err(error) => {
                state.unavailable_for(Some(Role::Project), &error.to_string());
                None
            }
        }
    }
    fn advance(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        update: impl FnOnce(&Stage) -> Result<Option<Stage>>,
    ) {
        let Some(observation) = observation else {
            return;
        };
        let Ok(mut state) = self.0.lock() else {
            return;
        };
        if !state.active(observation, elapsed) {
            return;
        }
        let Some(Data::Project {
            phase: Phase::Running { stage },
            ..
        }) = &state.original
        else {
            return;
        };
        match update(stage) {
            Ok(Some(next)) => {
                if let Some(Data::Project {
                    elapsed_ms, phase, ..
                }) = &mut state.original
                {
                    *elapsed_ms = elapsed;
                    *phase = Phase::Running { stage: next };
                }
                state.publish();
            }
            Ok(None) => {}
            Err(error) => state.unavailable(&error.to_string()),
        }
    }
    pub(crate) fn recording_proposal(&self, observation: Option<&Observation>, elapsed: u32) {
        self.advance(observation, elapsed, |stage| {
            Ok(matches!(stage, Stage::CheckingProposal).then_some(Stage::RecordingProposal))
        });
    }
    pub(crate) fn source(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        source_sha256: String,
    ) {
        self.advance(observation, elapsed, |phase| {
            if !matches!(phase, Stage::ReadingSource) {
                return Ok(None);
            }
            anyhow::ensure!(digest(&source_sha256), "Invalid captured source identity");
            Ok(Some(Stage::PreparingChecker { source_sha256 }))
        });
    }
    pub(crate) fn checking(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        prepared: &meden_runtime::verification::artifact::Inputs,
    ) {
        self.advance(observation, elapsed, |phase| {
            let Stage::PreparingChecker { source_sha256 } = phase else {
                return Ok(None);
            };
            anyhow::ensure!(
                prepared.source_sha256 == *source_sha256,
                "Prepared verifier source differs from the original capture"
            );
            let prepared_sha256 = format!("{:x}", Sha256::digest(serde_json::to_vec(prepared)?));
            Ok(Some(Stage::Checking {
                inputs: Inputs {
                    source_sha256: source_sha256.clone(),
                    prepared_sha256,
                },
            }))
        });
    }
    pub(crate) fn refreshing(&self, observation: Option<&Observation>, elapsed: u32) {
        self.advance(observation, elapsed, |phase| {
            Ok(match phase {
                Stage::Checking { inputs } => Some(Stage::RefreshingSource {
                    inputs: inputs.clone(),
                }),
                _ => None,
            })
        });
    }
    pub(crate) fn retaining(&self, observation: Option<&Observation>, elapsed: u32) {
        self.advance(observation, elapsed, |phase| {
            Ok(match phase {
                Stage::RefreshingSource { inputs } => Some(Stage::RetainingEvidence {
                    inputs: inputs.clone(),
                }),
                _ => None,
            })
        });
    }
    pub(crate) fn failed_observation(&self, observation: Option<&Observation>, message: &str) {
        let Some(observation) = observation else {
            return;
        };
        if let Ok(mut state) = self.0.lock() {
            if state.active(observation, u32::MAX) {
                state.unavailable(message);
            }
        }
    }
    pub(crate) fn finish(
        &self,
        observation: Option<Observation>,
        elapsed: u32,
        outcome: Completion<'_>,
    ) {
        let Some(observation) = observation else {
            return;
        };
        let Ok(mut state) = self.0.lock() else {
            return;
        };
        if !state.active(&observation, elapsed) {
            return;
        }
        if matches!(state.original, Some(Data::Recorded { .. })) {
            recorded::finish(&mut state, elapsed, outcome);
            return;
        }
        let result = (|| -> Result<Phase> {
            let Some(Data::Project { context, phase, .. }) = &state.original else {
                anyhow::bail!("Verification observation missing");
            };
            let during = phase
                .stage()
                .context("Verification observation is already terminal")?;
            Ok(match outcome {
                Completion::Proposed(summary) => checked_proposal(context, during, summary)?,
                Completion::Admitted(summary) => {
                    let Subject::Accepted { reference } = &context.subject else {
                        anyhow::bail!("Evidence completion has no original accepted subject");
                    };
                    let Stage::RetainingEvidence { inputs } = during else {
                        anyhow::bail!("Verification completion preceded retention");
                    };
                    anyhow::ensure!(
                        summary.accepted
                            && summary.verified
                            && summary.accepted_reference == summary.reference
                            && summary.reference.as_ref().is_some_and(|original| {
                                super::evidence::Reference::new(original) == *reference
                            })
                            && summary.evidence_class == Some(context.class)
                            && summary.scope == context.scope
                            && summary.source_path == context.source_path
                            && summary.source_sha256.as_deref()
                                == Some(inputs.source_sha256.as_str())
                            && summary.artifact.is_some()
                            && summary.artifact_sha256.as_deref().is_some_and(digest)
                            && summary.verification_failure.is_none()
                            && summary.verification_diagnostic.is_none()
                            && summary.completion_label
                                == crate::verification::completed_label(context.class),
                        "Admitted verification context changed"
                    );
                    Phase::Admitted {
                        inputs: inputs.clone(),
                        completion_label: summary.completion_label.into(),
                    }
                }
                Completion::Recorded(_) => {
                    anyhow::bail!("Historical completion cannot admit project evidence")
                }
                Completion::Failed(kind, message) => Phase::Failed {
                    during: during.clone(),
                    kind,
                    message: diagnostic(message),
                },
                Completion::Cancelled(message) => Phase::Cancelled {
                    during: during.clone(),
                    message: diagnostic(message),
                },
            })
        })();
        match result {
            Ok(next) => {
                if let Some(Data::Project {
                    elapsed_ms, phase, ..
                }) = &mut state.original
                {
                    *elapsed_ms = elapsed;
                    *phase = next;
                }
                state.publish();
            }
            Err(error) => state.unavailable(&error.to_string()),
        }
    }
}
fn checked_proposal(
    context: &Context,
    during: &Stage,
    summary: &project::Summary,
) -> Result<Phase> {
    let Subject::Proposal { session, contract } = &context.subject else {
        anyhow::bail!("Proposal completion has no original draft");
    };
    let reference = summary
        .reference
        .as_ref()
        .context("Checked proposal has no reference")?;
    anyhow::ensure!(
        matches!(during, Stage::RecordingProposal)
            && summary.proposed
            && !summary.accepted
            && !summary.verified
            && reference.session == *session
            && reference.contract == *contract
            && summary.contract.as_ref().is_some_and(|original| original
                .identity()
                .is_ok_and(|identity| identity == contract.as_str())
                && original.backend() == context.backend
                && original.description().name == context.name)
            && summary.evidence_class == Some(context.class)
            && summary.scope == context.scope
            && summary.source_path == context.source_path
            && summary.artifact.is_none()
            && summary.source_sha256.is_none()
            && summary.artifact_sha256.is_none()
            && summary.verification_failure.is_none()
            && summary.verification_diagnostic.is_none(),
        "Checked proposal context changed"
    );
    Ok(Phase::Proposed {
        reference: super::evidence::Reference::new(reference),
    })
}
#[derive(Clone, Copy)]
pub(crate) enum Completion<'a> {
    Recorded(&'a crate::records::store::Observation),
    Proposed(&'a project::Summary),
    Admitted(&'a project::Summary),
    Failed(FailureKind, &'a str),
    Cancelled(&'a str),
}
/// Read-only transport handle; it cannot manufacture or advance observations.
#[derive(Clone)]
pub struct Reader(Arc<Mutex<State>>);
impl Reader {
    /// Exact already-frozen bytes, independent of project/checker execution.
    pub fn bytes(&self, revision: &ArtifactId<Artifact>) -> Option<Arc<[u8]>> {
        self.0.lock().ok()?.publication.as_ref()?.bytes(revision)
    }
}
impl Serialize for Reader {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self.0.lock() {
            Ok(state) => state.status.serialize(serializer),
            Err(_) => Status::Unavailable {
                role: None,
                message: "Verification observation is unavailable".into(),
            }
            .serialize(serializer),
        }
    }
}

/// Identity context for this owner's immutable publications, never authority.
pub enum Artifact {}
/// Original project/recorded check observation context; never a kernel ticket.
pub enum Job {}
impl super::publication::Domain for Artifact {
    const PREFIX: &'static [u8] = b"meden.verification.observation.v1\0";
}
impl super::publication::Data for Data {
    type Domain = Artifact;
}

#[cfg(test)]
mod tests;
