//! Historical work observations in the existing latest-job publication owner.
use super::{diagnostic, Completion, Data, Job, JobId, Observation, Publisher, State};
use crate::records::{lake::Outcome, store, Check, Work};
use anyhow::{Context as _, Result};
use meden_runtime::verification::FailureKind;
use schemars::JsonSchema;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Host,
    Model,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub(super) origin: Origin,
    pub(super) job: JobId<Job>,
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    session: String,
    #[schemars(length(min = 1, max = 1000))]
    id: String,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    work: Work,
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    engine_sha256: String,
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    source_revision: String,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Stage {
    ReadingRecord,
    Preparing { work: Work },
    Capturing { work: Work, engine_sha256: String },
    RecordingStart { inputs: Inputs },
    Building { inputs: Inputs },
    Inspecting { inputs: Inputs },
    Refreshing { inputs: Inputs },
    Persisting { inputs: Inputs },
}
impl Stage {
    fn inputs(&self) -> Option<&Inputs> {
        match self {
            Self::RecordingStart { inputs }
            | Self::Building { inputs }
            | Self::Inspecting { inputs }
            | Self::Refreshing { inputs }
            | Self::Persisting { inputs } => Some(inputs),
            _ => None,
        }
    }
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Checked,
    Rejected,
    Cancelled,
    Inconclusive,
    Unavailable,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Phase {
    Running {
        stage: Stage,
    },
    /// Published only after the original history write succeeded. It cannot
    /// accept a contract, admit evidence or restore any native verification.
    Historical {
        inputs: Inputs,
        #[schemars(regex(pattern = r"^event_[a-f0-9]{64}$"))]
        event: String,
        #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
        record_sha256: String,
        result: ResultKind,
    },
    Failed {
        during: Stage,
        kind: FailureKind,
        #[schemars(length(max = 8000))]
        message: String,
    },
    Cancelled {
        during: Stage,
        #[schemars(length(max = 8000))]
        message: String,
    },
}
impl Phase {
    pub(super) fn stage(&self) -> Option<&Stage> {
        if let Self::Running { stage } = self {
            Some(stage)
        } else {
            None
        }
    }
}
fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(super::digest)
}
fn original(inputs: &Inputs, record: &Check) -> bool {
    record.engine_sha256 == inputs.engine_sha256
        && record.source_revision == inputs.source_revision
        && record.module == inputs.work.module
        && record.theorems == inputs.work.theorems
        && record.requirement == inputs.work.requirement
}
impl Publisher {
    pub(crate) fn begin_recorded(
        &self,
        session: &[u8],
        id: &str,
        origin: Origin,
    ) -> Option<Observation> {
        let mut state = self.0.lock().ok()?;
        let result = (|| -> Result<(u64, Context)> {
            anyhow::ensure!(crate::records::valid_name(id), "Invalid recorded work id");
            let sequence = state
                .sequence
                .checked_add(1)
                .context("Check observation sequence exhausted")?;
            let mut hash = Sha256::new();
            hash.update(b"meden.recorded.check.v1\0");
            hash.update(state.epoch);
            hash.update(sequence.to_le_bytes());
            hash.update(serde_json::to_vec(&(session, id, origin))?);
            Ok((
                sequence,
                Context {
                    origin,
                    job: JobId::from_digest(hash.finalize()),
                    session: format!("{:x}", Sha256::digest(session)),
                    id: id.into(),
                },
            ))
        })();
        match result {
            Ok((sequence, context)) => {
                let observation = Observation {
                    job: context.job.clone(),
                };
                state.sequence = sequence;
                state.original = Some(Data::Recorded {
                    context,
                    elapsed_ms: 0,
                    phase: Phase::Running {
                        stage: Stage::ReadingRecord,
                    },
                });
                state.publish();
                Some(observation)
            }
            Err(error) => {
                state.unavailable_for(
                    Some(match origin {
                        Origin::Host => super::Role::RecordedHost,
                        Origin::Model => super::Role::RecordedModel,
                    }),
                    &error.to_string(),
                );
                None
            }
        }
    }
    fn recorded_advance(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        update: impl FnOnce(&Context, &Stage) -> Result<Option<Stage>>,
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
        let Some(Data::Recorded {
            context,
            phase: Phase::Running { stage },
            ..
        }) = &state.original
        else {
            return;
        };
        match update(context, stage) {
            Ok(Some(next)) => {
                if let Some(Data::Recorded {
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
    pub(crate) fn recorded_work(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        work: &Work,
    ) {
        self.recorded_advance(observation, elapsed, |context, boundary| {
            if !matches!(boundary, Stage::ReadingRecord) {
                return Ok(None);
            }
            anyhow::ensure!(context.id == work.id, "Recorded work changed after request");
            Ok(Some(Stage::Preparing { work: work.clone() }))
        });
    }
    pub(crate) fn recorded_engine(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        engine: &str,
    ) {
        self.recorded_advance(observation, elapsed, |_, boundary| {
            let Stage::Preparing { work } = boundary else {
                return Ok(None);
            };
            anyhow::ensure!(digest(engine), "Invalid recorded engine identity");
            Ok(Some(Stage::Capturing {
                work: work.clone(),
                engine_sha256: engine.into(),
            }))
        });
    }
    pub(crate) fn recorded_source(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        source: &str,
    ) {
        self.recorded_advance(observation, elapsed, |_, boundary| {
            let Stage::Capturing {
                work,
                engine_sha256,
            } = boundary
            else {
                return Ok(None);
            };
            anyhow::ensure!(digest(source), "Invalid recorded source identity");
            Ok(Some(Stage::RecordingStart {
                inputs: Inputs {
                    work: work.clone(),
                    engine_sha256: engine_sha256.clone(),
                    source_revision: source.into(),
                },
            }))
        });
    }
    pub(crate) fn recorded_step(
        &self,
        observation: Option<&Observation>,
        elapsed: u32,
        step: Step,
    ) {
        self.recorded_advance(observation, elapsed, |_, boundary| {
            let Some(inputs) = boundary.inputs() else {
                return Ok(None);
            };
            Ok(match (boundary, step) {
                (Stage::RecordingStart { .. }, Step::Building) => Some(Stage::Building {
                    inputs: inputs.clone(),
                }),
                (Stage::Building { .. }, Step::Inspecting) => Some(Stage::Inspecting {
                    inputs: inputs.clone(),
                }),
                (Stage::Building { .. } | Stage::Inspecting { .. }, Step::Refreshing) => {
                    Some(Stage::Refreshing {
                        inputs: inputs.clone(),
                    })
                }
                (Stage::Refreshing { .. }, Step::Persisting) => Some(Stage::Persisting {
                    inputs: inputs.clone(),
                }),
                _ => None,
            })
        });
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Step {
    Building,
    Inspecting,
    Refreshing,
    Persisting,
}
fn complete(context: &Context, phase: &Phase, outcome: Completion<'_>) -> Result<Phase> {
    let during = phase
        .stage()
        .context("Recorded check is already terminal")?;
    Ok(match outcome {
        Completion::Recorded(store::Observation {
            record,
            outcome,
            event,
        }) => {
            let Stage::Persisting { inputs } = during else {
                anyhow::bail!("Historical completion preceded persistence");
            };
            anyhow::ensure!(
                context.id == inputs.work.id
                    && original(inputs, record)
                    && event.strip_prefix("event_").is_some_and(super::digest)
                    && (!matches!(outcome, Outcome::Completed | Outcome::Inconclusive)
                        || record.after_revision.as_deref()
                            == Some(inputs.source_revision.as_str())),
                "Original recorded-check context changed"
            );
            let result = match (*outcome, record.success) {
                (Outcome::Completed, true) => ResultKind::Checked,
                (Outcome::Completed, false) => ResultKind::Rejected,
                (Outcome::Cancelled, false) => ResultKind::Cancelled,
                (Outcome::Inconclusive, false) => ResultKind::Inconclusive,
                (Outcome::Unavailable, false) => ResultKind::Unavailable,
                _ => anyhow::bail!("Historical success disagrees with host outcome"),
            };
            Phase::Historical {
                inputs: inputs.clone(),
                event: event.clone(),
                record_sha256: format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(record)?)),
                result,
            }
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
        Completion::Proposed(_) | Completion::Admitted(_) => {
            anyhow::bail!("Project completion cannot replace a recorded check")
        }
    })
}
pub(super) fn finish(state: &mut State, elapsed: u32, outcome: Completion<'_>) {
    let Some(Data::Recorded { context, phase, .. }) = &state.original else {
        return;
    };
    match complete(context, phase, outcome) {
        Ok(next) => {
            if let Some(Data::Recorded {
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

#[cfg(test)]
mod tests;
