#![cfg(target_os = "linux")]
//! Shared reviewed repair with a typed finite model, real file workers and Z3.
//! Scripted replies test orchestration, not model quality or native evidence admission.
use anyhow::{Context as _, Result};
use meden_agent::conversation::{ChatModel, ModelError, Reply};
use meden_app::{
    analysis::{self, repair, sources},
    identity::{snapshots::GraphRevision, ContractId, RequirementRevision, SessionId},
    model, project, semantic,
};
use meden_runtime::{
    plugin::{Definition, Descriptor, Role},
    process::Control,
    verification::{finite, FailureKind},
    workspace::{SessionId as HostSession, Workspace},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
const SPEC: &str = include_str!("../../../verification/examples/finite-counter.contract.json");
const BROKEN: &str =
    include_str!("../../../verification/examples/finite-counter-broken.model.json");
const FIXED: &str = include_str!("../../../verification/examples/finite-counter.model.json");
const FILE: &str = "counter.model.json";
const DEP: &str = "counter.contract.json";
fn worker() -> &'static Path {
    static WORKER: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
        std::env::var_os("MEDEN_TEST_WORKER").map_or_else(
            || std::path::PathBuf::from(env!("CARGO_BIN_EXE_meden")),
            std::path::PathBuf::from,
        )
    });
    WORKER.as_path()
}
fn control() -> Control {
    Control::new(Arc::new(AtomicBool::new(false)))
}
#[derive(Serialize)]
struct Observed {
    source: sources::Snapshot,
    specification: finite::Contract,
    model: finite::Model,
    graph: GraphRevision,
    dependency: sources::Snapshot,
    #[serde(skip)]
    bad_refresh: bool,
    #[serde(skip)]
    bad_projection: bool,
    #[serde(serialize_with = "body_scalar")]
    body: Arc<AtomicUsize>,
}
fn body_scalar<S: serde::Serializer>(
    value: &Arc<AtomicUsize>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    value.load(Ordering::SeqCst).serialize(serializer)
}
impl sources::Source for Observed {
    fn sources(&self) -> &sources::Snapshot {
        &self.source
    }
}
#[derive(Serialize)]
struct Meaning {
    width: u8,
    transition: finite::Model,
}
impl semantic::Observation for Observed {
    type Context = Meaning;
    fn graph_revision(&self) -> Option<&GraphRevision> {
        Some(&self.graph)
    }
    fn context(&self, target: &semantic::Target) -> Result<Option<Meaning>> {
        anyhow::ensure!(
            target.file == FILE && target.name == "step",
            "Foreign finite target"
        );
        Ok(Some(Meaning {
            width: self.specification.width,
            transition: self.model.clone(),
        }))
    }
}
impl analysis::Observation for Observed {
    fn check(&self, context: &meden_app::adapters::Context<'_>) -> Result<analysis::Freshness> {
        let current = sources::capture(context.workspace, context.control, &[DEP.into()])?;
        if current.revision != self.dependency.revision {
            return Ok(analysis::Freshness::Stale(anyhow::anyhow!(
                "Finite specification changed"
            )));
        }
        Ok(analysis::Freshness::Current)
    }
}
#[derive(Serialize, Deserialize)]
struct Guard {
    dependency: meden_app::identity::snapshots::SourceRevision,
    roots: Vec<String>,
}
impl repair::Context for Guard {
    fn check(&self, context: &meden_app::adapters::Context<'_>) -> Result<()> {
        anyhow::ensure!(
            sources::capture(context.workspace, context.control, &[DEP.into()])?.revision
                == self.dependency,
            "Finite specification changed since review"
        );
        Ok(())
    }
    fn paths(&self) -> Option<&[String]> {
        Some(&self.roots)
    }
    fn relocatable(&self) -> bool {
        true
    }
}
impl repair::Observation for Observed {
    type Repair = Guard;
    fn repair_context(&self) -> Result<Guard> {
        Ok(Guard {
            dependency: self.dependency.revision.clone(),
            roots: vec![FILE.into()],
        })
    }
    fn refreshed_input(&self, original: &semantic::Input) -> Result<semantic::Input> {
        let mut input = original.clone();
        input.source_revision.clone_from(&self.source.revision);
        input.graph_revision = Some(self.graph.clone());
        if self.bad_refresh {
            input.requirement = "Changed intent".into();
        }
        Ok(input)
    }
}
fn observe(workspace: &Workspace) -> Result<Observed> {
    let source = sources::capture(workspace, &control(), &[FILE.into()])?;
    let dependency = sources::capture(workspace, &control(), &[DEP.into()])?;
    let specification =
        finite::Contract::parse(dependency.files.get(DEP).context("specification")?)?;
    let text = source.files.get(FILE).context("model source")?;
    // The actual finite backend validates the full grammar/domain and prepares
    // its real public solver resources. Only the explicit tests below run Z3.
    let _prepared = finite::prepare(text, &specification, worker(), &control())?;
    let model = serde_json::from_str(text)?;
    let graph = GraphRevision::from_digest(Sha256::digest(serde_json::to_vec(&(
        &source,
        &specification,
    ))?));
    Ok(Observed {
        source,
        specification,
        model,
        graph,
        dependency,
        bad_refresh: false,
        bad_projection: false,
        body: Arc::default(),
    })
}
fn input(original: &Observed) -> semantic::Input {
    semantic::Input {
        source_revision: original.source.revision.clone(),
        graph_revision: Some(original.graph.clone()),
        target: semantic::Target {
            file: FILE.into(),
            line: 1,
            name: "step".into(),
        },
        requirement:
            "Preserve the original four-bit counter bounds and reject overflowing increments."
                .into(),
    }
}
struct Scripted {
    binding: model::Bound,
    replies: Mutex<VecDeque<(&'static str, Value)>>,
    requests: Mutex<Vec<Vec<Value>>>,
    authorized: AtomicBool,
}
impl Scripted {
    fn new() -> Result<Self> {
        let descriptor = Descriptor::new(Definition {
            id: "fixture.repair",
            role: Role::Model,
            version: "1",
            formats: vec!["scripted".into()],
            implementation: b"typed repair conformance, not inference",
            configuration: json!({}),
            input: model::input_schema()?,
            output: model::output_schema()?,
        })?;
        Ok(Self {
            binding: model::Bound::new(descriptor, "fixture".into(), "test://no-transport".into())?,
            replies: Mutex::default(),
            requests: Mutex::default(),
            authorized: AtomicBool::new(true),
        })
    }
    fn assessment(&self) -> Result<()> {
        self.assessment_for("step")
    }
    fn assessment_for(&self, name: &str) -> Result<()> {
        self.replies.lock().map_err(|e|anyhow::anyhow!(e.to_string()))?.extend([
            ("record_informalization",json!({"lemmaName":name,"naturalLanguage":"The counter uses the supplied finite transition.","preconditions":"Four-bit inputs","postcondition":"One simultaneous transition","scope":"One step","strength":"moderate","confidence":0.9})),
            ("record_comparison",json!({"lemmaName":name,"match":false,"discrepancy":"Scripted counter mismatch","weakeningType":"wrong-property","explanation":"An actual solver result is checked separately"})),
        ]);
        Ok(())
    }
    fn proposal(&self, old: &str, new: &str, file: &str) -> Result<()> {
        self.replies.lock().map_err(|e|anyhow::anyhow!(e.to_string()))?.push_back(("propose_repair",json!({"file":file,"old_text":old,"new_text":new,"explanation":"Preserve the original counter behavior without overflow."})));
        Ok(())
    }
}
impl model::Model for Scripted {
    fn binding(&self) -> &model::Bound {
        &self.binding
    }
    fn is_authorized(&self, _: &HostSession) -> bool {
        self.authorized.load(Ordering::SeqCst)
    }
    fn for_session<'a>(&'a self, _: &'a HostSession) -> Box<dyn ChatModel + 'a> {
        Box::new(Caller(self))
    }
}
struct Caller<'a>(&'a Scripted);
impl ChatModel for Caller<'_> {
    fn reply(&self, messages: &[Value], _: &[Value]) -> std::result::Result<Reply, ModelError> {
        if !self.0.authorized.load(Ordering::SeqCst) {
            return Err(ModelError::Unavailable("Scripted processor ended".into()));
        }
        self.0
            .requests
            .lock()
            .map_err(|e| ModelError::Unavailable(e.to_string()))?
            .push(messages.to_vec());
        let (name, value) = self
            .0
            .replies
            .lock()
            .map_err(|e| ModelError::Unavailable(e.to_string()))?
            .pop_front()
            .ok_or_else(|| ModelError::Unavailable("Unexpected request".into()))?;
        serde_json::from_value(json!({"content":null,"tool_calls":[{"id":name,"function":{"name":name,"arguments":value.to_string()}}]})).map_err(|e|ModelError::Unavailable(e.to_string()))
    }
}
fn setup() -> Result<(tempfile::TempDir, Workspace, project::Project, Scripted)> {
    let temp = tempfile::tempdir()?;
    std::fs::write(temp.path().join(FILE), BROKEN)?;
    std::fs::write(temp.path().join(DEP), SPEC)?;
    let workspace = Workspace::open(temp.path(), worker())?;
    let project = project::Project::new(temp.path())?;
    Ok((temp, workspace, project, Scripted::new()?))
}
fn finding(
    workspace: &Workspace,
    model: &Scripted,
    original: &Observed,
) -> Result<analysis::Record> {
    model.assessment()?;
    analysis::run(
        workspace,
        model,
        None,
        &control(),
        original,
        input(original),
        &|_| {},
    )
}
fn review(
    workspace: &Workspace,
    project: &project::Project,
    model: &Scripted,
    original: &Observed,
    policy: &meden_core::Policy,
) -> Result<(repair::Queue<Guard>, repair::Review)> {
    let finding = finding(workspace, model, original)?;
    model.proposal(BROKEN, FIXED, FILE)?;
    let pending = repair::prepare(
        workspace,
        model,
        &control(),
        policy,
        original,
        &finding,
        &project.stored_status()?,
    )?;
    let mut queue = repair::Queue::default();
    let review = queue.install(workspace, pending)?;
    Ok((queue, review))
}
#[test]
fn finite_repair_reviews_once_applies_exact_source_and_reanalyzes_without_changing_intent(
) -> Result<()> {
    let (temp, workspace, project, model) = setup()?;
    let original = observe(&workspace)?;
    assert_eq!(
        finite::prepare(BROKEN, &original.specification, worker(), &control())?
            .check(&control())
            .err()
            .map(|e| e.kind),
        Some(FailureKind::Unverified)
    );
    let (mut queue, review) = review(
        &workspace,
        &project,
        &model,
        &original,
        &meden_app::workspace::file_policy()?,
    )?;
    assert_eq!(std::fs::read_to_string(temp.path().join(FILE))?, BROKEN);
    assert!(review.diff.contains("--- a/counter.model.json") && review.diff.contains('+'));
    let dirty = AtomicBool::new(false);
    let applied = queue.apply(&workspace, &control(), &review.reference, None, &|| {
        dirty.store(true, Ordering::SeqCst);
    })?;
    assert!(dirty.load(Ordering::SeqCst));
    assert_eq!(std::fs::read_to_string(temp.path().join(FILE))?, FIXED);
    assert!(queue
        .apply(&workspace, &control(), &review.reference, None, &|| {})
        .is_err());
    model.assessment()?;
    let (after, record) = applied.reanalyze(
        &workspace,
        &model,
        None,
        &control(),
        |_, paths, old| {
            assert_eq!(paths, &[FILE.to_owned()]);
            assert_eq!(old.requirement, input(&original).requirement);
            observe(&workspace)
        },
        &|_| {},
    )?;
    assert_eq!(record.input.requirement, input(&original).requirement);
    assert_ne!(record.input.source_revision, original.source.revision);
    assert_eq!(
        finite::prepare(FIXED, &after.specification, worker(), &control())?
            .check(&control())?
            .report()
            .class,
        meden_runtime::verification::artifact::Class::Bounded
    );
    assert!(!project.stored_status()?.verified);
    Ok(())
}
#[test]
fn reviewed_repairs_refuse_stale_sources_dependencies_foreign_contracts_denied_policy_and_cancellation(
) -> Result<()> {
    for case in [
        "source",
        "dependency",
        "contract",
        "policy",
        "cancel",
        "reference",
        "reject",
        "restart",
    ] {
        let (temp, workspace, project, model) = setup()?;
        let original = observe(&workspace)?;
        let policy = if case == "policy" {
            meden_core::Policy::from_labels("deny", &[], &[])?
        } else {
            meden_app::workspace::file_policy()?
        };
        let (mut queue, review) = review(&workspace, &project, &model, &original, &policy)?;
        let stop = control();
        let dirty = AtomicBool::new(false);
        let mut expected = BROKEN.to_owned();
        match case {
            "source" => {
                expected.push('\n');
                std::fs::write(temp.path().join(FILE), &expected)?;
            }
            "dependency" => std::fs::write(temp.path().join(DEP), format!("{SPEC}\n"))?,
            "cancel" => stop.cancel(),
            "reject" => queue.reject(&workspace, &review.reference)?,
            "restart" => queue = repair::Queue::default(),
            _ => {}
        }
        let foreign = project::Reference {
            revision: RequirementRevision::new(1)?,
            session: SessionId::parse("other")?,
            contract: ContractId::parse(&"1".repeat(64))?,
        };
        let mut reference = review.reference.clone();
        if case == "reference" {
            reference.sha256 = format!("sha256:{}", "0".repeat(64));
        }
        assert!(
            queue
                .apply(
                    &workspace,
                    &stop,
                    &reference,
                    if case == "contract" {
                        Some(&foreign)
                    } else {
                        None
                    },
                    &|| dirty.store(true, Ordering::SeqCst)
                )
                .is_err(),
            "{case}"
        );
        assert!(!dirty.load(Ordering::SeqCst), "{case}");
        assert_eq!(
            std::fs::read_to_string(temp.path().join(FILE))?,
            expected,
            "{case}"
        );
        assert!(
            queue
                .apply(&workspace, &control(), &review.reference, None, &|| {})
                .is_err(),
            "{case} replay"
        );
    }
    Ok(())
}
#[test]
fn applied_repair_retains_edits_but_refuses_reanalysis_that_replaces_original_intent() -> Result<()>
{
    let (temp, workspace, project, model) = setup()?;
    let original = observe(&workspace)?;
    let (mut queue, review) = review(
        &workspace,
        &project,
        &model,
        &original,
        &meden_app::workspace::file_policy()?,
    )?;
    let applied = queue.apply(&workspace, &control(), &review.reference, None, &|| {})?;
    let count = model
        .requests
        .lock()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .len();
    assert!(applied
        .reanalyze(
            &workspace,
            &model,
            None,
            &control(),
            |_, _, _| {
                let mut after = observe(&workspace)?;
                after.bad_refresh = true;
                Ok(after)
            },
            &|_| {}
        )
        .is_err());
    assert_eq!(
        model
            .requests
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .len(),
        count
    );
    assert_eq!(std::fs::read_to_string(temp.path().join(FILE))?, FIXED);
    assert!(!project.stored_status()?.verified);
    Ok(())
}

// The native owner receives this different decoded AST through exactly the same
// interface as Lean. It neither downcasts the result nor selects its repair type.
impl meden_app::view::Projection for Observed {
    fn page(&self) -> Result<meden_app::view::SourcePage> {
        use meden_app::view::{Graph, Kind, Location, Node, ProofHint, SourcePage};
        let mut page = SourcePage::from_snapshot(&self.source)?;
        page.compiled = true;
        page.description =
            "Validated finite transition data; graph observations confer no evidence.".into();
        page.graph = Some(Graph {
            revision: if self.bad_projection {
                GraphRevision::parse(&format!("sha256:{}", "0".repeat(64)))?
            } else {
                self.graph.clone()
            },
            declarations: vec![Node {
                id: "step".into(),
                name: "step".into(),
                label: "step".into(),
                kind: Kind::Definition,
                is_main_theorem: false,
                statement: format!("{}-bit simultaneous transition", self.specification.width),
                source: Location {
                    file: FILE.into(),
                    line: Some(1),
                },
                proof_status: ProofHint::Unknown,
                axioms: Vec::new(),
                semantic: None,
            }],
            edges: Vec::new(),
        });
        Ok(page)
    }
}
struct FinitePreparation {
    identity: usize,
    body: Arc<AtomicUsize>,
    calls: Arc<Mutex<Vec<usize>>>,
}
impl analysis::prepared::Backend for FinitePreparation {
    type Output = Observed;
    fn read(
        &self,
        context: &meden_app::adapters::Context<'_>,
        paths: &[String],
    ) -> Result<(Observed, bool)> {
        anyhow::ensure!(
            paths == [FILE.to_owned()],
            "Foreign finite source selection"
        );
        self.calls
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .push(self.identity);
        let mut output = observe(context.workspace)?;
        output.body = Arc::clone(&self.body);
        Ok((output, false))
    }
}
struct ForeignProjection {
    wrong_graph: bool,
}
impl analysis::prepared::Backend for ForeignProjection {
    type Output = Observed;
    fn read(
        &self,
        context: &meden_app::adapters::Context<'_>,
        _: &[String],
    ) -> Result<(Observed, bool)> {
        let mut observed = observe(context.workspace)?;
        observed.bad_projection = self.wrong_graph;
        Ok((observed, false))
    }
}
fn retain_native_views(
    context: &meden_app::adapters::Context<'_>,
    original: &analysis::prepared::Snapshot,
) -> Result<()> {
    use meden_app::view::{PreparedSource, SourcePage};
    let mut views = meden_app::view::sources::Sources::default();
    // Independent plain data and the finite AST share the actual native owner.
    views.publish(PreparedSource::new(
        meden_app::adapters::preparation::plain(context, &[DEP.into()])?,
    )?);
    views.publish(PreparedSource::new(original.clone())?);
    assert!(views.file(FILE).is_some() && views.file(DEP).is_some());
    assert!(SourcePage::from_snapshot(original)?.compiled);
    Ok(())
}
#[test]
fn native_owner_retains_typed_finite_preparation_through_review_and_actual_solver_recovery(
) -> Result<()> {
    use analysis::prepared::{Preparation, Repairs, Run};
    use meden_app::view::SourcePage;
    let (temp, workspace, project, model) = setup()?;
    let stop = control();
    let context = meden_app::adapters::Context {
        workspace: &workspace,
        control: &stop,
        observer: None,
    };
    let calls = Arc::new(Mutex::new(Vec::new()));
    let selected = Preparation::new(FinitePreparation {
        identity: 7,
        body: Arc::default(),
        calls: Arc::clone(&calls),
    });
    for invalid in [
        vec![String::new()],
        vec!["bad\0path".into()],
        vec![FILE.into(), FILE.into()],
        vec!["../../outside".into()],
    ] {
        assert!(selected.read(&context, &invalid).is_err());
    }
    assert!(calls
        .lock()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .is_empty());
    let (original, _) = selected.read(&context, &[FILE.into()])?;
    let (foreign, _) =
        Preparation::new(ForeignProjection { wrong_graph: true }).read(&context, &[FILE.into()])?;
    assert!(SourcePage::from_snapshot(&foreign).is_err());
    assert!(Preparation::new(ForeignProjection { wrong_graph: false })
        .read(&context, &[DEP.into()])
        .is_err());
    let original_body = serde_json::to_value(&original)?;
    let original_input = input(&observe(&workspace)?);
    assert_eq!(serde_json::to_value(observe(&workspace)?)?, original_body);
    retain_native_views(&context, &original)?;
    assert_eq!(
        finite::prepare(BROKEN, &finite::Contract::parse(SPEC)?, worker(), &stop)?
            .check(&stop)
            .err()
            .map(|e| e.kind),
        Some(FailureKind::Unverified)
    );
    let run = Run {
        workspace: &workspace,
        gateway: &model,
        judge: None,
        control: &stop,
        notify: &|_| {},
    };
    model.assessment()?;
    let finding = original.analyze(&run, original_input.clone())?;
    model.proposal(BROKEN, FIXED, FILE)?;
    let pending = original.prepare_repair(
        &run,
        &meden_app::workspace::file_policy()?,
        &finding,
        &project.stored_status()?,
    )?;
    let mut repairs = Repairs::default();
    let review = repairs.install(&workspace, pending)?;
    // Creating/selecting another implementation cannot replace the pending pair.
    let alternative = Preparation::new(FinitePreparation {
        identity: 99,
        body: Arc::default(),
        calls: Arc::clone(&calls),
    });
    drop(selected);
    drop(original);
    drop(alternative);
    model.assessment()?;
    let dirty = AtomicBool::new(false);
    let (after, record) = repairs.apply(&run, &review.reference, None, &|| {
        dirty.store(true, Ordering::SeqCst);
    })?;
    assert!(dirty.load(Ordering::SeqCst));
    assert_eq!(
        *calls.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?,
        vec![7, 7]
    );
    assert_eq!(record.input.requirement, original_input.requirement);
    assert_ne!(record.input.source_revision, original_input.source_revision);
    assert_eq!(
        after.sources().files.get(FILE).context("repaired source")?,
        FIXED
    );
    assert_eq!(std::fs::read_to_string(temp.path().join(FILE))?, FIXED);
    assert!(repairs
        .apply(&run, &review.reference, None, &|| {})
        .is_err());
    assert_eq!(
        finite::prepare(FIXED, &finite::Contract::parse(SPEC)?, worker(), &stop)?
            .check(&stop)?
            .report()
            .class,
        meden_runtime::verification::artifact::Class::Bounded
    );
    assert!(!project.stored_status()?.verified);
    Ok(())
}

#[test]
fn native_owner_delegates_repair_refusal_and_terminal_consumption_without_repreparing() -> Result<()>
{
    use analysis::prepared::{Preparation, Repairs, Run};
    for case in [
        "dependency",
        "policy",
        "cancel",
        "reference",
        "reject",
        "restart",
    ] {
        let (temp, workspace, project, model) = setup()?;
        let stop = control();
        let context = meden_app::adapters::Context {
            workspace: &workspace,
            control: &stop,
            observer: None,
        };
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (original, _) = Preparation::new(FinitePreparation {
            identity: 17,
            body: Arc::default(),
            calls: Arc::clone(&calls),
        })
        .read(&context, &[FILE.into()])?;
        let run = Run {
            workspace: &workspace,
            gateway: &model,
            judge: None,
            control: &stop,
            notify: &|_| {},
        };
        model.assessment()?;
        let finding = original.analyze(&run, input(&observe(&workspace)?))?;
        model.proposal(BROKEN, FIXED, FILE)?;
        let policy = if case == "policy" {
            meden_core::Policy::from_labels("deny", &[], &[])?
        } else {
            meden_app::workspace::file_policy()?
        };
        let pending =
            original.prepare_repair(&run, &policy, &finding, &project.stored_status()?)?;
        let mut repairs = Repairs::default();
        let review = repairs.install(&workspace, pending)?;
        match case {
            "dependency" => std::fs::write(temp.path().join(DEP), format!("{SPEC}\n"))?,
            "cancel" => stop.cancel(),
            "reject" => repairs.reject(&workspace, &review.reference)?,
            "restart" => repairs = Repairs::default(),
            _ => {}
        }
        let mut reference = review.reference.clone();
        if case == "reference" {
            reference.sha256 = format!("sha256:{}", "0".repeat(64));
        }
        let dirty = AtomicBool::new(false);
        assert!(
            repairs
                .apply(&run, &reference, None, &|| dirty
                    .store(true, Ordering::SeqCst))
                .is_err(),
            "{case}"
        );
        assert!(!dirty.load(Ordering::SeqCst), "{case}");
        assert_eq!(
            std::fs::read_to_string(temp.path().join(FILE))?,
            BROKEN,
            "{case}"
        );
        assert!(
            repairs
                .apply(&run, &review.reference, None, &|| {})
                .is_err(),
            "{case} replay"
        );
        assert_eq!(
            *calls.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?,
            vec![17],
            "{case}"
        );
    }
    Ok(())
}

#[test]
fn native_owner_refuses_context_drift_since_binding_before_the_first_model_call() -> Result<()> {
    use analysis::prepared::{Preparation, Run};
    let (temp, workspace, project, model) = setup()?;
    let stop = control();
    let body = Arc::new(AtomicUsize::new(0));
    let preparation = Preparation::new(FinitePreparation {
        identity: 23,
        calls: Arc::default(),
        body: Arc::clone(&body),
    });
    let (original, _) = preparation.read(
        &meden_app::adapters::Context {
            workspace: &workspace,
            control: &stop,
            observer: None,
        },
        &[FILE.into()],
    )?;
    let cloned = original.clone();
    let target = input(&observe(&workspace)?);
    let frozen = serde_json::to_value(&original)?;
    body.store(1, Ordering::SeqCst);
    assert!(serde_json::to_value(&original).is_err());
    assert!(meden_app::view::SourcePage::from_snapshot(&original).is_err());
    assert!(cloned.details().is_err());
    let run = Run {
        workspace: &workspace,
        gateway: &model,
        judge: None,
        control: &stop,
        notify: &|_| {},
    };
    assert!(cloned.analyze(&run, target).is_err());
    assert!(model
        .requests
        .lock()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .is_empty());
    assert!(analysis::load(temp.path())?.is_none());
    assert_eq!(std::fs::read_to_string(temp.path().join(FILE))?, BROKEN);
    assert!(!project.stored_status()?.verified);
    body.store(0, Ordering::SeqCst);
    assert_eq!(serde_json::to_value(cloned)?, frozen);
    Ok(())
}

fn production_finite_page(
    selected: &analysis::prepared::Preparation,
    context: &meden_app::adapters::Context<'_>,
) -> Result<(analysis::prepared::Snapshot, meden_app::view::SourcePage)> {
    use meden_app::view::SourcePage;
    let span = meden_runtime::metrics::Span::start("test.finite_source");
    let (original, hit) = selected.read(context, &[DEP.into()])?;
    assert!(!hit);
    let page = SourcePage::from_snapshot(&original)?;
    let graph = page.graph.as_ref().context("finite graph")?;
    assert_eq!(graph.declarations.len(), 3);
    assert_eq!(graph.edges.len(), 2);
    assert!(graph
        .declarations
        .iter()
        .all(|n| matches!(n.proof_status, meden_app::view::ProofHint::Unknown)));
    let node = graph
        .declarations
        .iter()
        .find(|n| n.name == "property.step")
        .context("original property")?;
    let explanation = node.semantic.as_ref().context("prepared reading/search")?;
    assert_eq!(explanation.checker.status, "counterexample");
    assert!(explanation
        .checker
        .candidates
        .iter()
        .any(|c| c.contains("input.value = 15")
            && c.contains("argument.amount = 1")
            && c.contains("output.value = 0")));
    let trace = serde_json::to_value(span.context())?
        .get("trace")
        .context("trace")?
        .clone();
    let metrics = serde_json::to_value(meden_runtime::metrics::snapshot())?;
    let before = metrics
        .get("records")
        .and_then(Value::as_array)
        .context("timing records")?
        .iter()
        .filter_map(|r| r.pointer("/context/span").and_then(Value::as_u64))
        .max()
        .unwrap_or(0);
    let (cached, hit) = selected.read(context, &[DEP.into()])?;
    assert!(hit);
    assert_eq!(
        serde_json::to_value(SourcePage::from_snapshot(&cached)?)?,
        serde_json::to_value(&page)?
    );
    let metrics = serde_json::to_value(meden_runtime::metrics::snapshot())?;
    let warm = metrics
        .get("records")
        .and_then(Value::as_array)
        .context("warm records")?
        .iter()
        .filter(|r| {
            r.pointer("/context/trace") == Some(&trace)
                && r.pointer("/context/span")
                    .and_then(Value::as_u64)
                    .is_some_and(|id| id > before)
        })
        .collect::<Vec<_>>();
    assert!(warm
        .iter()
        .any(|r| r.get("name").and_then(Value::as_str) == Some("analysis.finite.prepare")));
    assert!(!warm.iter().any(|r| matches!(
        r.get("name").and_then(Value::as_str),
        Some("finite.compile" | "analysis.z3.search" | "finite.render")
    )));

    Ok((original, page))
}

#[test]
fn production_finite_native_source_caches_actual_search_and_repairs_through_original_backend(
) -> Result<()> {
    use meden_app::{
        adapters::finite::Backend,
        analysis::prepared::{Preparation, Repairs, Run},
        view::SourcePage,
    };
    let (_temp, workspace, project, model) = setup()?;
    let control = control();
    let context = meden_app::adapters::Context {
        workspace: &workspace,
        control: &control,
        observer: None,
    };
    let selected = Preparation::new(Backend::new(worker().into()));
    let (original, page) = production_finite_page(&selected, &context)?;
    let graph = page.graph.as_ref().context("graph")?;
    let node = graph
        .declarations
        .iter()
        .find(|n| n.name == "transition.value")
        .context("original transition")?;
    let input = semantic::Input {
        source_revision: page.revision.clone(),
        graph_revision: Some(graph.revision.clone()),
        target: semantic::Target {
            file: node.source.file.clone(),
            line: usize::try_from(node.source.line.context("original line")?)?,
            name: node.name.clone(),
        },
        requirement: "Preserve the original counter behavior without overflow".into(),
    };
    let run = Run {
        workspace: &workspace,
        gateway: &model,
        judge: None,
        control: &control,
        notify: &|_| {},
    };
    model.assessment_for("transition.value")?;
    let finding = original.analyze(&run, input.clone())?;
    assert!(finding.finding_id.is_some());
    let first = serde_json::to_string(
        &model
            .requests
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .first()
            .context("first actual translation request")?,
    )?;
    assert!(first.contains("simultaneous_transition") && first.contains("selected"));
    assert!(
        !first.contains("independently")
            && !first.contains("Return the incremented value exactly")
            && !first.contains("counterexample")
    );
    model.proposal(BROKEN, FIXED, FILE)?;
    let pending = original.prepare_repair(
        &run,
        &meden_app::workspace::file_policy()?,
        &finding,
        &project.stored_status()?,
    )?;
    let mut repairs = Repairs::default();
    let review = repairs.install(&workspace, pending)?;
    model.assessment_for("transition.value")?;
    let (after, record) = repairs.apply(&run, &review.reference, None, &|| {})?;
    assert_eq!(record.input.requirement, input.requirement);
    assert_ne!(record.input.source_revision, input.source_revision);
    let next = SourcePage::from_snapshot(&after)?;
    assert!(next
        .graph
        .as_ref()
        .context("repaired graph")?
        .declarations
        .iter()
        .filter(|n| matches!(n.kind, meden_app::view::Kind::Theorem))
        .all(|n| n
            .semantic
            .as_ref()
            .is_some_and(|s| s.checker.status == "not_found")));
    assert_eq!(
        after.sources().files.get(FILE).map(String::as_str),
        Some(FIXED)
    );
    assert!(!project.stored_status()?.verified);
    assert!(repairs
        .apply(&run, &review.reference, None, &|| {})
        .is_err());
    Ok(())
}
