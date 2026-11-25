//! Reviewed, single-file candidate repairs using the frozen-request review kernel.
//! Parsing, source attribution, filesystem effects and model quality remain adapters.
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::model::Model;
use anyhow::{Context as _, Result};
use meden_agent::conversation::ChatModel;
use meden_agent::tool::Args;
use meden_core::{
    export_control::{ExportEvent, Origin, State},
    file::{Operation, Request},
    invocation::{Attempt, Notice},
    Policy,
};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::marker::PhantomData;

use super::{sources, Record};
use crate::semantic::{self, Input};

/// Backend-owned immutable repair observations, never approval or evidence.
pub trait Context: Serialize + DeserializeOwned {
    fn check(&self, context: &crate::adapters::Context<'_>) -> Result<()>;
    fn paths(&self) -> Option<&[String]>;
    fn relocatable(&self) -> bool;
}
pub trait Observation: super::Observation {
    type Repair: Context;
    fn repair_context(&self) -> Result<Self::Repair>;
    fn refreshed_input(&self, original: &Input) -> Result<Input>;
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    #[schemars(regex(pattern = r"^repair-[a-f0-9]{64}$"))]
    pub id: String,
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub sha256: String,
}
#[derive(Clone, Serialize)]
pub struct Review {
    pub reference: Reference,
    pub model: String,
    pub processor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_definition: Option<serde_json::Value>,
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
struct Frozen<G> {
    id: String,
    model: String,
    processor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_definition: Option<serde_json::Value>,
    finding_id: String,
    requirement_id: String,
    input: Input,
    accepted_reference: Option<crate::project::Reference>,
    files: BTreeMap<String, String>,
    context: G,
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
/// Backend context cannot be exchanged at the shared review queue.
/// ```compile_fail,E0308
/// use meden_app::analysis::repair::{Context,Queue,Pending,Review};
/// fn wrong<A:Context,B:Context>(q:&mut Queue<A>,p:Pending<B>,w:&meden_runtime::workspace::Workspace)->anyhow::Result<Review> { q.install(w,p) }
/// ```
pub struct Pending<G> {
    context: PhantomData<fn() -> G>,
    state: State,
    created: Instant,
}
pub struct Queue<G> {
    pending: Option<Pending<G>>,
}
impl<G> Default for Queue<G> {
    fn default() -> Self {
        Self { pending: None }
    }
}
/// Applied patches keep their reviewed intent private through reanalysis.
/// ```compile_fail,E0616
/// use meden_app::analysis::repair::Applied;
/// fn weaken<G>(applied:&mut Applied<G>) { applied.input.requirement.clear(); }
/// ```
pub struct Applied<G> {
    context: G,
    input: Input,
    paths: Vec<String>,
    review: Review,
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
fn observe<G: Context>(workspace: &Workspace, control: &Control, frozen: &Frozen<G>) -> Result<()> {
    frozen.context.check(&crate::adapters::Context {
        workspace,
        control,
        observer: None,
    })?;
    observe_sources(workspace, control, &frozen.input, &frozen.files)
}
fn observe_sources(
    workspace: &Workspace,
    control: &Control,
    input: &Input,
    files: &BTreeMap<String, String>,
) -> Result<()> {
    let paths = files.keys().cloned().collect::<Vec<_>>();
    let current = sources::capture(workspace, control, &paths)?;
    anyhow::ensure!(
        current.revision == input.source_revision,
        "Sources changed since the Finding; reread and analyze again before repairing"
    );
    Ok(())
}

pub fn prepare<O: Observation>(
    workspace: &Workspace,
    gateway: &dyn Model,
    control: &Control,
    policy: &Policy,
    snapshot: &O,
    record: &Record,
    project: &crate::project::Summary,
) -> Result<Pending<O::Repair>> {
    control.with_active(|| Ok(()))?;
    anyhow::ensure!(
        gateway.is_authorized(workspace.session_id()),
        "Review the configured processor before preparing a repair"
    );
    let original_body = serde_json::to_value(snapshot)?;
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
    semantic::Prepared::new(record.input.clone(), snapshot)?;
    let context = snapshot.repair_context()?;
    let original_context = serde_json::to_vec(&context)?;
    context.check(&crate::adapters::Context {
        workspace,
        control,
        observer: None,
    })?;
    observe_sources(workspace, control, &record.input, &snapshot.sources().files)?;
    anyhow::ensure!(!control.flag().load(Ordering::SeqCst), "Repair cancelled");
    let caller = crate::model::bind(gateway, workspace.session_id());
    let reply = caller.reply(&[
        json!({"role":"system","content":include_str!("repair.md")}),
        json!({"role":"user","content":json!({"input":record.input,"finding":record.assessment,"accepted_contract":project.accepted_contract,"files":snapshot.sources().files}).to_string()}),
    ], &[json!({"type":"function","function":{"name":"propose_repair","description":"Return a single data-only source replacement for host review.","parameters":{"type":"object","properties":{
        "file":{"type":"string"},"old_text":{"type":"string"},"new_text":{"type":"string"},"explanation":{"type":"string"}},"required":["file","old_text","new_text","explanation"],"additionalProperties":false}}})])?;
    let suggestion: Suggestion = semantic::decode(reply, "propose_repair")?;
    anyhow::ensure!(
        serde_json::to_value(snapshot)? == original_body
            && serde_json::to_vec(&context)? == original_context,
        "Repair source context changed during proposal; no proposal admitted"
    );
    anyhow::ensure!(
        !control.flag().load(Ordering::SeqCst) && gateway.is_authorized(workspace.session_id()),
        "Repair cancelled or processor permission ended; no proposal admitted"
    );
    let replacement = replacement(snapshot, &context, &record.input, &suggestion)?;
    let original = snapshot
        .sources()
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
        model: caller.binding().view().model.clone(),
        processor: caller.binding().view().endpoint.clone(),
        model_definition: Some(serde_json::to_value(caller.binding().definition())?),
        finding_id: finding.clone(),
        requirement_id: record.requirement_id.clone(),
        input: record.input.clone(),
        accepted_reference: project.accepted_reference.clone(),
        files: snapshot.sources().files.clone(),
        context,
        request,
        policy: policy.clone(),
        explanation: suggestion.explanation,
    };
    observe(workspace, control, &frozen)?;
    control.with_active(|| Ok(()))?;
    Ok(Pending {
        context: PhantomData,
        state: State::new(
            workspace.session_id().as_bytes().to_vec(),
            format!("/workspace/{}", suggestion.file),
            serde_json::to_vec(&frozen)?,
        ),
        created: Instant::now(),
    })
}

fn replacement<O: Observation>(
    snapshot: &O,
    context: &O::Repair,
    input: &Input,
    suggestion: &Suggestion,
) -> Result<String> {
    anyhow::ensure!(
        snapshot.sources().files.contains_key(&suggestion.file)
            && !suggestion.old_text.is_empty()
            && suggestion.old_text != suggestion.new_text
            && !suggestion.explanation.trim().is_empty()
            && suggestion.explanation.len() <= 16_384,
        "Repair must change one observed file and explain the change"
    );
    let original = snapshot
        .sources()
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
    if !context.relocatable() && suggestion.file == input.target.file {
        let before = input.target.line.saturating_sub(1);
        anyhow::ensure!(original.lines().take(before).eq(replacement.lines().take(before))
            && replacement.lines().nth(before).is_some_and(|line|line.contains(input.target.name.rsplit(['.', ':']).next().unwrap_or_default())),
            "Source-only repair must preserve the declaration's line anchor; reread a compiled declaration to relocate it");
    }
    anyhow::ensure!(
        snapshot
            .sources()
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

impl<G: Context> Pending<G> {
    fn review(&self) -> Result<Review> {
        let frozen: Frozen<G> = serde_json::from_slice(self.state.request().body())?;
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
            model_definition: frozen.model_definition,
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
impl<G: Context> Queue<G> {
    pub fn install(&mut self, workspace: &Workspace, mut pending: Pending<G>) -> Result<Review> {
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
                    workspace.session_id().as_bytes()
                )
                .accepted,
            "Repair could not enter review"
        );
        log(workspace.path(), &review, "reviewed", None, None)?;
        self.pending = Some(pending);
        Ok(review)
    }
    pub fn abandon(&mut self, workspace: &Workspace) -> Result<()> {
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
    pub fn reject(&mut self, workspace: &Workspace, reference: &Reference) -> Result<()> {
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
            workspace.session_id().as_bytes(),
        );
        log(workspace.path(), &review, "rejected", None, None)
    }
    pub fn apply(
        &mut self,
        workspace: &Workspace,
        control: &Control,
        reference: &Reference,
        accepted_reference: Option<&crate::project::Reference>,
        dirty: &dyn Fn(),
    ) -> Result<Applied<G>> {
        let mut pending = self
            .pending
            .take()
            .context("Repair already consumed, rejected or unavailable after restart")?;
        let review = pending.review()?;
        let result = (|| -> Result<Applied<G>> {
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
                    workspace.session_id().as_bytes(),
                )
                .effect
                .context("Repair approval expired or belongs to another session")?;
            let frozen: Frozen<G> = serde_json::from_slice(effect.into_request().body())?;
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
                    .context
                    .paths()
                    .map_or_else(|| frozen.files.into_keys().collect(), <[String]>::to_vec),
                context: frozen.context,
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
fn record_reanalysis<G>(root: &Path, applied: &Applied<G>, result: &Result<Record>) -> Result<()> {
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

impl<G: Context> Applied<G> {
    /// Consume one applied patch's frozen intent through a backend's source
    /// preparation, then the same shared assessment and history operation.
    pub fn reanalyze<O: Observation<Repair = G>>(
        self,
        workspace: &Workspace,
        gateway: &dyn Model,
        judge: Option<&dyn Model>,
        control: &Control,
        prepare: impl FnOnce(&G, &[String], &Input) -> Result<O>,
        notify: &dyn Fn(&Record),
    ) -> Result<(O, Record)> {
        let result = (|| {
            let snapshot = prepare(&self.context, &self.paths, &self.input)?;
            let input = snapshot.refreshed_input(&self.input)?;
            anyhow::ensure!(
                input.target.file == self.input.target.file
                    && input.target.name == self.input.target.name
                    && input.requirement == self.input.requirement
                    && input.graph_revision.is_some() == self.input.graph_revision.is_some()
                    && (self.context.relocatable() || input.target.line == self.input.target.line),
                "Repair applied; reanalysis changed the frozen target, requirement or context mode"
            );
            let record = super::run(workspace, gateway, judge, control, &snapshot, input, notify)?;
            Ok((snapshot, record))
        })();
        let record_result = result
            .as_ref()
            .map(|(_, record)| record.clone())
            .map_err(|error| anyhow::anyhow!("{error:#}"));
        record_reanalysis(workspace.path(),&self,&record_result)
            .context("Repair was applied; reanalysis linkage could not be saved. Inspect retained files and analysis history")?;
        result.context("Repair was applied; reanalysis stopped. Files are retained; reread and explicitly rerun")
    }
}
