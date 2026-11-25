use crate::adapters::lean::{search::Report, Snapshot};
type Request = super::Request<Snapshot>;
type Invocation = super::Invocation<Snapshot, Report>;
type Completed = super::Completed<Snapshot, Report>;
use super::*;
use meden_runtime::{plugin::Definition, process::Control};
use serde_json::json;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
fn request(name: &str) -> Result<Request> {
    let files = BTreeMap::from([("M.lean", "theorem x : True := True.intro\n")]);
    let snapshot = serde_json::from_value(
        json!({"revision":format!("sha256:{}",crate::atlas::digest(&serde_json::to_vec(&files)?)),"files":files,"compilation":{"inputs":{"kind":"lake","fingerprint":"0".repeat(64)},"collection":{"roots":["M.lean"],"external_modules":[],"modules":[],"packages":[],"metadata":{"files":{},"directories":{}}},"graph":{"revision":format!("sha256:{}","1".repeat(64)),"upstream":"fixture","resources_sha256":"fixture","graph":{"metadata":{"projectName":"fixture","leanVersion":"fixture"},"nodes":[{"id":name,"shortName":name,"kind":"theorem","filePath":"M.lean","lineStart":1,"lineEnd":1,"hasSorry":false,"statement":"True","value":null,"axioms":[],"standardAxioms":[],"externalDependencies":[],"semantic":{"version":1,"origin":"elaborated","sentence":"True","refutation":"False","checker":null}}],"edges":[],"externalReferences":[]}}}}),
    )?;
    Ok(Request {
        observation: serde_json::from_value(json!(crate::atlas::digest(name.as_bytes())))?,
        snapshot: Arc::new(snapshot),
    })
}
fn report(name: &str) -> Result<Report> {
    Ok(serde_json::from_value(
        json!({"checks":{name:{"engine":"fixture","status":"not_found","candidates":[]}}}),
    )?)
}
struct Fake {
    phase: Arc<AtomicUsize>,
    role: Role,
    bad_schema: bool,
}
impl Searcher for Fake {
    type Input = Snapshot;
    type Output = Report;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(Definition {
            id: "fixture.search",
            role: self.role,
            version: "1",
            formats: vec!["fixture".into()],
            implementation: b"search fixture",
            configuration: json!({"phase":self.phase.load(Ordering::SeqCst)}),
            input: if self.bad_schema {
                json!({})
            } else {
                input_schema::<Snapshot>()?
            },
            output: output_schema::<Snapshot, Report>()?,
        })?)
    }
    fn search(&self, _: &Context<'_>, _: Invocation) -> Result<Completed> {
        anyhow::bail!("fixture does not dispatch I/O")
    }
}
fn fake(phase: &Arc<AtomicUsize>) -> Fake {
    Fake {
        phase: phase.clone(),
        role: Role::CounterexampleSearch,
        bad_schema: false,
    }
}
#[test]
fn registrations_require_original_role_schemas_and_unique_known_methods() -> Result<()> {
    let phase = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::default();
    assert!(registry
        .register(Fake {
            role: Role::Compiler,
            ..fake(&phase)
        })
        .is_err());
    assert!(registry
        .register(Fake {
            bad_schema: true,
            ..fake(&phase)
        })
        .is_err());
    registry.register(fake(&phase))?;
    assert!(registry.register(fake(&phase)).is_err());
    assert!(registry.select("unknown").is_err());
    phase.store(1, Ordering::SeqCst);
    assert!(registry.select("fixture.search").is_err());
    Ok(())
}
#[test]
fn raw_descriptions_cannot_be_published_and_checks_must_cover_exact_declarations() -> Result<()> {
    let raw = request("x")?;
    assert!(crate::view::SourcePage::from_snapshot(raw.snapshot()).is_err());
    for result in [
        Report {
            checks: BTreeMap::new(),
        },
        report("foreign")?,
    ] {
        assert!(result.project(raw.snapshot().clone()).is_err());
    }
    assert_eq!(
        report("x")?
            .project(raw.snapshot().clone())?
            .graph()
            .graph
            .nodes
            .len(),
        1
    );
    Ok(())
}
#[test]
fn consuming_calls_preserve_original_inputs_stop_on_drift_and_cancellation() -> Result<()> {
    let phase = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::default();
    registry.register(fake(&phase))?;
    let selected = registry.select("fixture.search")?;
    let stop = Arc::new(AtomicBool::new(false));
    let control = Control::new(stop.clone());
    let original = selected.dispatch(&control, request("x")?, |_, input| {
        Ok(input.observe(report("x")?))
    })?;
    assert!(selected
        .dispatch(&control, request("y")?, |_, _| Ok(original))
        .is_err());
    assert!(selected
        .dispatch(&control, request("x")?, |_, input| {
            phase.store(1, Ordering::SeqCst);
            Ok(input.observe(report("x")?))
        })
        .is_err());
    phase.store(0, Ordering::SeqCst);
    assert!(selected
        .dispatch(&control, request("x")?, |_, input| {
            stop.store(true, Ordering::SeqCst);
            Ok(input.observe(report("x")?))
        })
        .is_err());
    let calls = AtomicUsize::new(0);
    assert!(selected
        .dispatch(&control, request("x")?, |_, input| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(input.observe(report("x")?))
        })
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    Ok(())
}
