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
        atomic::{AtomicBool, Ordering},
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
    Path::new(env!("CARGO_BIN_EXE_meden"))
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
        self.replies.lock().map_err(|e|anyhow::anyhow!(e.to_string()))?.extend([
            ("record_informalization",json!({"lemmaName":"step","naturalLanguage":"The counter uses the supplied finite transition.","preconditions":"Four-bit inputs","postcondition":"One simultaneous transition","scope":"One step","strength":"moderate","confidence":0.9})),
            ("record_comparison",json!({"lemmaName":"step","match":false,"discrepancy":"Scripted counter mismatch","weakeningType":"wrong-property","explanation":"An actual solver result is checked separately"})),
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
