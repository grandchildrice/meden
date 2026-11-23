//! Candidate search over original compiled inputs. Results cannot admit evidence.
use crate::{
    adapters::Context,
    analysis::sources::Snapshot,
    explanation::{cache::Observation, Check},
};
use anyhow::{Context as _, Result};
use meden_runtime::plugin::{Descriptor, Role};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, schemars::JsonSchema)]
pub struct Request {
    observation: String,
    snapshot: Snapshot,
}
impl Request {
    pub fn new(original: &Observation) -> Self {
        Self {
            observation: original.id().into(),
            snapshot: original.snapshot().clone(),
        }
    }
    pub fn observation(&self) -> &str {
        &self.observation
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    fn identity(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub checks: BTreeMap<String, Check>,
}
impl Report {
    pub(crate) fn project(&self, mut snapshot: Snapshot) -> Result<Snapshot> {
        let graph = snapshot
            .atlas
            .as_mut()
            .context("Search requires a compiled graph")?;
        let expected = graph
            .graph
            .nodes
            .iter()
            .filter(|n| n.semantic.is_some())
            .map(|n| n.id.as_str())
            .collect::<BTreeSet<_>>();
        anyhow::ensure!(
            self.checks.len() == expected.len()
                && self.checks.keys().all(|n| expected.contains(n.as_str()))
                && self.checks.values().all(Check::valid),
            "Missing, foreign or invalid search observations"
        );
        for node in &mut graph.graph.nodes {
            if let Some(description) = &mut node.semantic {
                description.checker = Some(
                    self.checks
                        .get(&node.id)
                        .context("Missing declaration search")?
                        .clone(),
                );
            }
        }
        crate::view::SourcePage::from_snapshot(&snapshot)?;
        Ok(snapshot)
    }
}
pub struct Invocation {
    request: Request,
    definition: Descriptor,
}
impl Invocation {
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn definition(&self) -> &Descriptor {
        &self.definition
    }
    pub fn observe(self, report: Report) -> Completed {
        Completed {
            request: self.request,
            definition: self.definition,
            report,
        }
    }
}
pub struct Completed {
    request: Request,
    definition: Descriptor,
    report: Report,
}
impl Completed {
    pub(crate) fn into_report(self) -> Report {
        self.report
    }
}
pub fn input_schema() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(Request))?)
}
pub fn output_schema() -> Result<serde_json::Value> {
    Ok(serde_json::to_value(schemars::schema_for!(Report))?)
}
pub trait Searcher: Send + Sync {
    fn descriptor(&self) -> Result<Descriptor>;
    fn search(&self, context: &Context<'_>, invocation: Invocation) -> Result<Completed>;
}
struct Installed {
    searcher: Box<dyn Searcher>,
    definition: Descriptor,
}
pub struct Registration<'a> {
    installed: &'a Installed,
}
impl Registration<'_> {
    pub fn definition(&self) -> &Descriptor {
        &self.installed.definition
    }
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.installed.searcher.descriptor()? == self.installed.definition,
            "Search method changed after selection"
        );
        Ok(())
    }
    pub fn search(&self, context: &Context<'_>, request: Request) -> Result<Completed> {
        self.dispatch(context.control, request, |searcher, input| {
            searcher.search(context, input)
        })
    }
    fn dispatch(
        &self,
        control: &meden_runtime::process::Control,
        request: Request,
        call: impl FnOnce(&dyn Searcher, Invocation) -> Result<Completed>,
    ) -> Result<Completed> {
        control.with_active(|| Ok(()))?;
        self.validate()?;
        let original = request.identity()?;
        anyhow::ensure!(
            request.snapshot.atlas.as_ref().is_some_and(|g| g
                .graph
                .nodes
                .iter()
                .all(|n| n.semantic.as_ref().is_none_or(|s| s.checker.is_none()))),
            "Search requires the original unsearched compilation"
        );
        let output = call(
            self.installed.searcher.as_ref(),
            Invocation {
                request,
                definition: self.installed.definition.clone(),
            },
        )?;
        self.validate()?;
        control.with_active(|| Ok(()))?;
        anyhow::ensure!(
            output.definition == self.installed.definition
                && output.request.identity()? == original,
            "Search returned another invocation's observation"
        );
        output.report.project(output.request.snapshot.clone())?;
        Ok(output)
    }
}
#[derive(Default)]
pub struct Registry {
    installed: BTreeMap<String, Installed>,
}
impl Registry {
    pub fn register(&mut self, searcher: impl Searcher + 'static) -> Result<()> {
        let definition = searcher.descriptor()?;
        let id = definition.id().to_owned();
        anyhow::ensure!(
            definition.role() == Role::CounterexampleSearch
                && definition.accepts_schema(&input_schema()?)?
                && definition.accepts_output_schema(&output_schema()?)?,
            "Search definition does not match its typed role"
        );
        anyhow::ensure!(
            self.installed.len() < 64 && !self.installed.contains_key(&id),
            "Duplicate or excessive search registration"
        );
        self.installed.insert(
            id,
            Installed {
                searcher: Box::new(searcher),
                definition,
            },
        );
        Ok(())
    }
    pub fn select(&self, id: &str) -> Result<Registration<'_>> {
        let selected = Registration {
            installed: self
                .installed
                .get(id)
                .context("Search implementation is not registered")?,
        };
        selected.validate()?;
        Ok(selected)
    }
}

#[cfg(test)]
mod tests {
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
            json!({"revision":format!("sha256:{}",crate::atlas::digest(&serde_json::to_vec(&files)?)),"files":files,"collection":null,"atlas":{"revision":format!("sha256:{}","1".repeat(64)),"upstream":"fixture","resources_sha256":"fixture","graph":{"metadata":{"projectName":"fixture","leanVersion":"fixture"},"nodes":[{"id":name,"shortName":name,"kind":"theorem","filePath":"M.lean","lineStart":1,"lineEnd":1,"hasSorry":false,"statement":"True","value":null,"axioms":[],"standardAxioms":[],"externalDependencies":[],"semantic":{"version":1,"origin":"elaborated","sentence":"True","refutation":"False","checker":null}}],"edges":[],"externalReferences":[]}}}),
        )?;
        Ok(Request {
            observation: name.into(),
            snapshot,
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
                    input_schema()?
                },
                output: output_schema()?,
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
    fn raw_descriptions_cannot_be_published_and_checks_must_cover_exact_declarations() -> Result<()>
    {
        let raw = request("x")?;
        assert!(crate::view::SourcePage::from_snapshot(raw.snapshot()).is_err());
        for result in [
            Report {
                checks: BTreeMap::new(),
            },
            report("foreign")?,
        ] {
            assert!(result.project(raw.snapshot.clone()).is_err());
        }
        assert!(report("x")?.project(raw.snapshot)?.atlas.is_some());
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
}
