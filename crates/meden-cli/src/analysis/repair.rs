//! Reviewed, single-file candidate repairs using the frozen-request review kernel.
//! Parsing, source attribution, filesystem effects and model quality remain adapters.
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use meden_agent::{conversation::ChatModel, tool::Args};
use meden_core::{
    export_control::{ExportEvent, Origin, State},
    file::{Operation, Request},
    invocation::{Attempt, Notice},
    Policy,
};
use meden_llm::Gateway;
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{sources, Record};
use crate::semantic::{self, Input};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reference {
    pub id: String,
    pub sha256: String,
}
#[derive(Clone, Serialize)]
pub(crate) struct Review {
    pub reference: Reference,
    pub model: String,
    pub processor: String,
    pub finding_id: String,
    pub requirement_id: String,
    pub input: Input,
    pub file: String,
    pub accepted_reference: Option<crate::project::Reference>,
    pub explanation: String,
    pub diff: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Suggestion {
    file: String,
    old_text: String,
    new_text: String,
    explanation: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Frozen {
    id: String,
    model: String,
    processor: String,
    finding_id: String,
    requirement_id: String,
    input: Input,
    accepted_reference: Option<crate::project::Reference>,
    files: BTreeMap<String, String>,
    resources: Option<String>,
    roots: Option<Vec<String>>,
    request: Request<Args>,
    policy: Policy,
    explanation: String,
}
#[derive(Serialize)]
struct Log<'a> {
    version: u8,
    review: &'a Review,
    state: &'a str,
    error: Option<&'a str>,
    analysis_id: Option<&'a str>,
}
pub(crate) struct Pending {
    state: State,
    created: Instant,
}
#[derive(Default)]
pub(crate) struct Queue {
    pending: Option<Pending>,
}
pub(crate) struct Applied {
    pub input: Input,
    pub paths: Vec<String>,
    pub review: Review,
}

fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn id() -> Result<String> {
    use std::io::Read;
    let mut bytes = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(format!("repair-{:x}", Sha256::digest(bytes)))
}
fn log(
    root: &Path,
    review: &Review,
    state: &str,
    error: Option<&str>,
    analysis_id: Option<&str>,
) -> Result<()> {
    // IDs originate here and never from a model/browser-selected storage path.
    anyhow::ensure!(
        review
            .reference
            .id
            .strip_prefix("repair-")
            .is_some_and(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())),
        "invalid repair identity"
    );
    crate::storage::write(
        &crate::storage::directory(root)?
            .join("repairs")
            .join(format!("{}.json", review.reference.id)),
        &Log {
            version: 1,
            review,
            state,
            error,
            analysis_id,
        },
    )
}
fn observe(workspace: &Workspace, control: &Control, frozen: &Frozen) -> Result<()> {
    observe_sources(
        workspace,
        control,
        &frozen.input,
        &frozen.files,
        frozen.resources.as_deref(),
    )
}
fn observe_sources(
    workspace: &Workspace,
    control: &Control,
    input: &Input,
    files: &BTreeMap<String, String>,
    resources: Option<&str>,
) -> Result<()> {
    let paths = files.keys().cloned().collect::<Vec<_>>();
    let current = sources::capture(workspace, control, &paths)?;
    anyhow::ensure!(
        current.revision == input.source_revision,
        "Sources changed since the Finding; reread and analyze again before repairing"
    );
    if let Some(resources) = resources {
        anyhow::ensure!(
            crate::atlas::Configuration::host().current_resources()? == resources,
            "Atlas resources changed since the Finding; reread and analyze again"
        );
    }
    Ok(())
}

pub(crate) fn prepare(
    workspace: &Workspace,
    gateway: &Gateway,
    control: &Control,
    policy: &Policy,
    snapshot: &sources::Snapshot,
    record: &Record,
    project: &crate::project::Summary,
) -> Result<Pending> {
    let finding = record
        .finding_id
        .as_ref()
        .context("Only a retained mismatch/unclear Finding can request a repair")?;
    anyhow::ensure!(
        record.assessment.is_some()
            && matches!(
                record.state,
                super::State::Complete | super::State::Historical
            ),
        "The Finding is unavailable or stale; reread and analyze again"
    );
    semantic::Prepared::new(
        record.input.clone(),
        snapshot.files.clone(),
        snapshot.atlas.as_ref(),
    )?;
    let resources = snapshot
        .atlas
        .as_ref()
        .map(|atlas| atlas.resources_sha256.clone());
    observe_sources(
        workspace,
        control,
        &record.input,
        &snapshot.files,
        resources.as_deref(),
    )?;
    anyhow::ensure!(!control.flag().load(Ordering::SeqCst), "Repair cancelled");
    let reply = gateway.for_session(workspace.session_id()).reply(&[
        json!({"role":"system","content":include_str!("repair.md")}),
        json!({"role":"user","content":json!({"input":record.input,"finding":record.assessment,"accepted_contract":project.accepted_contract,"files":snapshot.files}).to_string()}),
    ], &[json!({"type":"function","function":{"name":"propose_repair","description":"Return a single data-only source replacement for host review.","parameters":{"type":"object","properties":{
        "file":{"type":"string"},"old_text":{"type":"string"},"new_text":{"type":"string"},"explanation":{"type":"string"}},"required":["file","old_text","new_text","explanation"],"additionalProperties":false}}})])?;
    let suggestion: Suggestion = semantic::decode(reply, "propose_repair")?;
    anyhow::ensure!(
        !control.flag().load(Ordering::SeqCst) && gateway.is_authorized(workspace.session_id()),
        "Repair cancelled or processor permission ended; no proposal admitted"
    );
    let replacement = replacement(snapshot, &record.input, &suggestion)?;
    let original = snapshot
        .files
        .get(&suggestion.file)
        .context("repair source unavailable")?;
    let args = Args::from([
        ("old_text".into(), json!(original)),
        ("new_text".into(), json!(replacement)),
        ("whole_file".into(), json!(true)),
    ]);
    let request = Request::new(
        Operation::Edit,
        format!("/workspace/{}", suggestion.file).as_str().into(),
        args,
    );
    let frozen = Frozen {
        id: id()?,
        model: gateway.provider().model.clone(),
        processor: gateway.endpoint().into(),
        finding_id: finding.clone(),
        requirement_id: record.requirement_id.clone(),
        input: record.input.clone(),
        accepted_reference: project.accepted_reference.clone(),
        files: snapshot.files.clone(),
        resources,
        roots: snapshot
            .collection
            .as_ref()
            .map(|collection| collection.roots.clone()),
        request,
        policy: policy.clone(),
        explanation: suggestion.explanation,
    };
    observe(workspace, control, &frozen)?;
    control.with_active(|| Ok(()))?;
    Ok(Pending {
        state: State::new(
            workspace.session_id().to_vec(),
            format!("/workspace/{}", suggestion.file),
            serde_json::to_vec(&frozen)?,
        ),
        created: Instant::now(),
    })
}

fn replacement(
    snapshot: &sources::Snapshot,
    input: &Input,
    suggestion: &Suggestion,
) -> Result<String> {
    anyhow::ensure!(
        snapshot.files.contains_key(&suggestion.file)
            && !suggestion.old_text.is_empty()
            && suggestion.old_text != suggestion.new_text
            && !suggestion.explanation.trim().is_empty()
            && suggestion.explanation.len() <= 16_384,
        "Repair must change one observed file and explain the change"
    );
    let original = snapshot
        .files
        .get(&suggestion.file)
        .context("repair source unavailable")?;
    anyhow::ensure!(
        original.matches(&suggestion.old_text).count() == 1,
        "Repair old_text must match exactly once; no file changed"
    );
    let replacement = original.replacen(&suggestion.old_text, &suggestion.new_text, 1);
    anyhow::ensure!(
        replacement.len() <= 65_536,
        "Repaired source exceeds 64 KiB"
    );
    if snapshot.atlas.is_none() && suggestion.file == input.target.file {
        let before = input.target.line.saturating_sub(1);
        anyhow::ensure!(original.lines().take(before).eq(replacement.lines().take(before))
            && replacement.lines().nth(before).is_some_and(|line|line.contains(input.target.name.rsplit(['.', ':']).next().unwrap_or_default())),
            "Source-only repair must preserve the declaration's line anchor; use Atlas for relocatable Lean declarations");
    }
    anyhow::ensure!(
        snapshot
            .files
            .values()
            .map(String::len)
            .sum::<usize>()
            .saturating_sub(original.len())
            .saturating_add(replacement.len())
            <= 1_048_576,
        "Repaired sources exceed 1 MiB"
    );
    Ok(replacement)
}
fn diff_lines(prefix: char, source: &str) -> String {
    let mut diff = String::new();
    for line in source.split_inclusive('\n') {
        diff.push(prefix);
        diff.push_str(line);
        if !line.ends_with('\n') {
            diff.push_str("\n\\ No newline at end of file\n");
        }
    }
    diff
}

impl Pending {
    fn review(&self) -> Result<Review> {
        let frozen: Frozen = serde_json::from_slice(self.state.request().body())?;
        let (_, _, args) = frozen.request.into_parts();
        let original = args
            .get("old_text")
            .and_then(|v| v.as_str())
            .context("frozen repair old_text missing")?;
        let replacement = args
            .get("new_text")
            .and_then(|v| v.as_str())
            .context("frozen repair new_text missing")?;
        let file = self
            .state
            .request()
            .target()
            .strip_prefix("/workspace/")
            .context("frozen repair destination missing")?
            .to_owned();
        let removed = diff_lines('-', original);
        let added = diff_lines('+', replacement);
        Ok(Review {
            reference: Reference {
                id: frozen.id,
                sha256: hash(self.state.request().body()),
            },
            model: frozen.model,
            processor: frozen.processor,
            finding_id: frozen.finding_id,
            requirement_id: frozen.requirement_id,
            input: frozen.input,
            file: file.clone(),
            accepted_reference: frozen.accepted_reference,
            explanation: frozen.explanation,
            diff: format!(
                "--- a/{file}\n+++ b/{file}\n@@ -1,{} +1,{} @@\n{removed}{added}",
                original.lines().count(),
                replacement.lines().count()
            ),
        })
    }
}
impl Queue {
    pub(crate) fn install(
        &mut self,
        workspace: &Workspace,
        mut pending: Pending,
    ) -> Result<Review> {
        if let Some(old) = self.pending.take() {
            log(workspace.path(), &old.review()?, "superseded", None, None)?;
        }
        let review = pending.review()?;
        anyhow::ensure!(
            pending
                .state
                .advance(
                    Origin::Host,
                    ExportEvent::Review,
                    pending.created.elapsed().as_secs(),
                    1800,
                    workspace.session_id()
                )
                .accepted,
            "Repair could not enter review"
        );
        log(workspace.path(), &review, "reviewed", None, None)?;
        self.pending = Some(pending);
        Ok(review)
    }
    pub(crate) fn abandon(&mut self, workspace: &Workspace) -> Result<()> {
        if let Some(pending) = self.pending.take() {
            log(
                workspace.path(),
                &pending.review()?,
                "superseded",
                None,
                None,
            )?;
        }
        Ok(())
    }
    pub(crate) fn reject(&mut self, workspace: &Workspace, reference: &Reference) -> Result<()> {
        let mut pending = self.pending.take().context("No repair awaits review")?;
        let review = pending.review()?;
        anyhow::ensure!(
            review.reference == *reference,
            "Repair review target changed; no approval retained"
        );
        pending.state.advance(
            Origin::Host,
            ExportEvent::Reject,
            pending.created.elapsed().as_secs(),
            1800,
            workspace.session_id(),
        );
        log(workspace.path(), &review, "rejected", None, None)
    }
    pub(crate) fn apply(
        &mut self,
        workspace: &Workspace,
        control: &Control,
        reference: &Reference,
        accepted_reference: Option<&crate::project::Reference>,
        dirty: &dyn Fn(),
    ) -> Result<Applied> {
        let mut pending = self
            .pending
            .take()
            .context("Repair already consumed, rejected or unavailable after restart")?;
        let review = pending.review()?;
        let result = (|| -> Result<Applied> {
            anyhow::ensure!(
                review.reference == *reference,
                "Repair review target changed; no file changed"
            );
            let effect = pending
                .state
                .advance(
                    Origin::Host,
                    ExportEvent::Approve,
                    pending.created.elapsed().as_secs(),
                    1800,
                    workspace.session_id(),
                )
                .effect
                .context("Repair approval expired or belongs to another session")?;
            let frozen: Frozen = serde_json::from_slice(effect.into_request().body())?;
            anyhow::ensure!(
                frozen.accepted_reference.as_ref() == accepted_reference,
                "Accepted contract changed since repair preparation; no file changed"
            );
            observe(workspace, control, &frozen)?;
            log(workspace.path(), &review, "applying", None, None)?;
            let mut attempt = Attempt::with_policy(
                Ok::<_, &str>(frozen.request.prepare()),
                &frozen.policy,
                "Repair file policy refused",
            );
            anyhow::ensure!(
                matches!(attempt.announce(), Notice::Allowed),
                "Repair file policy refused"
            );
            let call = attempt
                .dispatch(control.flag().load(Ordering::SeqCst), None::<()>)
                .map_err(|_| anyhow::anyhow!("Repair stopped before dispatch"))?;
            let (request, policy) = call.into_parts();
            dirty();
            meden_runtime::file::run(
                workspace,
                control,
                &request,
                policy,
                &Limits {
                    timeout: Duration::from_secs(30),
                    capture_bytes: 160_000,
                    output_bytes: 1_000_000,
                },
            )?;
            log(
                workspace.path(),
                &review,
                "applied_reanalysis_required",
                None,
                None,
            )?;
            Ok(Applied {
                input: frozen.input,
                paths: frozen
                    .roots
                    .unwrap_or_else(|| frozen.files.into_keys().collect()),
                review: review.clone(),
            })
        })();
        if let Err(error) = &result {
            let _ = log(
                workspace.path(),
                &review,
                "stopped",
                Some(&format!("{error:#}")),
                None,
            );
        }
        result
    }
}
pub(crate) fn record_reanalysis(
    root: &Path,
    applied: &Applied,
    result: &Result<Record>,
) -> Result<()> {
    match result {
        Ok(record) => log(root, &applied.review, "reanalyzed", None, Some(&record.id)),
        Err(error) => log(
            root,
            &applied.review,
            "applied_reanalysis_stopped",
            Some(&format!("{error:#}")),
            None,
        ),
    }
}
