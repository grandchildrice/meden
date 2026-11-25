//! Complete typed finite observations projected into the common source view.
//! Candidate searches never supply accepted-project evidence or proof status.
use super::{
    compiled::{Compiled, Compiler},
    positions,
    render::{Expression, Renderer, Term},
    search::{Report, Searcher},
};
use crate::{
    analysis::{
        self, repair,
        sources::{self, Source},
    },
    semantic, view,
};
use crate::{compiler::Compiler as _, meaning::Renderer as _, search::Searcher as _};
use anyhow::{Context as _, Result};
use meden_runtime::verification::finite;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
pub(super) fn identity() -> String {
    super::digest(
        &[
            include_bytes!("snapshot.rs").as_slice(),
            include_bytes!("positions.rs").as_slice(),
        ]
        .concat(),
    )
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub(super) compiled: Compiled,
    pub(super) report: Report,
    pub(super) readings: BTreeMap<String, String>,
    pub(super) graph: crate::identity::snapshots::GraphRevision,
    pub(super) binding: Binding,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    compiler: String,
    searcher: String,
    renderer: String,
    context: String,
    worker: PathBuf,
}
impl Binding {
    pub(super) fn new(
        worker: PathBuf,
        compiler: &meden_runtime::plugin::Descriptor,
        searcher: &meden_runtime::plugin::Descriptor,
        renderer: &meden_runtime::plugin::Descriptor,
    ) -> Self {
        Self {
            compiler: compiler.identity().into(),
            searcher: searcher.identity().into(),
            renderer: renderer.identity().into(),
            context: identity(),
            worker,
        }
    }
    pub(super) fn check(&self) -> Result<()> {
        let current = Self::new(
            self.worker.clone(),
            &Compiler.descriptor()?,
            &Searcher {
                worker: self.worker.clone(),
            }
            .descriptor()?,
            &Renderer.descriptor()?,
        );
        anyhow::ensure!(
            self.compiler == current.compiler
                && self.searcher == current.searcher
                && self.renderer == current.renderer
                && self.context == current.context,
            "Originally selected finite analysis implementation or resources changed"
        );
        Ok(())
    }
}
pub(super) struct Declaration {
    pub id: String,
    pub name: String,
    pub file: String,
    pub line: usize,
    pub term: Term,
}
impl Compiled {
    pub(super) fn declarations(&self) -> Result<Vec<Declaration>> {
        let contract_positions = positions::values(
            self.source
                .files
                .get(&self.path)
                .context("Original contract source unavailable")?,
        )?;
        let model_positions = positions::values(
            self.source
                .files
                .get(&self.contract.source_path)
                .context("Original transition source unavailable")?,
        )?;
        let mut nodes = Vec::new();
        for (i, p) in self.contract.properties.iter().enumerate() {
            let name = format!("property.{}", p.name);
            nodes.push(Declaration {
                id: format!("{}::{name}", self.path),
                name,
                file: self.path.clone(),
                line: *contract_positions
                    .get(&format!("/properties/{i}"))
                    .context("Original property position unavailable")?,
                term: Term::Predicate {
                    value: p.statement.clone(),
                },
            });
        }
        for (field, value) in &self.model.transition {
            let name = format!("transition.{field}");
            nodes.push(Declaration {
                id: format!("{}::{name}", self.contract.source_path),
                name,
                file: self.contract.source_path.clone(),
                line: *model_positions
                    .get(&format!("/transition/{field}"))
                    .context("Original transition position unavailable")?,
                term: Term::Word {
                    value: value.clone(),
                },
            });
        }
        Ok(nodes)
    }
    pub(super) fn expression(&self, term: Term) -> Expression {
        Expression {
            width: self.contract.width,
            fields: self.contract.fields.clone(),
            parameters: self.contract.parameters.clone(),
            term,
        }
    }
}
impl Source for Snapshot {
    fn sources(&self) -> &sources::Snapshot {
        &self.compiled.source
    }
}
#[derive(Serialize)]
pub struct Meaning {
    domain: Domain,
    width: u8,
    fields: Vec<String>,
    parameters: Vec<String>,
    simultaneous_transition: finite::Model,
    selected: Term,
}
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Domain {
    UnsignedBitvectorsWithModularArithmeticAndSimultaneousAssignments,
}
impl semantic::Observation for Snapshot {
    type Context = Meaning;
    fn graph_revision(&self) -> Option<&crate::identity::snapshots::GraphRevision> {
        Some(&self.graph)
    }
    fn context(&self, target: &semantic::Target) -> Result<Option<Meaning>> {
        let d = self
            .compiled
            .declarations()?
            .into_iter()
            .find(|d| d.file == target.file && d.name == target.name && d.line == target.line)
            .context("Selected finite declaration differs from its original source location")?;
        Ok(Some(Meaning {
            domain: Domain::UnsignedBitvectorsWithModularArithmeticAndSimultaneousAssignments,
            width: self.compiled.contract.width,
            fields: self.compiled.contract.fields.clone(),
            parameters: self.compiled.contract.parameters.clone(),
            simultaneous_transition: self.compiled.model.clone(),
            selected: d.term,
        }))
    }
}
impl analysis::Observation for Snapshot {
    fn check(&self, context: &crate::adapters::Context<'_>) -> Result<analysis::Freshness> {
        context.control.with_active(|| Ok(()))?;
        self.binding.check()?;
        Ok(analysis::Freshness::Current)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repair {
    binding: Binding,
    paths: Vec<String>,
}
impl repair::Context for Repair {
    fn check(&self, context: &crate::adapters::Context<'_>) -> Result<()> {
        context.control.with_active(|| Ok(()))?;
        self.binding.check()
    }
    fn paths(&self) -> Option<&[String]> {
        Some(&self.paths)
    }
    fn relocatable(&self) -> bool {
        true
    }
}
impl repair::Observation for Snapshot {
    type Repair = Repair;
    fn repair_context(&self) -> Result<Repair> {
        Ok(Repair {
            binding: self.binding.clone(),
            paths: vec![self.compiled.path.clone()],
        })
    }
    fn refreshed_input(&self, original: &semantic::Input) -> Result<semantic::Input> {
        let node = self
            .compiled
            .declarations()?
            .into_iter()
            .find(|d| d.file == original.target.file && d.name == original.target.name)
            .context("Repair applied; original finite declaration is missing")?;
        let mut input = original.clone();
        input.source_revision = self.compiled.source.revision.clone();
        input.graph_revision = Some(self.graph.clone());
        input.target.line = node.line;
        Ok(input)
    }
}
fn outputs_word(w: &finite::Word, fields: &mut BTreeSet<String>) {
    use finite::Word;
    match w {
        Word::Output { field } => {
            fields.insert(field.clone());
        }
        Word::Add { left, right } | Word::Sub { left, right } => {
            outputs_word(left, fields);
            outputs_word(right, fields);
        }
        Word::If { condition, yes, no } => {
            outputs_predicate(condition, fields);
            outputs_word(yes, fields);
            outputs_word(no, fields);
        }
        _ => {}
    }
}
fn outputs_predicate(p: &finite::Predicate, fields: &mut BTreeSet<String>) {
    use finite::Predicate;
    match p {
        Predicate::Equal { left, right }
        | Predicate::Less { left, right }
        | Predicate::LessEqual { left, right } => {
            outputs_word(left, fields);
            outputs_word(right, fields);
        }
        Predicate::Not { argument } => outputs_predicate(argument, fields),
        Predicate::And { left, right } | Predicate::Or { left, right } => {
            outputs_predicate(left, fields);
            outputs_predicate(right, fields);
        }
        _ => {}
    }
}
impl Snapshot {
    fn checker(&self, name: &str) -> Result<crate::explanation::Check> {
        let key = name
            .strip_prefix("property.")
            .context("Finite property name unavailable")?;
        let outcome = self
            .report
            .properties
            .get(key)
            .context("Finite property observation unavailable")?;
        let (status, candidates) = match outcome {
            finite::Observation::NotFound => ("not_found", vec![]),
            finite::Observation::Counterexample { witness } => {
                let values = self
                    .compiled
                    .contract
                    .fields
                    .iter()
                    .zip(&witness.before)
                    .map(|(f, v)| format!("input.{f} = {v}"))
                    .chain(
                        self.compiled
                            .contract
                            .parameters
                            .iter()
                            .zip(&witness.arguments)
                            .map(|(f, v)| format!("argument.{f} = {v}")),
                    )
                    .chain(
                        self.compiled
                            .contract
                            .fields
                            .iter()
                            .zip(&witness.after)
                            .map(|(f, v)| format!("output.{f} = {v}")),
                    )
                    .collect::<Vec<_>>();
                ("counterexample", vec![values.join("; ")])
            }
        };
        let bits = self
            .compiled
            .contract
            .fields
            .len()
            .saturating_add(self.compiled.contract.parameters.len())
            .saturating_mul(usize::from(self.compiled.contract.width));
        let assignments = 1u64
            .checked_shl(u32::try_from(bits)?)
            .context("Finite input domain exceeds bounds")?;
        Ok(crate::explanation::Check {
            engine: "Z3 QF_BV / complete finite evaluator".into(),
            status: status.into(),
            candidates,
            diagnostic: Some(self.compiled.contract.scope()),
            seed: None,
            num_inst: Some(assignments),
            num_retries: None,
            max_size: None,
            mode: None,
        })
    }
    fn node(&self, d: Declaration) -> Result<view::Node> {
        let (kind, statement, refutation, checker) = match &d.term {
            Term::Predicate { value } => (
                view::Kind::Theorem,
                value.text(),
                format!("not ({})", value.text()),
                self.checker(&d.name)?,
            ),
            Term::Word { value } => (
                view::Kind::Definition,
                value.text(),
                String::new(),
                crate::explanation::Check {
                    engine: "Finite data typing".into(),
                    status: "not_proposition".into(),
                    candidates: vec![],
                    diagnostic: None,
                    seed: None,
                    num_inst: None,
                    num_retries: None,
                    max_size: None,
                    mode: None,
                },
            ),
        };
        let sentence = self
            .readings
            .get(&d.id)
            .context("Original finite reading unavailable")?
            .clone();
        Ok(view::Node {
            id: d.id,
            name: d.name.clone(),
            label: d.name,
            kind,
            is_main_theorem: false,
            statement,
            source: view::Location {
                file: d.file,
                line: Some(u32::try_from(d.line)?),
            },
            proof_status: view::ProofHint::Unknown,
            axioms: vec![],
            semantic: Some(crate::explanation::Explanation {
                version: 1,
                origin: "elaborated".into(),
                sentence,
                refutation,
                checker,
                intermediate: None,
                renderer: Some("finite.english".into()),
                rendering_error: None,
            }),
        })
    }
}
impl view::Projection for Snapshot {
    fn page(&self) -> Result<view::SourcePage> {
        use crate::{compiler::Observation as _, search::Observation as _};
        self.compiled.validate()?;
        self.report.validate(&self.compiled)?;
        let original = self.compiled.declarations()?;
        anyhow::ensure!(
            self.readings.keys().collect::<BTreeSet<_>>()
                == original.iter().map(|d| &d.id).collect(),
            "Finite readings are incomplete or contain foreign nodes"
        );
        let mut declarations = Vec::new();
        let mut edges = Vec::new();
        for d in original {
            if let Term::Predicate { value } = &d.term {
                let mut fields = BTreeSet::new();
                outputs_predicate(value, &mut fields);
                for field in fields {
                    edges.push(view::Edge {
                        source: d.id.clone(),
                        target: format!(
                            "{}::transition.{field}",
                            self.compiled.contract.source_path
                        ),
                        site: view::Site::Value,
                    });
                }
            }
            declarations.push(self.node(d)?);
        }
        Ok(view::SourcePage {
            revision: self.compiled.source.revision.clone(),
            files: self
                .compiled
                .source
                .files
                .iter()
                .map(|(p, s)| Ok((p.clone(), view::SourceText::new(s.clone())?)))
                .collect::<Result<_>>()?,
            roots: vec![self.compiled.path.clone()],
            graph: Some(view::Graph {
                revision: self.graph.clone(),
                declarations,
                edges,
            }),
            description: self.compiled.contract.scope(),
            external: vec![],
            compiled: true,
            inputs_revision: None,
        })
    }
}
