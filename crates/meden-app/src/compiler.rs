//! Decoded compiler role. Results are observations; no cache state or plugin
//! registration accepts a contract, grants I/O or admits verification evidence.
use crate::{adapters::Context, analysis::sources::Snapshot};
use anyhow::Result;
use meden_runtime::plugin::{Descriptor, Role};
use std::collections::BTreeMap;

/// An owned selection from an explicit application command.
#[derive(Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
pub struct Request {
    paths: Vec<String>,
}

/// Original request and definition, frozen by host registration before dispatch.
pub struct Invocation {
    request: Request,
    definition: Descriptor,
}
impl Invocation {
    /// The original host source selection.
    pub fn paths(&self) -> &[String] {
        self.request.paths()
    }
    /// The immutable definition used for this dispatch and cache lookup.
    pub fn definition(&self) -> &Descriptor {
        &self.definition
    }
    /// Attribute newly decoded data to this original invocation. Cached results
    /// use the checked restoration path instead of being relabelled here.
    pub fn observe(self, snapshot: Snapshot) -> Compiled {
        Compiled {
            reused: false,
            definition: self.definition,
            request: self.request,
            snapshot,
        }
    }
    pub(crate) fn restore(
        &self,
        snapshot: Snapshot,
        definition: &serde_json::Value,
    ) -> Result<Compiled> {
        anyhow::ensure!(
            *definition == serde_json::to_value(&self.definition)?,
            "Cached compiler definition differs from its invocation"
        );
        Ok(Compiled {
            reused: true,
            definition: self.definition.clone(),
            request: self.request.clone(),
            snapshot,
        })
    }
}

/// Completed observation attributed only to the originally selected compiler.
/// Neither its binding nor its snapshot can be replaced by a consumer.
pub struct Compiled {
    reused: bool,
    definition: Descriptor,
    request: Request,
    snapshot: Snapshot,
}
impl Compiled {
    pub(crate) fn reused(&self) -> bool {
        self.reused
    }
    pub(crate) fn into_parts(self) -> (Descriptor, Snapshot) {
        (self.definition, self.snapshot)
    }
}

/// Serialized source-selection shape of the compiler role.
pub fn input_schema() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(Request))?)
}
/// Decoded observation shape; this cannot carry an evidence receipt.
pub fn output_schema() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(Snapshot))?)
}
impl Request {
    /// Bound the request before dispatch. Empty selects configured project roots.
    /// Each backend still checks actual paths through the policy-bound worker.
    pub fn new(paths: Vec<String>) -> Result<Self> {
        anyhow::ensure!(
            paths.len() <= 32
                && paths
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == paths.len()
                && paths
                    .iter()
                    .all(|p| !p.is_empty() && p.len() <= 4096 && !p.contains('\0')),
            "Invalid compiler source selection"
        );
        Ok(Self { paths })
    }
    /// Original selection; implementations cannot substitute a later UI selection.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }
}

/// Host-selected typed compiler implementation. The role never receives shell
/// commands, a model-selected executable or an evidence-admission callback.
pub trait Compiler: Send + Sync {
    /// Actual implementation, configuration, resource and typed schema identity.
    fn descriptor(&self) -> Result<Descriptor>;
    /// Consume the original source request and return a complete decoded result.
    fn compile(&self, context: &Context<'_>, request: Invocation) -> Result<Compiled>;
}

struct Installed {
    compiler: Box<dyn Compiler>,
    definition: Descriptor,
}
impl Installed {
    fn current(&self) -> Result<()> {
        anyhow::ensure!(
            self.compiler.descriptor()? == self.definition,
            "Compiler definition changed after registration"
        );
        Ok(())
    }
}

/// Immutable selected registration, distinct from model/workspace configuration.
pub struct Registration<'a> {
    id: &'a str,
    installed: &'a Installed,
}
impl Registration<'_> {
    /// Stable host registration key; not a verification/checker identity.
    pub fn id(&self) -> &str {
        self.id
    }
    /// Original definition frozen by host registration.
    pub fn definition(&self) -> &Descriptor {
        &self.installed.definition
    }
    /// Consume the original request through the retained host implementation.
    pub fn compile(self, context: &Context<'_>, request: Request) -> Result<Compiled> {
        self.dispatch(request, |compiler, input| compiler.compile(context, input))
    }
    fn dispatch(
        self,
        request: Request,
        call: impl FnOnce(&dyn Compiler, Invocation) -> Result<Compiled>,
    ) -> Result<Compiled> {
        self.installed.current()?;
        let selected = request
            .paths()
            .iter()
            .map(|p| crate::analysis::observe::relative("", p))
            .collect::<Result<Vec<_>>>()?;
        let definition = self.installed.definition.clone();
        let original = request.clone();
        let output = call(
            self.installed.compiler.as_ref(),
            Invocation {
                request,
                definition: definition.clone(),
            },
        )?;
        self.installed.current()?;
        anyhow::ensure!(
            output.definition == definition && output.request == original,
            "Compiler returned an observation from another invocation"
        );
        let snapshot = &output.snapshot;
        anyhow::ensure!(
            selected.iter().all(|p| snapshot.files.contains_key(p)),
            "Compiler observation omitted an originally selected source"
        );
        anyhow::ensure!(
            snapshot.atlas.is_some(),
            "Compiler did not produce a compiled observation"
        );
        anyhow::ensure!(
            snapshot.atlas.as_ref().is_some_and(|g| g
                .graph
                .nodes
                .iter()
                .all(|n| n.semantic.as_ref().is_none_or(|s| s.checker.is_none()))),
            "Compiler returned a search observation"
        );
        crate::view::SourcePage::validate_compilation(snapshot)?;
        Ok(output)
    }
}

/// Explicit host registry. It is never deserialized from repository/model data.
#[derive(Default)]
pub struct Registry {
    compilers: BTreeMap<String, Installed>,
}
impl Registry {
    /// Register once. Replacements and unbounded/malformed IDs are refused.
    pub fn register(&mut self, id: &str, compiler: impl Compiler + 'static) -> Result<()> {
        anyhow::ensure!(
            !id.is_empty()
                && id.len() <= 128
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.')),
            "Invalid compiler registration ID"
        );
        anyhow::ensure!(
            !self.compilers.contains_key(id),
            "Duplicate compiler registration: {id}"
        );
        anyhow::ensure!(
            self.compilers.len() < 64,
            "Compiler registry exceeds 64 implementations"
        );
        let definition = compiler.descriptor()?;
        anyhow::ensure!(
            definition.role() == Role::Compiler
                && definition.id() == id
                && definition.accepts_schema(&input_schema()?)?
                && definition.accepts_output_schema(&output_schema()?)?,
            "Compiler definition does not match its typed role"
        );
        self.compilers.insert(
            id.into(),
            Installed {
                compiler: Box::new(compiler),
                definition,
            },
        );
        Ok(())
    }
    /// Select only a known host implementation. Borrowing freezes registration
    /// against mutation while a selected operation is retained.
    ///
    /// ```compile_fail,E0505
    /// use meden_app::compiler::Registry;
    /// fn keep_original(registry: Registry) -> anyhow::Result<()> {
    ///     let selected = registry.select("installed.compiler")?;
    ///     drop(registry);
    ///     println!("{}", selected.id());
    ///     Ok(())
    /// }
    /// ```
    pub fn select(&self, id: &str) -> Result<Registration<'_>> {
        let (id, installed) = self
            .compilers
            .get_key_value(id)
            .ok_or_else(|| anyhow::anyhow!("Compiler is not registered: {id}"))?;
        installed.current()?;
        Ok(Registration { id, installed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Unsupported;
    impl Compiler for Unsupported {
        fn descriptor(&self) -> Result<Descriptor> {
            definition("compiler.selected", Role::Compiler, 0)
        }
        fn compile(&self, _: &Context<'_>, _: Invocation) -> Result<Compiled> {
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
            output: output_schema()?,
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
        fn descriptor(&self) -> Result<Descriptor> {
            definition(
                "fixture.compiler",
                self.role,
                self.phase.load(std::sync::atomic::Ordering::SeqCst),
            )
        }
        fn compile(&self, _: &Context<'_>, _: Invocation) -> Result<Compiled> {
            anyhow::bail!("Unit fixture has no physical compiler")
        }
    }
    fn observed() -> Result<Snapshot> {
        let files = BTreeMap::from([("Spec.lean".into(), "def value : Nat := 1\n".into())]);
        Ok(Snapshot {
            revision: format!(
                "sha256:{}",
                crate::atlas::digest(&serde_json::to_vec(&files)?)
            ),
            files,
            collection: None,
            atlas: Some(serde_json::from_value(serde_json::json!({
                "revision":format!("sha256:{}", "1".repeat(64)), "upstream":"fixture", "resources_sha256":"fixture",
                "graph":{"metadata":{"projectName":"fixture","leanVersion":"fixture"},"nodes":[{"id":"value","shortName":"value","kind":"definition","filePath":"Spec.lean","lineStart":1,"lineEnd":1,"hasSorry":false,"statement":"Nat","value":"1","axioms":[],"standardAxioms":[],"externalDependencies":[]}],"edges":[],"externalReferences":[]}
            }))?),
        })
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
                    output_schema()?
                },
            })?)
        }
        fn compile(&self, _: &Context<'_>, _: Invocation) -> Result<Compiled> {
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
        let install = |phase| -> Result<Registry> {
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
}
