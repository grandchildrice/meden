use super::*;
use crate::adapters::lean::Snapshot;
use anyhow::Context as _;
use sha2::Digest as _;
struct Unsupported;
impl Compiler for Unsupported {
    type Output = Snapshot;
    fn descriptor(&self) -> Result<Descriptor> {
        definition("compiler.selected", Role::Compiler, 0)
    }
    fn compile(&self, _: &Context<'_>, _: Invocation<Snapshot>) -> Result<Compiled<Snapshot>> {
        anyhow::bail!("unsupported fixture")
    }
}
fn definition(id: &str, role: Role, version: usize) -> Result<Descriptor> {
    Ok(Descriptor::new(meden_runtime::plugin::Definition {
        id,
        role,
        version: "1",
        formats: vec!["fixture".into()],
        implementation: b"fixture compiler",
        configuration: serde_json::json!({"version":version}),
        input: input_schema()?,
        output: output_schema::<Snapshot>()?,
    })?)
}

#[test]
fn registration_requires_a_known_unique_host_key() -> Result<()> {
    let mut registry = Registry::default();
    assert!(registry.select("not.installed").is_err());
    for invalid in ["", "../tool", "command --execute", "provider:unselected"] {
        assert!(registry.register(invalid, Unsupported).is_err());
    }
    registry.register("compiler.selected", Unsupported)?;
    assert!(registry.register("compiler.selected", Unsupported).is_err());
    assert_eq!(
        registry.select("compiler.selected")?.id(),
        "compiler.selected"
    );
    Ok(())
}

#[test]
fn decoded_selection_is_bounded_and_keeps_exact_original_paths() -> Result<()> {
    let input = vec!["Nested/Source.lean".into()];
    assert_eq!(Request::new(input.clone())?.paths(), input);
    assert!(Request::new(Vec::new()).is_ok());
    assert!(Request::new(vec![String::new()]).is_err());
    assert!(Request::new(vec!["bad\0path".into()]).is_err());
    assert!(Request::new(vec!["x".repeat(4097)]).is_err());
    assert!(Request::new(vec!["Source.lean".into(); 33]).is_err());
    assert!(Request::new(vec!["Source.lean".into(); 2]).is_err());
    Ok(())
}

struct Changing {
    phase: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    role: Role,
}
impl Compiler for Changing {
    type Output = Snapshot;
    fn descriptor(&self) -> Result<Descriptor> {
        definition(
            "fixture.compiler",
            self.role,
            self.phase.load(std::sync::atomic::Ordering::SeqCst),
        )
    }
    fn compile(&self, _: &Context<'_>, _: Invocation<Snapshot>) -> Result<Compiled<Snapshot>> {
        anyhow::bail!("Unit fixture has no physical compiler")
    }
}
fn observed() -> Result<Snapshot> {
    let files = BTreeMap::from([("Spec.lean".into(), "def value : Nat := 1\n".into())]);
    Ok(crate::adapters::lean::snapshot::fixture(
        crate::analysis::sources::Snapshot {
            revision: crate::identity::snapshots::SourceRevision::from_digest(
                sha2::Sha256::digest(serde_json::to_vec(&files)?),
            ),
            files,
        },
        serde_json::from_value(serde_json::json!({
            "revision":format!("sha256:{}", "1".repeat(64)), "upstream":"fixture", "resources_sha256":"fixture",
            "graph":{"metadata":{"projectName":"fixture","leanVersion":"fixture"},"nodes":[{"id":"value","shortName":"value","kind":"definition","filePath":"Spec.lean","lineStart":1,"lineEnd":1,"hasSorry":false,"statement":"Nat","value":"1","axioms":[],"standardAxioms":[],"externalDependencies":[]}],"edges":[],"externalReferences":[]}
        }))?,
    ))
}
#[test]
fn original_definition_is_required_before_and_after_dispatch() -> Result<()> {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let phase = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::default();
    assert!(registry
        .register(
            "fixture.compiler",
            Changing {
                phase: phase.clone(),
                role: Role::Verifier
            }
        )
        .is_err());
    registry.register(
        "fixture.compiler",
        Changing {
            phase: phase.clone(),
            role: Role::Compiler,
        },
    )?;
    let selected = registry.select("fixture.compiler")?;
    let original = selected.definition().clone();
    phase.store(1, Ordering::SeqCst);
    assert!(registry.select("fixture.compiler").is_err());
    let calls = AtomicUsize::new(0);
    assert!(selected
        .dispatch(Request::new(vec!["Spec.lean".into()])?, |_, input| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(input.observe(observed()?))
        })
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    phase.store(0, Ordering::SeqCst);
    assert!(registry
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["Spec.lean".into()])?, |_, input| {
            assert_eq!(input.paths(), &["Spec.lean"]);
            assert_eq!(input.definition(), &original);
            phase.store(2, Ordering::SeqCst);
            Ok(input.observe(observed()?))
        })
        .is_err());
    phase.store(0, Ordering::SeqCst);
    let output = registry
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["Spec.lean".into()])?, |_, input| {
            Ok(input.observe(observed()?))
        })?;
    assert_eq!(output.into_parts().0, original);
    assert!(registry
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["Other.lean".into()])?, |_, input| Ok(
            input.observe(observed()?)
        ))
        .is_err());
    Ok(())
}

struct WrongSchema {
    output: bool,
}
impl Compiler for WrongSchema {
    type Output = Snapshot;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(meden_runtime::plugin::Definition {
            id: "wrong.schema",
            role: Role::Compiler,
            version: "1",
            formats: vec!["fixture".into()],
            implementation: b"fixture",
            configuration: serde_json::json!({}),
            input: if self.output {
                input_schema()?
            } else {
                serde_json::json!({"type":"string"})
            },
            output: if self.output {
                serde_json::json!({"type":"string"})
            } else {
                output_schema::<Snapshot>()?
            },
        })?)
    }
    fn compile(&self, _: &Context<'_>, _: Invocation<Snapshot>) -> Result<Compiled<Snapshot>> {
        anyhow::bail!("Wrong schema must not dispatch")
    }
}
#[test]
fn both_port_schemas_are_checked_before_installation() {
    for output in [false, true] {
        assert!(Registry::default()
            .register("wrong.schema", WrongSchema { output })
            .is_err());
    }
}

#[test]
fn a_completed_observation_from_another_invocation_cannot_be_relabelled() -> Result<()> {
    use std::sync::{atomic::AtomicUsize, Arc};
    let install = |phase| -> Result<Registry<Snapshot>> {
        let mut registry = Registry::default();
        registry.register(
            "fixture.compiler",
            Changing {
                phase: Arc::new(AtomicUsize::new(phase)),
                role: Role::Compiler,
            },
        )?;
        Ok(registry)
    };
    let original = install(0)?;
    let replacement = install(1)?;
    let foreign = replacement
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["Spec.lean".into()])?, |_, input| {
            Ok(input.observe(observed()?))
        })?;
    assert!(original
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["Spec.lean".into()])?, |_, _| Ok(foreign))
        .is_err());
    let other_request = original
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["./Spec.lean".into()])?, |_, input| {
            Ok(input.observe(observed()?))
        })?;
    assert!(original
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["Spec.lean".into()])?, |_, _| Ok(
            other_request
        ))
        .is_err());
    let different_definition = replacement.select("fixture.compiler")?.definition().clone();
    assert!(original
        .select("fixture.compiler")?
        .dispatch(Request::new(vec!["Spec.lean".into()])?, |_, input| input
            .restore(
                observed()?,
                &serde_json::to_value(&different_definition)?
            ))
        .is_err());
    Ok(())
}

#[derive(serde::Serialize, schemars::JsonSchema)]
struct FiniteObservation {
    source: sources::Snapshot,
    contract: meden_runtime::verification::finite::Contract,
    #[serde(skip)]
    #[schemars(skip)]
    projections: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl sources::Source for FiniteObservation {
    fn sources(&self) -> &sources::Snapshot {
        &self.source
    }
}
impl Observation for FiniteObservation {
    fn validate(&self) -> Result<()> {
        self.contract.validate()?;
        let text = self
            .source
            .files
            .get("Spec.contract.json")
            .ok_or_else(|| anyhow::anyhow!("Missing original finite source"))?;
        anyhow::ensure!(
            meden_runtime::verification::finite::Contract::parse(text)? == self.contract,
            "Finite AST differs from original source"
        );
        Ok(())
    }
}
impl crate::view::Projection for FiniteObservation {
    fn page(&self) -> Result<crate::view::SourcePage> {
        use crate::view::{Graph, Kind, Location, Node, ProofHint, SourceText};
        self.projections
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let revision =
            crate::identity::snapshots::GraphRevision::from_digest(sha2::Sha256::digest(
                serde_json::to_vec(&(&self.source.revision, &self.contract))?,
            ));
        Ok(crate::view::SourcePage {
            revision: self.source.revision.clone(),
            files: self
                .source
                .files
                .iter()
                .map(|(p, s)| Ok((p.clone(), SourceText::new(s.clone())?)))
                .collect::<Result<_>>()?,
            roots: self.source.files.keys().cloned().collect(),
            graph: Some(Graph {
                revision,
                declarations: self
                    .contract
                    .properties
                    .iter()
                    .map(|p| Node {
                        id: p.name.clone(),
                        name: p.name.clone(),
                        label: p.name.clone(),
                        kind: Kind::Theorem,
                        is_main_theorem: false,
                        statement: p.statement.text(),
                        source: Location {
                            file: "Spec.contract.json".into(),
                            line: None,
                        },
                        proof_status: ProofHint::Unknown,
                        axioms: Vec::new(),
                        semantic: None,
                    })
                    .collect(),
                edges: Vec::new(),
            }),
            description: "Validated finite contract declarations; no solver has run.".into(),
            external: Vec::new(),
            compiled: true,
            inputs_revision: None,
        })
    }
}
struct FiniteDecoder;
impl Compiler for FiniteDecoder {
    type Output = FiniteObservation;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(meden_runtime::plugin::Definition {
            id: "finite.decoder",
            role: Role::Compiler,
            version: "1",
            formats: vec!["finite-bitvectors-v1".into()],
            implementation: b"conformance: actual bounded finite AST decoder, not solver execution",
            configuration: serde_json::json!({}),
            input: input_schema()?,
            output: output_schema::<FiniteObservation>()?,
        })?)
    }
    fn compile(
        &self,
        _: &Context<'_>,
        _: Invocation<FiniteObservation>,
    ) -> Result<Compiled<FiniteObservation>> {
        anyhow::bail!(
            "Unit conformance supplies original source data; no physical worker is exercised"
        )
    }
}
fn finite_source(text: &str) -> Result<sources::Snapshot> {
    let files = BTreeMap::from([("Spec.contract.json".into(), text.into())]);
    Ok(sources::Snapshot {
        revision: crate::identity::snapshots::SourceRevision::from_digest(sha2::Sha256::digest(
            serde_json::to_vec(&files)?,
        )),
        files,
    })
}
#[test]
fn a_different_decoded_ast_uses_the_same_port_and_frozen_view_without_backend_branches(
) -> Result<()> {
    use crate::view::{PreparedSource, SourcePage};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let text = include_str!("../../../../verification/examples/finite-counter.contract.json");
    let source = finite_source(text)?;
    let count = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::<FiniteObservation>::default();
    registry.register("finite.decoder", FiniteDecoder)?;
    let definition = registry.select("finite.decoder")?.definition().clone();
    let compiled = registry.select("finite.decoder")?.dispatch(
        Request::new(vec!["Spec.contract.json".into()])?,
        |_, call| {
            Ok(call.observe(FiniteObservation {
                source,
                contract: meden_runtime::verification::finite::Contract::parse(text)?,
                projections: count.clone(),
            }))
        },
    )?;
    let (original, output) = compiled.into_parts();
    assert_eq!(definition, original);
    let prepared = PreparedSource::new(output)?;
    let clone = prepared.clone();
    assert!(std::ptr::eq(prepared.snapshot(), clone.snapshot()));
    let id = prepared.publication().reference().revision.clone();
    let mut owner = crate::view::sources::Sources::default();
    owner.publish(prepared);
    for _ in 0..8 {
        let bytes = owner
            .bytes(&id)
            .ok_or_else(|| anyhow::anyhow!("Missing generic source view"))?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        let page: SourcePage =
            serde_json::from_value(value.get("data").context("Complete generic view")?.clone())?;
        assert!(page.compiled);
        assert_eq!(page.graph.as_ref().map(|g| g.declarations.len()), Some(2));
        assert_eq!(
            serde_json::to_value(owner.index())?
                .pointer("/files/Spec.contract.json/revision")
                .context("Original file reference")?,
            &serde_json::to_value(&id)?
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(owner.file("Spec.contract.json").is_some());
    owner.clear_current();
    owner.forget_file("Spec.contract.json");
    assert!(owner.bytes(&id).is_none());
    Ok(())
}
#[test]
fn common_source_checks_and_backend_ast_checks_remain_independent() -> Result<()> {
    use std::sync::{atomic::AtomicUsize, Arc};
    let text = include_str!("../../../../verification/examples/finite-counter.contract.json");
    let mut registry = Registry::<FiniteObservation>::default();
    registry.register("finite.decoder", FiniteDecoder)?;
    for fault in 0..4 {
        let mut source = finite_source(text)?;
        let mut contract = meden_runtime::verification::finite::Contract::parse(text)?;
        match fault {
            0 => {
                source.revision = crate::identity::snapshots::SourceRevision::from_digest(
                    sha2::Sha256::digest(b"foreign input"),
                );
            }
            1 => {
                let original = source
                    .files
                    .remove("Spec.contract.json")
                    .ok_or_else(|| anyhow::anyhow!("source"))?;
                source
                    .files
                    .insert("../Spec.contract.json".into(), original);
                source.revision = crate::identity::snapshots::SourceRevision::from_digest(
                    sha2::Sha256::digest(serde_json::to_vec(&source.files)?),
                );
            }
            2 => contract.width = 0,
            _ => contract.description = "Unrelated but well-formed metadata".into(),
        }
        assert!(registry
            .select("finite.decoder")?
            .dispatch(
                Request::new(vec!["Spec.contract.json".into()])?,
                |_, call| Ok(call.observe(FiniteObservation {
                    source,
                    contract,
                    projections: Arc::new(AtomicUsize::new(0))
                }))
            )
            .is_err());
    }
    Ok(())
}
