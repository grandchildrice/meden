//! Bounded resource-local progress and immutable diagnostic queries.
//! These are observations, never kernel job tickets or evidence receipts.
use super::{
    job_id::JobId,
    publication::{ArtifactId, ArtifactRef, Publication},
};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{ser::SerializeMap, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

const LOG_BYTES: usize = 65_536;
const JOBS: usize = 64;

/// Host-observed stop class; diagnostics never select it.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// Completed preparation operation failed.
    Operation,
    /// Actual enclosing host control observed cancellation.
    Cancelled,
    /// The progress adapter supplied an invalid observation.
    Observation,
}
/// Internal terminal observation, separately attributed by its host.
pub struct Failure {
    /// Typed original stop cause.
    pub kind: FailureKind,
    /// Local diagnostic.
    pub message: String,
}
impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self {
            kind: FailureKind::Operation,
            message,
        }
    }
}
/// Disjoint display outcomes; only successful completion has a cache observation.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    /// The host is still preparing this resource.
    Preparing,
    /// Preparation completed; this does not imply admitted evidence.
    Ready {
        /// All required preparation stages reused their original artifacts.
        cache_hit: bool,
    },
    /// Host-observed failure, independent of diagnostic wording.
    Failed {
        /// Typed host stop observation, independent of text.
        kind: FailureKind,
        /// At most 8000 UTF-8 bytes, explicitly marked when shortened.
        #[schemars(length(max = 8000))]
        error: String,
    },
}
/// Small generated status; no log body is repeated by polling.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Status {
    /// Host-session/resource/sequence-bound observation identity.
    pub job: JobId<Job>,
    /// Exact original host-selected resource, not an executable request.
    #[schemars(length(min = 1, max = 4096))]
    pub resource: String,
    /// Original adapter step label, at most 512 UTF-8 bytes.
    #[schemars(length(min = 1, max = 512))]
    pub phase: String,
    /// Host-observed duration, saturating after about 49 days.
    pub elapsed_ms: u32,
    /// Duration within the current step.
    pub phase_elapsed_ms: u32,
    /// Applicable host-selected timeout, zero when none is reported.
    pub limit_seconds: u32,
    /// Terminal outcomes cannot return to preparation.
    pub outcome: Outcome,
    /// Exact current bounded diagnostic body, if one has been emitted.
    pub log: Option<ArtifactRef<Artifact>>,
}
/// One latest resource-local status, bounded to 64 entries by its owner.
pub type Index = BTreeMap<String, Status>;
/// Bounded latest output, explicitly marked when earlier text was dropped.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Log {
    /// Original preparation identity, independent of publication identity.
    pub job: JobId<Job>,
    /// Original resource; the viewer checks this as well as the job.
    #[schemars(length(min = 1, max = 4096))]
    pub resource: String,
    /// Literal host-normalized diagnostic tail, not source or evidence.
    #[schemars(length(max = 65_536))]
    pub text: String,
    /// Some earlier output was not retained, including worker truncation.
    pub truncated: bool,
}
/// Non-deserializable handle: late observers cannot update a replacement job.
#[derive(Clone)]
pub struct Observation {
    job: JobId<Job>,
    resource: String,
}
struct Entry {
    status: Status,
    phase_started: u32,
    prefix: String,
    text: String,
    truncated: bool,
    publication: Option<Publication<Artifact>>,
}
/// One bounded owner. Clocks, output normalization and execution stay in hosts.
pub struct Jobs {
    epoch: [u8; 32],
    sequence: u64,
    entries: BTreeMap<String, Entry>,
    order: VecDeque<String>,
}
impl Jobs {
    /// Bind display observations to a fresh host session without publishing it.
    pub fn new(session: &[u8]) -> Self {
        Self {
            epoch: Sha256::digest(session).into(),
            sequence: 0,
            entries: BTreeMap::new(),
            order: VecDeque::new(),
        }
    }
    /// Begin a new observation and retire any prior publication for its resource.
    pub fn begin(&mut self, resource: &str, phase: &str) -> Result<Observation> {
        anyhow::ensure!(
            !resource.is_empty() && resource.len() <= 4096 && !resource.contains('\0'),
            "Invalid preparation resource"
        );
        anyhow::ensure!(
            !phase.is_empty() && phase.len() <= 512,
            "Invalid preparation step"
        );
        let next = self
            .sequence
            .checked_add(1)
            .context("Preparation sequence exhausted")?;
        let mut hash = Sha256::new();
        hash.update(b"meden.preparation.job.v1\0");
        hash.update(self.epoch);
        hash.update(next.to_le_bytes());
        hash.update(resource.as_bytes());
        let job = JobId::from_digest(hash.finalize());
        self.sequence = next;
        self.remove(resource);
        self.order.push_back(resource.into());
        self.entries.insert(
            resource.into(),
            Entry {
                status: Status {
                    job: job.clone(),
                    resource: resource.into(),
                    phase: phase.into(),
                    elapsed_ms: 0,
                    phase_elapsed_ms: 0,
                    limit_seconds: 0,
                    outcome: Outcome::Preparing,
                    log: None,
                },
                phase_started: 0,
                prefix: String::new(),
                text: String::new(),
                truncated: false,
                publication: None,
            },
        );
        while self.entries.len() > JOBS {
            let oldest = self
                .order
                .front()
                .cloned()
                .context("Preparation order missing")?;
            self.remove(&oldest);
        }
        Ok(Observation {
            job,
            resource: resource.into(),
        })
    }
    fn active(&mut self, original: &Observation, elapsed: u32) -> Option<&mut Entry> {
        self.entries.get_mut(&original.resource).filter(|entry| {
            entry.status.job == original.job
                && matches!(entry.status.outcome, Outcome::Preparing)
                && elapsed >= entry.status.elapsed_ms
        })
    }
    /// Observe a phase start or cumulative output snapshot from its original host.
    /// Stale, completed or backwards-time observations have no effect.
    pub fn update(
        &mut self,
        original: &Observation,
        elapsed: u32,
        phase: &str,
        limit_seconds: u32,
        output: Option<(&str, bool)>,
    ) -> Result<()> {
        let Some(entry) = self.active(original, elapsed) else {
            return Ok(());
        };
        anyhow::ensure!(
            !phase.is_empty() && phase.len() <= 512,
            "Invalid preparation step"
        );
        if output.is_none() {
            let header = format!("\n[{phase}]\n");
            let (prefix, truncated) = tail(&[&entry.text, &header]);
            entry.prefix = prefix;
            entry.truncated |= truncated;
            entry.status.phase = phase.into();
            entry.phase_started = elapsed;
            entry.status.limit_seconds = limit_seconds;
        } else {
            anyhow::ensure!(
                entry.status.phase == phase,
                "Preparation output has a different step"
            );
        }
        let (output, truncated) = output.unwrap_or(("", false));
        let (text, clipped) = tail(&[&entry.prefix, output]);
        let truncated = entry.truncated || truncated || clipped;
        let log = Log {
            job: original.job.clone(),
            resource: original.resource.clone(),
            text: text.clone(),
            truncated,
        };
        // Serialize only at a changed-output boundary, never on status reads.
        if entry.text != text || entry.truncated != truncated || entry.publication.is_none() {
            let _timing = meden_runtime::metrics::Span::start("view.preparation_log.materialize");
            let publication = Publication::new(&log)?;
            entry.status.log = Some(publication.reference().clone());
            entry.publication = Some(publication);
            entry.text = text;
            entry.truncated = truncated;
        }
        entry.status.elapsed_ms = elapsed;
        entry.status.phase_elapsed_ms = elapsed.saturating_sub(entry.phase_started);
        Ok(())
    }
    /// Terminal host observation. It cannot revive a superseded preparation.
    pub fn finish(&mut self, original: &Observation, elapsed: u32, result: Result<bool, Failure>) {
        let Some(entry) = self.active(original, elapsed) else {
            return;
        };
        entry.status.elapsed_ms = elapsed;
        entry.status.phase_elapsed_ms = elapsed.saturating_sub(entry.phase_started);
        entry.status.outcome = match result {
            Ok(cache_hit) => Outcome::Ready { cache_hit },
            Err(error) => Outcome::Failed {
                kind: error.kind,
                error: diagnostic(&error.message),
            },
        };
    }
    /// Exact frozen bytes only; a superseded resource's log is unavailable.
    pub fn bytes(&self, revision: &ArtifactId<Artifact>) -> Option<Arc<[u8]>> {
        self.entries
            .values()
            .find_map(|entry| entry.publication.as_ref()?.bytes(revision))
    }
    /// Explicit invalidation retires this resource and its observer handle.
    pub fn remove(&mut self, resource: &str) {
        self.entries.remove(resource);
        self.order.retain(|old| old != resource);
    }
    /// Refresh retires all observations without renewing sequence identities.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}
impl Serialize for Jobs {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (key, entry) in &self.entries {
            map.serialize_entry(key, &entry.status)?;
        }
        map.end()
    }
}
/// Byte bounds also fit the browser's UTF-16 validation limits.
fn diagnostic(message: &str) -> String {
    const MARKER: &str = "\n… (diagnostic shortened; see build log)";
    super::publication::diagnostic(message, MARKER)
}

/// Keep the exact UTF-8 suffix within the byte bound without joining full output.
fn tail(parts: &[&str]) -> (String, bool) {
    let total = parts
        .iter()
        .fold(0usize, |n, part| n.saturating_add(part.len()));
    let mut skip = total.saturating_sub(LOG_BYTES);
    let mut result = String::with_capacity(total.min(LOG_BYTES));
    for part in parts {
        if skip >= part.len() {
            skip = skip.saturating_sub(part.len());
            continue;
        }
        while !part.is_char_boundary(skip) {
            skip = skip.saturating_add(1);
        }
        result.push_str(&part[skip..]);
        skip = 0;
    }
    (result, total > LOG_BYTES)
}

/// Identity context for this owner's immutable publications, never authority.
pub enum Artifact {}
/// Resource preparation observation context, distinct from verification checks.
pub enum Job {}
impl super::publication::Domain for Artifact {
    const PREFIX: &'static [u8] = b"meden.preparation.log.v1\0";
}
impl super::publication::Data for Log {
    type Domain = Artifact;
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn bounded_tail_matches_full_original_concatenation(
            parts in prop::collection::vec(".{0,2000}", 0..20),
            repetitions in 1usize..8,
        ) {
            let expanded = parts.iter().map(|part| part.repeat(repetitions)).collect::<Vec<_>>();
            let original = expanded.concat();
            let mut start = original.len().saturating_sub(LOG_BYTES);
            while !original.is_char_boundary(start) { start = start.saturating_add(1); }
            let refs = expanded.iter().map(String::as_str).collect::<Vec<_>>();
            let (actual, truncated) = tail(&refs);
            prop_assert_eq!(Some(actual.as_str()), original.get(start..));
            prop_assert_eq!(truncated, original.len() > LOG_BYTES);
        }
    }
    fn status(jobs: &Jobs, path: &str) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(jobs)?
            .get(path)
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }
    #[test]
    fn frozen_logs_keep_original_binding_and_do_not_enter_status() -> Result<()> {
        let mut jobs = Jobs::new(b"session");
        let original = jobs.begin("src/A", "Compile")?;
        jobs.update(&original, 2, "Compile", 5, None)?;
        let text = format!("original\n{}\"\\", "λ日本語".repeat(9000));
        jobs.update(&original, 8, "Compile", 5, Some((&text, false)))?;
        let s = status(&jobs, "src/A")?;
        let id = s
            .pointer("/log/revision")
            .context("field")?
            .as_str()
            .context("log")?;
        let bytes = jobs.bytes(&ArtifactId::parse(id)?).context("body")?;
        let log: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            log.pointer("/data/job").context("field")?,
            s.pointer("/job").context("field")?
        );
        assert_eq!(log.pointer("/data/resource").context("field")?, "src/A");
        let tail = log
            .pointer("/data/text")
            .context("field")?
            .as_str()
            .context("text")?;
        assert!(tail.len() <= LOG_BYTES && text.ends_with(tail));
        assert_eq!(log.pointer("/data/truncated").context("field")?, true);
        assert!(serde_json::to_vec(&jobs)?.len() < 1000);
        jobs.update(&original, 9, "Compile", 5, Some((&text, false)))?;
        assert!(Arc::ptr_eq(
            &bytes,
            &jobs.bytes(&ArtifactId::parse(id)?).context("same")?
        ));
        jobs.finish(&original, 10, Ok(true));
        jobs.update(&original, 11, "Compile", 5, Some(("late", false)))?;
        jobs.finish(&original, 12, Err("late failure".to_owned().into()));
        assert_eq!(
            status(&jobs, "src/A")?
                .pointer("/outcome")
                .context("field")?,
            &serde_json::json!({"state":"ready", "cache_hit":true})
        );
        Ok(())
    }
    #[test]
    fn preparation_identity_keeps_original_digest_and_wire_bytes() -> Result<()> {
        let mut jobs = Jobs::new(b"host");
        let original = jobs.begin("src/λ.lean", "Compile")?;
        // Independently computed from the original domain, host hash, little-
        // endian sequence 1 and exact UTF-8 resource, without typed wrappers.
        let expected = "sha256:72c84e1c46bbdf1a7f364df0025d5fde28e126fea29918e424e9ad79d3d2659a";
        assert_eq!(
            status(&jobs, "src/λ.lean")?.get("job"),
            Some(&serde_json::json!(expected))
        );
        assert_eq!(
            serde_json::to_string(&original.job)?,
            format!("\"{expected}\"")
        );
        Ok(())
    }
    #[test]
    fn replacement_clear_eviction_and_restart_cannot_revive_old_observers() -> Result<()> {
        let mut jobs = Jobs::new(b"session");
        let first = jobs.begin("A", "Compile")?;
        jobs.update(&first, 1, "Compile", 5, Some(("old", false)))?;
        let old = status(&jobs, "A")?
            .pointer("/log/revision")
            .context("field")?
            .as_str()
            .context("old")?
            .to_owned();
        let next = jobs.begin("A", "Compile")?;
        jobs.update(&first, 20, "Compile", 5, Some(("late", false)))?;
        jobs.finish(&first, 21, Ok(true));
        assert_eq!(
            status(&jobs, "A")?
                .pointer("/outcome/state")
                .context("field")?,
            "preparing"
        );
        assert!(jobs.bytes(&ArtifactId::parse(&old)?).is_none());
        jobs.update(&next, 10, "Compile", 5, Some(("new", false)))?;
        jobs.update(&next, 9, "Compile", 5, Some(("backwards", false)))?;
        assert_eq!(
            status(&jobs, "A")?
                .pointer("/elapsed_ms")
                .context("field")?,
            10
        );
        jobs.clear();
        let fresh = jobs.begin("A", "Compile")?;
        assert_ne!(fresh.job, next.job);
        assert_ne!(Jobs::new(b"restart").begin("A", "Compile")?.job, first.job);
        for i in 0..64 {
            jobs.begin(&format!("file{i}"), "Compile")?;
        }
        jobs.finish(&fresh, 25, Ok(true));
        assert!(status(&jobs, "A")?.is_null());
        assert_eq!(jobs.entries.len(), 64);
        Ok(())
    }
    #[test]
    fn phase_snapshots_limits_and_diagnostics_are_bounded() -> Result<()> {
        let mut jobs = Jobs::new(b"session");
        assert!(jobs.begin("", "Compile").is_err());
        let original = jobs.begin("A", "Resolve")?;
        jobs.update(&original, 2, "Resolve", 5, None)?;
        jobs.update(&original, 3, "Resolve", 5, Some(("first\n", true)))?;
        jobs.update(&original, 4, "Compile", 9, None)?;
        jobs.update(&original, 5, "Compile", 9, Some(("second\n", false)))?;
        let s = status(&jobs, "A")?;
        let bytes = jobs
            .bytes(&ArtifactId::parse(
                s.pointer("/log/revision")
                    .context("field")?
                    .as_str()
                    .context("id")?,
            )?)
            .context("body")?;
        let log: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            log.pointer("/data/text").context("field")?,
            "\n[Resolve]\nfirst\n\n[Compile]\nsecond\n"
        );
        assert_eq!(log.pointer("/data/truncated").context("field")?, true);
        assert_eq!(s.pointer("/phase_elapsed_ms").context("field")?, 1);
        assert_eq!(s.pointer("/limit_seconds").context("field")?, 9);
        assert!(jobs
            .update(&original, 6, "Foreign", 9, Some(("foreign", false)))
            .is_err());
        assert_eq!(status(&jobs, "A")?, s);
        jobs.finish(&original, 8, Err("𝕏".repeat(9000).into()));
        let failed = status(&jobs, "A")?;
        let error = failed
            .pointer("/outcome/error")
            .context("error")?
            .as_str()
            .context("text")?;
        assert!(error.len() <= 8000);
        assert!(error.encode_utf16().count() <= 8000);
        assert!(error.contains("diagnostic shortened"));
        assert!(jobs.begin("B", &"𝕏".repeat(129)).is_err());
        assert!(jobs.begin("B", &"𝕏".repeat(128)).is_ok());
        Ok(())
    }
}
