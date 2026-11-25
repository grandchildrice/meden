//! Revision-bound candidate jobs. These observations carry no proof or authority.
use super::publication::{ArtifactId, Envelope, Publication};
use crate::{analysis, semantic};
use anyhow::Result;
use schemars::JsonSchema;
use serde::Serialize;
use std::sync::Arc;

/// Original records remain available to host operations; journal format is unchanged.
pub struct Records {
    /// Selected observation, including a running or explicitly stale candidate.
    pub current: Option<analysis::Record>,
    /// Complete retained history, never a receipt or a restored approval.
    pub history: Vec<analysis::Record>,
}

/// Lossless presentation of an original record. Borrowing avoids cloning all history.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Record<'a> {
    /// Original journal version, independently validated before publication.
    #[schemars(range(min = 2, max = 2))]
    version: u8,
    /// Canonical positive u64 decimal; browsers must not round an original sequence.
    #[schemars(regex(pattern = r"^[1-9][0-9]{0,19}$"))]
    sequence: String,
    #[schemars(regex(pattern = r"^analysis-[a-f0-9]{64}$"))]
    id: &'a str,
    #[schemars(regex(pattern = r"^requirement-[a-f0-9]{64}$"))]
    requirement_id: &'a str,
    #[schemars(regex(pattern = r"^finding-[a-f0-9]{64}$"))]
    finding_id: Option<&'a str>,
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    source_artifact: Option<&'a str>,
    /// Original assessment method, independent of a compiler or verifier role.
    engine: &'a str,
    model: &'a str,
    processor: &'a str,
    judge: Option<&'a crate::model::Binding>,
    /// Retained method diagnostics; they cannot instantiate a model or grant.
    #[serde(skip_serializing_if = "Option::is_none")]
    model_definition: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    judge_definition: Option<&'a serde_json::Value>,
    input: &'a semantic::Input,
    state: analysis::State,
    assessment: Option<&'a semantic::Assessment>,
    partial_translation: Option<&'a semantic::Translation>,
    error: Option<&'a str>,
}
impl<'a> Record<'a> {
    fn new(record: &'a analysis::Record) -> Result<Self> {
        record.input.validate()?;
        anyhow::ensure!(
            record.version == 2
                && record.sequence > 0
                && hex(&record.id, "analysis-")
                && hex(&record.requirement_id, "requirement-")
                && record
                    .finding_id
                    .as_deref()
                    .is_none_or(|s| hex(s, "finding-"))
                && record
                    .source_artifact
                    .as_deref()
                    .is_none_or(|s| hex(s, "sha256:"))
                && record.input.target.line <= 1_048_577,
            "Invalid candidate observation identity or input"
        );
        for translation in record
            .assessment
            .as_ref()
            .map(|a| &a.translation)
            .into_iter()
            .chain(record.partial_translation.iter())
        {
            anyhow::ensure!(
                (0.0..=1.0).contains(&translation.confidence),
                "Invalid candidate confidence"
            );
        }
        Ok(Self {
            version: record.version,
            sequence: record.sequence.to_string(),
            id: &record.id,
            requirement_id: &record.requirement_id,
            finding_id: record.finding_id.as_deref(),
            source_artifact: record.source_artifact.as_deref(),
            engine: &record.engine,
            model: &record.model,
            processor: &record.processor,
            judge: record.judge.as_ref(),
            model_definition: record.model_definition.as_ref(),
            judge_definition: record.judge_definition.as_ref(),
            input: &record.input,
            state: record.state,
            assessment: record.assessment.as_ref(),
            partial_translation: record.partial_translation.as_ref(),
            error: record.error.as_deref(),
        })
    }
}
fn hex(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Complete retained history, independent of the currently selected/running job.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct History<'a> {
    #[schemars(length(max = 1000))]
    history: Vec<Record<'a>>,
}
/// One exact selected/running observation.
pub type CurrentPublication<'a> = Envelope<Record<'a>>;
/// One exact complete retained history.
pub type HistoryPublication<'a> = Envelope<History<'a>>;

/// A selected candidate is absent, available, or explicitly unavailable.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Current {
    /// No job is selected; no successful assessment is invented.
    Empty,
    /// The exact complete observation is retained.
    Ready {
        /// Immutable body identity, never authority.
        revision: ArtifactId<CurrentArtifact>,
    },
    /// Original data is retained, but cannot be completely projected.
    Unavailable {
        /// Bounded host diagnostic.
        #[schemars(length(max = 8000))]
        message: String,
    },
}
/// Complete history availability, separate from current job progress.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum HistoryStatus {
    /// Complete original history is available by exact identity.
    Ready {
        /// Immutable body identity.
        revision: ArtifactId<HistoryArtifact>,
        /// Complete retained run count.
        #[schemars(range(max = 1000))]
        runs: u32,
    },
    /// No partial history is published and old bodies are retired.
    Unavailable {
        /// Bounded host diagnostic.
        #[schemars(length(max = 8000))]
        message: String,
    },
}
/// Atomic current/history references; neither carries proof or execution authority.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Status {
    current: Current,
    history: HistoryStatus,
}
/// Sole owner. Current updates never serialize or hash unchanged history.
pub struct Analyses {
    records: Records,
    status: Status,
    current: Option<Publication<CurrentArtifact>>,
    history: Option<Publication<HistoryArtifact>>,
}
impl Analyses {
    /// An unreadable journal has no invented empty successful history.
    pub fn unavailable(message: &str) -> Self {
        Self {
            records: Records {
                current: None,
                history: Vec::new(),
            },
            status: Status {
                current: Current::Empty,
                history: HistoryStatus::Unavailable {
                    message: diagnostic(message),
                },
            },
            current: None,
            history: None,
        }
    }
    /// Freeze complete history and its selected observation, without restoring authority.
    pub fn new(records: Records) -> Self {
        let mut value = Self::unavailable("Candidate observations are not prepared");
        value.records = records;
        value.refresh_history();
        value.refresh_current();
        value
    }
    /// Host read-only access to original records.
    pub fn data(&self) -> &Records {
        &self.records
    }
    /// Advance only the selected/running observation; history retains its original bytes.
    pub fn select(&mut self, record: Option<analysis::Record>) {
        self.records.current = record;
        self.refresh_current();
    }
    /// Replace complete original history only after the host observes that change.
    pub fn replace_history(&mut self, records: Vec<analysis::Record>) {
        self.records.history = records;
        self.refresh_history();
    }
    /// Source changes retire a current candidate without rewriting historical inputs.
    pub fn invalidate(&mut self) {
        if let Some(record) = self
            .records
            .current
            .as_mut()
            .filter(|r| r.state == analysis::State::Complete)
        {
            record.state = analysis::State::Stale;
            self.refresh_current();
        }
    }
    /// Exact current bytes only; no I/O or rematerialization.
    pub fn current_bytes(&self, revision: &ArtifactId<CurrentArtifact>) -> Option<Arc<[u8]>> {
        self.current.as_ref()?.bytes(revision)
    }
    /// Exact history bytes only; selected/progress changes do not retire them.
    pub fn history_bytes(&self, revision: &ArtifactId<HistoryArtifact>) -> Option<Arc<[u8]>> {
        self.history.as_ref()?.bytes(revision)
    }
    fn prepare_current(&self, record: &analysis::Record) -> Result<Publication<CurrentArtifact>> {
        let _timing = meden_runtime::metrics::Span::start("view.analysis.materialize");
        let data = Record::new(record)?;
        freeze(self.current.as_ref(), self.history.as_ref(), &data)
    }
    fn prepare_history(&self) -> Result<(Publication<HistoryArtifact>, u32)> {
        let _timing = meden_runtime::metrics::Span::start("view.analysis_history.materialize");
        anyhow::ensure!(
            self.records.history.len() <= 1000,
            "Candidate history exceeds 1000 runs; no partial publication"
        );
        let data = History {
            history: self
                .records
                .history
                .iter()
                .map(Record::new)
                .collect::<Result<_>>()?,
        };
        let publication = freeze(self.history.as_ref(), self.current.as_ref(), &data)?;
        Ok((publication, u32::try_from(self.records.history.len())?))
    }
    fn refresh_current(&mut self) {
        let Some(record) = self.records.current.as_ref() else {
            self.current = None;
            self.status.current = Current::Empty;
            return;
        };
        match self.prepare_current(record) {
            Ok(publication) => {
                self.status.current = Current::Ready {
                    revision: publication.reference().revision.clone(),
                };
                self.current = Some(publication);
            }
            Err(error) => {
                self.current = None;
                self.status.current = Current::Unavailable {
                    message: diagnostic(&format!("Candidate observation unavailable: {error:#}")),
                };
            }
        }
    }
    fn refresh_history(&mut self) {
        match self.prepare_history() {
            Ok((publication, runs)) => {
                self.status.history = HistoryStatus::Ready {
                    revision: publication.reference().revision.clone(),
                    runs,
                };
                self.history = Some(publication);
            }
            Err(error) => {
                self.history = None;
                self.status.history = HistoryStatus::Unavailable {
                    message: diagnostic(&format!("Candidate history unavailable: {error:#}")),
                };
            }
        }
    }
}
fn freeze<D: super::publication::Domain, O: super::publication::Domain>(
    previous: Option<&Publication<D>>,
    other: Option<&Publication<O>>,
    data: &impl super::publication::Data<Domain = D>,
) -> Result<Publication<D>> {
    let publication = match previous {
        Some(previous) => previous.update(data),
        None => Publication::new(data),
    }?;
    anyhow::ensure!(
        publication
            .size()
            .saturating_add(other.map_or(0, Publication::size))
            <= super::publication::MAX_BYTES,
        "Candidate publications exceed 64 MiB; no partial observation"
    );
    Ok(publication)
}
fn diagnostic(message: &str) -> String {
    super::publication::diagnostic(message, "\n… (diagnostic shortened)")
}
impl Serialize for Analyses {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.status.serialize(serializer)
    }
}
/// Identity context of the selected/running candidate observation.
pub enum CurrentArtifact {}
impl super::publication::Domain for CurrentArtifact {
    const PREFIX: &'static [u8] = b"meden.candidate-current.v1\0";
}
impl super::publication::Data for Record<'_> {
    type Domain = CurrentArtifact;
}
/// Identity context of complete candidate history, separate from the selection.
pub enum HistoryArtifact {}
impl super::publication::Domain for HistoryArtifact {
    const PREFIX: &'static [u8] = b"meden.candidate-history.v1\0";
}
impl super::publication::Data for History<'_> {
    type Domain = HistoryArtifact;
}

#[cfg(test)]
mod tests;
