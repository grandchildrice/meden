#![cfg(target_os = "linux")]
//! Generic assessment with real confined source reads and scripted model data.
//! The finite parser supplies a different typed AST; no solver is invoked here.
use anyhow::{Context as _, Result};
use meden_agent::conversation::{ChatModel, ModelError, Reply};
use meden_app::{analysis, identity::snapshots::GraphRevision, model, semantic};
use meden_runtime::{
    plugin::{Definition, Descriptor, Role},
    process::Control,
    verification::finite::{Contract, Predicate},
    workspace::{SessionId, Workspace},
};
use serde::{Serialize, Serializer};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
const TEXT: &str = include_str!("../../../verification/examples/finite-counter.contract.json");
const FILE: &str = "counter.contract.json";
#[test]
fn foreign_context_and_missing_model_approval_cannot_send_analysis_requests() -> Result<()> {
    let temp = tempfile::tempdir()?;
    std::fs::write(temp.path().join(FILE), TEXT)?;
    let workspace = open(temp.path())?;
    let model = Scripted::new(temp.path(), Scenario::Complete)?;
    let original = observed(&workspace, &model)?;
    let mut foreign = input(&original);
    foreign.graph_revision = None;
    assert!(analysis::run(
        &workspace,
        &model,
        None,
        &Control::new(model.stop.clone()),
        &original,
        foreign,
        &|_| {}
    )
    .is_err());
    assert!(analysis::load(temp.path())?.is_none());
    model.authorized.store(false, Ordering::SeqCst);
    assert!(analysis::run(
        &workspace,
        &model,
        None,
        &Control::new(model.stop.clone()),
        &original,
        input(&original),
        &|_| {}
    )
    .is_err());
    assert!(model
        .requests
        .lock()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .is_empty());
    assert_eq!(
        analysis::load(temp.path())?
            .context("retained refusal")?
            .state,
        analysis::State::Failed
    );
    Ok(())
}
#[derive(Serialize)]
struct Finite {
    source: analysis::sources::Snapshot,
    contract: Contract,
    graph: GraphRevision,
    #[serde(skip)]
    phase: Arc<AtomicUsize>,
    #[serde(serialize_with = "scalar")]
    body: Arc<AtomicUsize>,
}
fn scalar<S: Serializer>(
    value: &Arc<AtomicUsize>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    value.load(Ordering::SeqCst).serialize(serializer)
}
#[derive(Serialize)]
struct FiniteContext {
    width: u8,
    fields: Vec<String>,
    parameters: Vec<String>,
    statement: Predicate,
}
impl analysis::sources::Source for Finite {
    fn sources(&self) -> &analysis::sources::Snapshot {
        &self.source
    }
}
impl semantic::Observation for Finite {
    type Context = FiniteContext;
    fn graph_revision(&self) -> Option<&GraphRevision> {
        Some(&self.graph)
    }
    fn context(&self, target: &semantic::Target) -> Result<Option<FiniteContext>> {
        anyhow::ensure!(
            Contract::parse(self.source.files.get(FILE).context("missing source")?)?
                == self.contract,
            "Original finite AST differs from source"
        );
        anyhow::ensure!(target.file == FILE, "Foreign source target");
        let property = self
            .contract
            .properties
            .iter()
            .find(|p| p.name == target.name)
            .context("Unknown property")?;
        Ok(Some(FiniteContext {
            width: self.contract.width,
            fields: self.contract.fields.clone(),
            parameters: self.contract.parameters.clone(),
            statement: property.statement.clone(),
        }))
    }
}
impl analysis::Observation for Finite {
    fn check(&self, _: &meden_app::adapters::Context<'_>) -> Result<analysis::Freshness> {
        match self.phase.load(Ordering::SeqCst) {
            1 => Ok(analysis::Freshness::Stale(anyhow::anyhow!(
                "Finite dependency changed"
            ))),
            2 => anyhow::bail!("Finite inspection unavailable"),
            _ => Ok(analysis::Freshness::Current),
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum Scenario {
    Complete,
    File,
    Dependency,
    Unavailable,
    Body,
    Revoke,
    Cancel,
}
struct Scripted {
    binding: model::Bound,
    requests: Mutex<Vec<Vec<Value>>>,
    authorized: AtomicBool,
    scenario: Scenario,
    path: std::path::PathBuf,
    phase: Arc<AtomicUsize>,
    body: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}
impl Scripted {
    fn new(path: &Path, scenario: Scenario) -> Result<Self> {
        let definition = Descriptor::new(Definition {
            id: "fixture.assessment",
            role: Role::Model,
            version: "1",
            formats: vec!["scripted".into()],
            implementation: b"generic assessment conformance, not inference",
            configuration: json!({}),
            input: model::input_schema()?,
            output: model::output_schema()?,
        })?;
        Ok(Self {
            binding: model::Bound::new(definition, "fixture".into(), "test://no-transport".into())?,
            requests: Mutex::default(),
            authorized: AtomicBool::new(true),
            scenario,
            path: path.into(),
            phase: Arc::default(),
            body: Arc::default(),
            stop: Arc::default(),
        })
    }
}
impl model::Model for Scripted {
    fn binding(&self) -> &model::Bound {
        &self.binding
    }
    fn is_authorized(&self, _: &SessionId) -> bool {
        self.authorized.load(Ordering::SeqCst)
    }
    fn for_session<'a>(&'a self, _: &'a SessionId) -> Box<dyn ChatModel + 'a> {
        Box::new(Caller(self))
    }
}
struct Caller<'a>(&'a Scripted);
impl ChatModel for Caller<'_> {
    fn reply(&self, messages: &[Value], _: &[Value]) -> std::result::Result<Reply, ModelError> {
        if !self.0.authorized.load(Ordering::SeqCst) {
            return Err(ModelError::Unavailable(
                "Scripted processor permission ended".into(),
            ));
        }
        let mut requests = self
            .0
            .requests
            .lock()
            .map_err(|e| ModelError::Unavailable(e.to_string()))?;
        requests.push(messages.to_vec());
        let first = requests.len() == 1;
        let (name, arguments) = if first {
            (
                "record_informalization",
                json!({"lemmaName":"step","naturalLanguage":"The finite counter follows its selected transition predicate.","preconditions":"four-bit inputs","postcondition":"the selected predicate holds","scope":"one transition","strength":"moderate","confidence":0.9}),
            )
        } else {
            (
                "record_comparison",
                json!({"lemmaName":"step","match":false,"discrepancy":"Scripted comparison","weakeningType":"missing-case","explanation":"Conformance observation only"}),
            )
        };
        if first {
            match self.0.scenario {
                Scenario::Complete => {}
                Scenario::File => std::fs::write(self.0.path.join(FILE), format!("{TEXT}\n"))
                    .map_err(|e| ModelError::Unavailable(e.to_string()))?,
                Scenario::Dependency => self.0.phase.store(1, Ordering::SeqCst),
                Scenario::Unavailable => self.0.phase.store(2, Ordering::SeqCst),
                Scenario::Body => self.0.body.store(1, Ordering::SeqCst),
                Scenario::Revoke => self.0.authorized.store(false, Ordering::SeqCst),
                Scenario::Cancel => self.0.stop.store(true, Ordering::SeqCst),
            }
        }
        serde_json::from_value(json!({"content":null,"tool_calls":[{"id":name,"function":{"name":name,"arguments":arguments.to_string()}}]})).map_err(|e|ModelError::Unavailable(e.to_string()))
    }
}
fn open(root: &Path) -> Result<Workspace> {
    Ok(Workspace::open(
        root,
        Path::new(env!("CARGO_BIN_EXE_meden")),
    )?)
}
fn observed(workspace: &Workspace, model: &Scripted) -> Result<Finite> {
    let control = Control::new(model.stop.clone());
    let source = analysis::sources::capture(workspace, &control, &[FILE.into()])?;
    let contract = Contract::parse(TEXT)?;
    let graph = GraphRevision::from_digest(Sha256::digest(serde_json::to_vec(&contract)?));
    Ok(Finite {
        source,
        contract,
        graph,
        phase: model.phase.clone(),
        body: model.body.clone(),
    })
}
fn input(original: &Finite) -> semantic::Input {
    semantic::Input {
        source_revision: original.source.revision.clone(),
        graph_revision: Some(original.graph.clone()),
        target: semantic::Target {
            file: FILE.into(),
            line: 1,
            name: "step".into(),
        },
        requirement: "ORIGINAL_REQUIREMENT_ONLY_SENTINEL".into(),
    }
}
#[test]
fn typed_finite_context_uses_the_same_assessment_journal_and_frozen_model_requests() -> Result<()> {
    let temp = tempfile::tempdir()?;
    std::fs::write(temp.path().join(FILE), TEXT)?;
    let workspace = open(temp.path())?;
    let model = Scripted::new(temp.path(), Scenario::Complete)?;
    let original = observed(&workspace, &model)?;
    let outcome = analysis::run(
        &workspace,
        &model,
        None,
        &Control::new(model.stop.clone()),
        &original,
        input(&original),
        &|_| {},
    )?;
    assert_eq!(outcome.state, analysis::State::Complete);
    assert!(outcome.finding_id.is_some());
    let requests = model
        .requests
        .lock()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    assert_eq!(requests.len(), 2);
    let first = serde_json::to_string(requests.first().context("translation")?)?;
    assert!(!first.contains("ORIGINAL_REQUIREMENT_ONLY_SENTINEL"));
    assert!(!first.contains("Lean Atlas"));
    assert!(first.contains("elaborated") && first.contains("statement") && first.contains("width"));
    assert!(
        serde_json::to_string(requests.get(1).context("comparison")?)?
            .contains("ORIGINAL_REQUIREMENT_ONLY_SENTINEL")
    );
    drop(requests);
    let stored = analysis::load(temp.path())?.context("retained observation")?;
    assert_eq!(stored.state, analysis::State::Historical);
    assert_eq!(stored.source_artifact, outcome.source_artifact);
    assert_eq!(stored.input, input(&original));
    // The neutral source-only type uses the same public application operation.
    let mut lexical = input(&original);
    lexical.graph_revision = None;
    let lexical_model = Scripted::new(temp.path(), Scenario::Complete)?;
    assert_eq!(
        analysis::run(
            &workspace,
            &lexical_model,
            None,
            &Control::new(lexical_model.stop.clone()),
            &original.source,
            lexical,
            &|_| {}
        )?
        .state,
        analysis::State::Complete
    );
    Ok(())
}
#[test]
fn common_assessment_refuses_stale_unavailable_mutated_revoked_and_cancelled_observations(
) -> Result<()> {
    for scenario in [
        Scenario::File,
        Scenario::Dependency,
        Scenario::Unavailable,
        Scenario::Body,
        Scenario::Revoke,
        Scenario::Cancel,
    ] {
        let temp = tempfile::tempdir()?;
        std::fs::write(temp.path().join(FILE), TEXT)?;
        let workspace = open(temp.path())?;
        let model = Scripted::new(temp.path(), scenario)?;
        let original = observed(&workspace, &model)?;
        let notified = std::cell::RefCell::new(None);
        let result = analysis::run(
            &workspace,
            &model,
            None,
            &Control::new(model.stop.clone()),
            &original,
            input(&original),
            &|record| *notified.borrow_mut() = Some(record.clone()),
        );
        assert!(result.is_err(), "{scenario:?}");
        let stored = analysis::load(temp.path())?.context("retained failure")?;
        assert!(
            stored.assessment.is_none() && stored.finding_id.is_none(),
            "{scenario:?}"
        );
        let expected = match scenario {
            Scenario::File | Scenario::Dependency | Scenario::Body => analysis::State::Stale,
            Scenario::Cancel => analysis::State::Cancelled,
            _ => analysis::State::Failed,
        };
        assert_eq!(
            notified.borrow().as_ref().context("final notice")?.state,
            expected,
            "{scenario:?}"
        );
        assert_eq!(
            stored.state,
            if expected == analysis::State::Stale {
                analysis::State::Historical
            } else {
                expected
            },
            "{scenario:?}"
        );
        if matches!(scenario, Scenario::Cancel | Scenario::Revoke) {
            assert_eq!(
                model
                    .requests
                    .lock()
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .len(),
                1
            );
        }
    }
    Ok(())
}
