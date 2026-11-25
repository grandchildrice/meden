//! Port/storage conformance using the real finite data parser, without a solver.
//! Deliberately no Clone implementation or Lean graph on the input type.
use super::*;
use crate::{
    analysis::sources::{Snapshot, Source},
    explanation::cache::{Artifact, Identity, Observation as Retained, Search, Store},
    identity::snapshots::SourceRevision,
};
use meden_runtime::{plugin::Definition, process::Control, verification::finite::Contract};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Digest as _;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct FiniteInput {
    source: Snapshot,
    contract: Contract,
}
impl Source for FiniteInput {
    fn sources(&self) -> &Snapshot {
        &self.source
    }
}
impl compiler::Observation for FiniteInput {
    fn validate(&self) -> Result<()> {
        let text = self
            .source
            .files
            .get("counter.contract.json")
            .context("Missing original finite contract")?;
        anyhow::ensure!(
            self.source.files.len() == 1 && Contract::parse(text)? == self.contract,
            "Decoded finite contract differs from its original source"
        );
        Ok(())
    }
}
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
struct FiniteReport {
    /// Typed candidate inputs only; these are not counterexample evidence.
    candidates: BTreeMap<String, Vec<u8>>,
}
impl Observation<FiniteInput> for FiniteReport {
    fn validate(&self, input: &FiniteInput) -> Result<()> {
        anyhow::ensure!(
            self.candidates.len() == input.contract.properties.len()
                && input.contract.properties.iter().all(|property| {
                    self.candidates.get(&property.name).is_some_and(|values| {
                        values.len()
                            == input
                                .contract
                                .fields
                                .len()
                                .saturating_add(input.contract.parameters.len())
                            && values.iter().all(|v| {
                                u16::from(*v)
                                    < 1u16
                                        .checked_shl(u32::from(input.contract.width))
                                        .unwrap_or(0)
                            })
                    })
                }),
            "Finite candidates must cover the exact original properties and bounds"
        );
        Ok(())
    }
}
struct FiniteSearch(Arc<AtomicUsize>);
impl Searcher for FiniteSearch {
    type Input = FiniteInput;
    type Output = FiniteReport;
    fn descriptor(&self) -> Result<Descriptor> {
        Descriptor::new(Definition {
            id: "fixture.finite-candidates",
            role: Role::CounterexampleSearch,
            version: "1",
            formats: vec!["finite-bitvectors-v1".into()],
            implementation: b"typed finite port conformance; no solver",
            configuration: json!({"phase":self.0.load(Ordering::SeqCst)}),
            input: input_schema::<FiniteInput>()?,
            output: output_schema::<FiniteInput, FiniteReport>()?,
        })
        .map_err(Into::into)
    }
    fn search(
        &self,
        _: &Context<'_>,
        _: Invocation<FiniteInput, FiniteReport>,
    ) -> Result<Completed<FiniteInput, FiniteReport>> {
        anyhow::bail!("Port fixture has no physical solver")
    }
}
fn artifact() -> Result<Artifact<FiniteInput>> {
    let text = include_str!("../../../../verification/examples/finite-counter.contract.json");
    let files = BTreeMap::from([("counter.contract.json".into(), text.into())]);
    Ok(Artifact {
        version: 2,
        compiler: json!({"fixture":"finite parser, not a solver"}),
        snapshot: FiniteInput {
            source: Snapshot {
                revision: SourceRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(
                    &files,
                )?)),
                files,
            },
            contract: Contract::parse(text)?,
        },
    })
}
fn original() -> Result<Retained<FiniteInput>> {
    let artifact = artifact()?;
    Ok(Retained {
        identity: Identity::Compilation(Id::of(&artifact)?),
        reused: false,
        snapshot: Arc::new(artifact.snapshot),
    })
}
fn report() -> FiniteReport {
    FiniteReport {
        candidates: BTreeMap::from([
            ("step".into(), vec![15, 1]),
            ("monotone".into(), vec![0, 1]),
        ]),
    }
}
#[test]
fn finite_input_and_report_use_the_shared_port_without_ast_clones_or_lean_projection() -> Result<()>
{
    let phase = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::default();
    registry.register(FiniteSearch(phase))?;
    let original = original()?;
    let request = Request::new(&original)?;
    assert!(Arc::ptr_eq(&request.snapshot, &original.snapshot));
    assert_eq!(
        serde_json::to_value(&request)?,
        json!({"observation":original.compilation(),"snapshot":original.snapshot()})
    );
    let before = serde_json::to_vec(original.snapshot())?;
    let completed = registry.select("fixture.finite-candidates")?.dispatch(
        &Control::new(Arc::new(AtomicBool::new(false))),
        request,
        |_, call| {
            assert!(Arc::ptr_eq(&call.request.snapshot, &original.snapshot));
            Ok(call.observe(report()))
        },
    )?;
    assert_eq!(
        completed.into_report().candidates.get("step"),
        Some(&vec![15, 1])
    );
    assert_eq!(serde_json::to_vec(original.snapshot())?, before);
    Ok(())
}
#[test]
fn finite_storage_preserves_complete_role_bodies_and_refuses_changed_content() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store(temp.path().to_owned());
    let artifact = artifact()?;
    let original = store.write("finite selection".into(), &artifact)?;
    let restored = store.read::<Artifact<FiniteInput>>(&original)?;
    assert_eq!(
        serde_json::to_vec(&artifact)?,
        serde_json::to_vec(&restored)?
    );
    let search = Search {
        observation: original.clone(),
        searcher: json!({"fixture":"finite candidates"}),
        report: report(),
    };
    let search_id = store.write("finite search".into(), &search)?;
    let restored_search = store.read::<Search<FiniteReport>>(&search_id)?;
    assert_eq!(
        serde_json::to_vec(&search)?,
        serde_json::to_vec(&restored_search)?
    );
    assert_eq!(restored_search.observation, original);
    let retained = Retained {
        identity: Identity::Searched {
            compilation: original.clone(),
            search: search_id,
        },
        reused: true,
        snapshot: Arc::new(restored.snapshot),
    };
    assert!(Request::new(&retained).is_err());
    let mut changed = serde_json::to_value(artifact)?;
    *changed
        .get_mut("snapshot")
        .and_then(|v| v.get_mut("contract"))
        .and_then(|v| v.get_mut("width"))
        .context("missing width")? = json!(3);
    std::fs::write(
        temp.path()
            .join("observations")
            .join(format!("{original}.json")),
        serde_json::to_vec(&changed)?,
    )?;
    assert!(store.read::<Artifact<FiniteInput>>(&original).is_err());
    Ok(())
}
#[test]
fn finite_dispatch_refuses_source_ast_report_drift_foreign_calls_and_stops() -> Result<()> {
    let phase = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::default();
    registry.register(FiniteSearch(phase.clone()))?;
    let selected = registry.select("fixture.finite-candidates")?;
    let stop = Arc::new(AtomicBool::new(false));
    let control = Control::new(stop.clone());
    for source_changed in [false, true] {
        let mut original = original()?;
        let input = Arc::get_mut(&mut original.snapshot).context("shared fixture")?;
        if source_changed {
            input
                .source
                .files
                .get_mut("counter.contract.json")
                .context("missing contract")?
                .push(' ');
        } else {
            input.contract.width = 3;
        }
        let mut called = false;
        assert!(selected
            .dispatch(&control, Request::new(&original)?, |_, call| {
                called = true;
                Ok(call.observe(report()))
            })
            .is_err());
        assert!(!called);
    }
    for invalid in [
        FiniteReport {
            candidates: BTreeMap::new(),
        },
        FiniteReport {
            candidates: BTreeMap::from([
                ("foreign".into(), vec![0, 1]),
                ("step".into(), vec![0, 1]),
            ]),
        },
        FiniteReport {
            candidates: BTreeMap::from([
                ("monotone".into(), vec![16, 1]),
                ("step".into(), vec![0, 1]),
            ]),
        },
    ] {
        assert!(selected
            .dispatch(&control, Request::new(&original()?)?, |_, call| Ok(
                call.observe(invalid)
            ))
            .is_err());
    }
    let previous = selected.dispatch(&control, Request::new(&original()?)?, |_, call| {
        Ok(call.observe(report()))
    })?;
    let mut different = original()?;
    let input = Arc::get_mut(&mut different.snapshot).context("shared fixture")?;
    let text = input
        .source
        .files
        .get_mut("counter.contract.json")
        .context("missing contract")?;
    text.push(' ');
    input.source.revision = SourceRevision::from_digest(sha2::Sha256::digest(serde_json::to_vec(
        &input.source.files,
    )?));
    assert!(selected
        .dispatch(&control, Request::new(&different)?, |_, _| Ok(previous))
        .is_err());
    assert!(selected
        .dispatch(&control, Request::new(&original()?)?, |_, call| {
            phase.store(1, Ordering::SeqCst);
            Ok(call.observe(report()))
        })
        .is_err());
    phase.store(0, Ordering::SeqCst);
    assert!(selected
        .dispatch(&control, Request::new(&original()?)?, |_, call| {
            stop.store(true, Ordering::SeqCst);
            Ok(call.observe(report()))
        })
        .is_err());
    let mut called = false;
    assert!(selected
        .dispatch(&control, Request::new(&original()?)?, |_, call| {
            called = true;
            Ok(call.observe(report()))
        })
        .is_err());
    assert!(!called);
    Ok(())
}
