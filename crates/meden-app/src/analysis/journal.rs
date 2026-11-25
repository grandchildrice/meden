//! Atomic local history, serialized across GUI/TUI processes. No model-writable path.
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{Record, State};
use crate::semantic::{self, Input, Verdict};

const LIMIT: usize = 1000;
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u8,
    sequence: u64,
    entries: Vec<Record>,
}
struct Store {
    path: PathBuf,
    _lock: File,
}
impl Store {
    fn open(root: &Path) -> Result<Self> {
        let directory = crate::storage::directory(root)?;
        fs::create_dir_all(&directory)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("meaning-analysis.lock"))?;
        acquire(&lock)?;
        Ok(Self {
            path: directory.join("meaning-analysis.json"),
            _lock: lock,
        })
    }
    fn read(&self) -> Result<Journal> {
        let journal: Option<Journal> = crate::storage::read(&self.path)?;
        let journal = journal.unwrap_or(Journal {
            version: 2,
            ..Journal::default()
        });
        anyhow::ensure!(
            journal.version == 2 && journal.entries.len() <= LIMIT,
            "unsupported or oversized meaning-analysis history"
        );
        let mut previous = 0;
        let mut ids = BTreeSet::new();
        for record in &journal.entries {
            record.input.validate()?;
            anyhow::ensure!(
                record.version == 2
                    && record.engine == semantic::ENGINE
                    && record.sequence > previous
                    && record.sequence <= journal.sequence
                    && valid_id(&record.id)
                    && ids.insert(record.id.clone())
                    && record.requirement_id == requirement_id(&record.input.requirement)
                    && record.finding_id == finding_id(record)
                    && record
                        .source_artifact
                        .as_ref()
                        .is_none_or(|id| valid_hex(id, "sha256:")),
                "invalid meaning-analysis history linkage"
            );
            previous = record.sequence;
        }
        Ok(journal)
    }
}
#[cfg(target_os = "linux")]
fn acquire(file: &File) -> Result<()> {
    rustix::fs::flock(file, rustix::fs::FlockOperation::LockExclusive)?;
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn acquire(_file: &File) -> Result<()> {
    anyhow::bail!("Local analysis history requires Linux")
}

fn analysis_id() -> Result<String> {
    let mut random = [0u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut random)?;
    Ok(format!("analysis-{:x}", Sha256::digest(random)))
}
fn valid_id(id: &str) -> bool {
    valid_hex(id, "analysis-")
}
fn valid_hex(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

fn requirement_id(text: &str) -> String {
    format!("requirement-{:x}", Sha256::digest(text.as_bytes()))
}
fn finding_id(record: &Record) -> Option<String> {
    record
        .assessment
        .as_ref()
        .filter(|a| a.verdict != Verdict::Match)
        .map(|_| record.id.replacen("analysis-", "finding-", 1))
}
pub(super) fn begin(
    root: &Path,
    model: &crate::model::Bound,
    input: Input,
    source_artifact: String,
    judge: Option<&crate::model::Bound>,
) -> Result<Record> {
    let store = Store::open(root)?;
    let mut journal = if let Ok(journal) = store.read() {
        journal
    } else {
        // Explicit rerun recovers without deleting an unreadable/old checkpoint.
        let parent = store.path.parent().context("history parent missing")?;
        let backup = tempfile::Builder::new()
            .prefix("meaning-analysis-retained-")
            .suffix(".json")
            .tempfile_in(parent)?;
        fs::rename(&store.path, backup.path())?;
        backup.keep().map_err(|error| error.error)?;
        File::open(parent)?.sync_all()?;
        Journal {
            version: 2,
            ..Journal::default()
        }
    };
    anyhow::ensure!(journal.entries.len() < LIMIT,
        "Meaning history contains 1000 runs. Archive the local history before starting another; no entries were deleted.");
    let sequence = journal
        .sequence
        .checked_add(1)
        .context("analysis sequence exhausted")?;
    let record = Record {
        version: 2,
        sequence,
        id: analysis_id()?,
        requirement_id: requirement_id(&input.requirement),
        finding_id: None,
        source_artifact: Some(source_artifact),
        engine: semantic::ENGINE.into(),
        model: model.view().model.clone(),
        processor: model.view().endpoint.clone(),
        model_definition: Some(serde_json::to_value(model.definition())?),
        judge_definition: judge
            .map(|j| serde_json::to_value(j.definition()))
            .transpose()?,
        judge: judge.map(|j| j.view().clone()),
        input,
        state: State::Observing,
        assessment: None,
        partial_translation: None,
        error: None,
    };
    journal.sequence = sequence;
    journal.entries.push(record.clone());
    crate::storage::write(&store.path, &journal)?;
    Ok(record)
}
pub(super) fn save(root: &Path, record: &Record) -> Result<()> {
    let store = Store::open(root)?;
    let mut journal = store.read()?;
    let saved = journal
        .entries
        .iter_mut()
        .find(|entry| entry.id == record.id)
        .context("analysis is not present in retained history")?;
    anyhow::ensure!(
        saved.input == record.input
            && saved.model == record.model
            && saved.processor == record.processor
            && saved.judge == record.judge
            && saved.model_definition == record.model_definition
            && saved.judge_definition == record.judge_definition
            && saved.source_artifact == record.source_artifact
            && saved.requirement_id == record.requirement_id
            && saved.sequence == record.sequence,
        "analysis history binding changed"
    );
    let mut next = record.clone();
    next.finding_id = finding_id(record);
    *saved = next;
    crate::storage::write(&store.path, &journal)
}
pub fn list(root: &Path) -> Result<Vec<Record>> {
    let store = Store::open(root)?;
    let mut entries = store.read()?.entries;
    for record in &mut entries {
        record.state = match record.state {
            State::Complete | State::Stale | State::Historical => State::Historical,
            State::Observing | State::Translating | State::Comparing => {
                record.error = Some("Previous meaning analysis did not finish. Rerun explicitly; no old result is current.".into());
                State::Cancelled
            }
            state => state,
        };
    }
    Ok(entries)
}
pub fn select(root: &Path, id: &str) -> Result<Record> {
    list(root)?
        .into_iter()
        .find(|record| record.id == id || record.finding_id.as_deref() == Some(id))
        .context("Analysis/Finding ID not found in this project's local history")
}

/// Retained data only. Historical artifacts never restore a running snapshot or approval.
pub(super) fn retain_sources(root: &Path, snapshot: &impl Serialize) -> Result<String> {
    let value = serde_json::to_value(snapshot)?;
    let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&value)?));
    let path = crate::storage::directory(root)?
        .join("analysis-sources")
        .join(format!("{digest}.json"));
    if let Some(existing) = crate::storage::read::<serde_json::Value>(&path)? {
        anyhow::ensure!(
            existing == value,
            "Retained analysis sources changed; no old artifact was overwritten"
        );
    } else {
        crate::storage::write(&path, &value)?;
    }
    Ok(format!("sha256:{digest}"))
}
