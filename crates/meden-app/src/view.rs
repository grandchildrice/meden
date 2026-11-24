//! Read-only prepared source and graph projection. This data neither accepts a
//! contract nor carries a verification receipt. Backend observations and current
//! accepted-project evidence remain separate application contexts.
use crate::{analysis::sources::Snapshot, explanation::Explanation};
use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Candidate meaning jobs and retained observations, separate from evidence.
pub mod analyses;
/// Original delivery descriptions and lossless host review references.
pub mod delivery;
/// Complete typed directory observations, independent of inventory/compiler jobs.
pub mod directory;
/// Original reviewed intent and observed project evidence, without authority.
pub mod evidence;
/// Bounded complete file availability, independent of compiler support.
pub mod file;
/// Revision-bound transcript/work data, separate from status polling.
pub mod history;
/// Prepared generic navigation inventory and chunks, separate from source text.
pub mod inventory;
/// Bounded preparation observations and exact diagnostic queries.
pub mod jobs;
/// Exact immutable static-preview observations, separate from browser authority.
pub mod preview;
/// Bounded, immutable publication framing shared by read-model owners.
pub mod publication;
/// Immutable reader retention, separate from semantic graphs.
pub mod readers;
/// Lossless browser projection of an original host repair review.
pub mod repairs;
/// Canonical lossless counters used by native view and command adapters.
pub mod sequence;
/// Borrowed native session projection, independent of controller authority.
pub mod session;
/// Retention and exact-revision queries for prepared source/graph views.
pub mod sources;
/// Actual project verification progress, independent of kernel job authority.
pub mod verification_jobs;

/// A retained UTF-8 input, byte-bounded before publication. The generated wire
/// validator also limits string length; it does not replace this byte check.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct SourceText(#[schemars(length(max = 1_048_576))] String);
impl SourceText {
    /// Refuse oversized content without constructing a truncated source view.
    pub fn new(value: String) -> Result<Self> {
        anyhow::ensure!(value.len() <= 1_048_576, "Source view exceeds 1 MiB");
        Ok(Self(value))
    }
}
impl std::ops::Deref for SourceText {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for SourceText {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Source location in the exact retained input.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Location {
    /// Logical workspace-relative source path.
    #[schemars(length(min = 1, max = 4096))]
    pub file: String,
    /// One-based original line.
    #[schemars(range(min = 1))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}
/// Presentation category, independent of the checker implementation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Formal claim, without an inferred proof receipt.
    Theorem,
    /// Named definition or data declaration.
    Definition,
    /// Declared assumption.
    Axiom,
}
/// Only source/compiler observations. Successful verification is not inferred.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProofHint {
    /// No admitted verification is represented by this graph.
    Unknown,
    /// Backend explicitly reported a missing proof.
    Unproved,
}
/// Prepared graph node. It cannot assert a proved/bounded/refined receipt.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    /// Stable backend/source declaration identity.
    #[schemars(length(min = 1, max = 8192))]
    pub id: String,
    /// Original formal name used for source/meaning selection.
    #[schemars(length(min = 1, max = 1000))]
    pub name: String,
    /// Short display label.
    #[schemars(length(max = 1000))]
    pub label: String,
    /// Presentation category.
    pub kind: Kind,
    /// Explicit metadata about a project target, never a proof status.
    pub is_main_theorem: bool,
    /// Bounded original statement or lexical header.
    #[schemars(length(max = 100_000))]
    pub statement: String,
    /// Original source location.
    pub source: Location,
    /// Observation only; admitted evidence is queried independently.
    pub proof_status: ProofHint,
    /// Original assumptions reported by the backend.
    #[schemars(length(max = 1000))]
    pub axioms: Vec<String>,
    /// A prepared deterministic reading/search observation, if supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic: Option<Explanation>,
}
/// Edge scope used by generic dependency navigation.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Site {
    /// Statement or type dependency.
    Type,
    /// Implementation/proof-body dependency.
    Value,
    /// Retain an unclassified dependency conservatively.
    Unknown,
}

/// Immutable publication binding: retain the original backend snapshot for
/// explicit analysis, serialize only the materialized presentation projection.
#[derive(Clone)]
pub struct PreparedSource {
    snapshot: std::sync::Arc<Snapshot>,
    publication: publication::Publication,
}
impl PreparedSource {
    /// Materialize and validate once before the GUI publishes a source view.
    pub fn new(snapshot: Snapshot) -> Result<Self> {
        let _timing = meden_runtime::metrics::Span::start("analysis.materialize_view");
        let page = SourcePage::from_snapshot(&snapshot)?;
        let publication = publication::Publication::new(b"meden.source.view.v1\0", &page)?;
        Ok(Self {
            snapshot: std::sync::Arc::new(snapshot),
            publication,
        })
    }
    /// Borrow the exact original backend inputs; view data cannot replace them.
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    /// Complete frozen presentation, separately identified from input sources.
    pub fn publication(&self) -> &publication::Publication {
        &self.publication
    }
}
impl std::ops::Deref for PreparedSource {
    type Target = Snapshot;
    fn deref(&self) -> &Snapshot {
        &self.snapshot
    }
}
impl Serialize for PreparedSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.publication.serialize(serializer)
    }
}
/// Validated cross-reference within this graph.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    /// Referencing node ID.
    pub source: String,
    /// Referenced node ID.
    pub target: String,
    /// Type, implementation, or conservatively unknown dependency.
    pub site: Site,
}
/// One backend-independent graph; cycles are valid and preserved.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    /// Exact graph/artifact revision.
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub revision: String,
    /// Display nodes with source linkage.
    #[schemars(length(max = 5000))]
    pub declarations: Vec<Node>,
    /// Complete local cross-references in the prepared scope.
    #[schemars(length(max = 100_000))]
    pub edges: Vec<Edge>,
}
/// Read-only source page, materialized before publication. Raw prover records
/// and recorded-workspace conversation/evidence fields are absent.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourcePage {
    /// Revision of the exact retained source map.
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub revision: String,
    /// Bounded UTF-8 inputs. Other availability states are a separate projection.
    pub files: BTreeMap<String, SourceText>,
    /// Explicit selected roots rather than backend package/configuration internals.
    pub roots: Vec<String>,
    /// Prepared graph, absent when no supported graph was produced.
    pub graph: Option<Graph>,
    /// Human-readable backend/scope attribution prepared by the host.
    pub description: String,
    /// Explicit external boundaries, not invented declaration nodes.
    pub external: Vec<String>,
    /// Whether a semantic/compiler observation or lexical navigation was prepared.
    pub compiled: bool,
    /// Exact prepared compiler-input binding when a backend provides one.
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    pub inputs_revision: Option<String>,
}

impl SourcePage {
    /// Project exact immutable inputs. This performs no compiler/model/file I/O.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self> {
        Self::project(snapshot, true)
    }
    pub(crate) fn validate_compilation(snapshot: &Snapshot) -> Result<()> {
        Self::project(snapshot, false).map(|_| ())
    }
    fn project(snapshot: &Snapshot, searched: bool) -> Result<Self> {
        let (graph, description) = if let Some(extraction) = &snapshot.atlas {
            let nodes = extraction
                .graph
                .nodes
                .iter()
                .map(|node| {
                    Ok(Node {
                        id: node.id.clone(),
                        name: node.id.clone(),
                        label: node.short_name.clone(),
                        kind: kind(&node.kind),
                        is_main_theorem: node.meta.as_ref().is_some_and(|m| m.is_main_theorem),
                        statement: node.statement.clone(),
                        source: Location {
                            file: node.file_path.clone(),
                            line: node.line_start.map(u32::try_from).transpose()?,
                        },
                        proof_status: if node.has_sorry {
                            ProofHint::Unproved
                        } else {
                            ProofHint::Unknown
                        },
                        axioms: node
                            .axioms
                            .iter()
                            .chain(&node.standard_axioms)
                            .cloned()
                            .collect(),
                        semantic: if searched {
                            node.semantic
                                .clone()
                                .map(crate::explanation::Description::complete)
                                .transpose()?
                        } else {
                            None
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let edges = extraction
                .graph
                .edges
                .iter()
                .map(|edge| Edge {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    site: match edge.kind.as_str() {
                        "theorem_type_to_definition"
                        | "theorem_type_to_theorem"
                        | "definition_type_to_definition"
                        | "definition_type_to_theorem" => Site::Type,
                        "theorem_value_to_definition"
                        | "theorem_value_to_theorem"
                        | "definition_value_to_definition"
                        | "definition_value_to_theorem" => Site::Value,
                        _ => Site::Unknown,
                    },
                })
                .collect();
            (Some(Graph { revision: extraction.revision.clone(), declarations: nodes, edges }), format!("Compiled dependencies from Lean Atlas (Lean {}). Selected observed modules; external libraries remain unexpanded. Contract and proof acceptance are unchanged.", extraction.graph.metadata.lean_version))
        } else {
            (Some(lexical(snapshot)?), "Lexical reference candidates from retained source; no compiler or verification result is inferred.".into())
        };
        let page = Self {
            revision: snapshot.revision.clone(),
            files: snapshot
                .files
                .iter()
                .map(|(path, text)| Ok((path.clone(), SourceText::new(text.clone())?)))
                .collect::<Result<_>>()?,
            roots: snapshot.collection.as_ref().map_or_else(
                || snapshot.files.keys().cloned().collect(),
                |c| c.roots.clone(),
            ),
            external: snapshot
                .collection
                .as_ref()
                .map_or_else(Vec::new, |c| c.external_modules.clone()),
            inputs_revision: snapshot
                .collection
                .as_ref()
                .and_then(|c| c.prepared.clone()),
            compiled: snapshot.atlas.is_some(),
            graph,
            description,
        };
        page.validate()?;
        Ok(page)
    }
    /// Check bounds, identities, original source ranges and graph cross-references.
    pub fn validate(&self) -> Result<()> {
        let revision = format!(
            "sha256:{}",
            crate::atlas::digest(&serde_json::to_vec(&self.files)?)
        );
        anyhow::ensure!(
            self.revision == revision
                && !self.files.is_empty()
                && self.files.len() <= 32
                && self.files.keys().all(|p| !p.is_empty() && p.len() <= 4096)
                && self.files.values().all(|s| s.len() <= 1_048_576)
                && self.files.values().map(|s| s.len()).sum::<usize>() <= 16_777_216,
            "Source projection binding/bounds are invalid"
        );
        anyhow::ensure!(
            self.roots.iter().all(|p| self.files.contains_key(p)),
            "Source root is outside the retained inputs"
        );
        if let Some(graph) = &self.graph {
            anyhow::ensure!(
                graph.declarations.len() <= 5000 && graph.edges.len() <= 100_000,
                "Graph projection exceeds bounds"
            );
            let mut ids = BTreeSet::new();
            let lines = self
                .files
                .iter()
                .map(|(path, text)| (path, text.lines().count()))
                .collect::<BTreeMap<_, _>>();
            for node in &graph.declarations {
                anyhow::ensure!(
                    !node.id.is_empty()
                        && node.id.len() <= 8192
                        && ids.insert(&node.id)
                        && !node.name.is_empty()
                        && node.name.len() <= 1000
                        && node.label.len() <= 1000
                        && node.statement.len() <= 100_000
                        && node.axioms.len() <= 1000
                        && node.axioms.iter().all(|s| s.len() <= 1000),
                    "Invalid graph declaration"
                );
                let line_count = lines
                    .get(&node.source.file)
                    .context("Graph source is outside the retained inputs")?;
                anyhow::ensure!(
                    node.source.line.is_none_or(|line| line > 0
                        && usize::try_from(line).is_ok_and(|line| line <= *line_count))
                        && node
                            .semantic
                            .as_ref()
                            .is_none_or(Explanation::<crate::explanation::Check>::valid),
                    "Invalid graph source or explanation"
                );
            }
            anyhow::ensure!(
                graph
                    .edges
                    .iter()
                    .all(|edge| ids.contains(&edge.source) && ids.contains(&edge.target)),
                "Graph dependency target is missing"
            );
        }
        Ok(())
    }
}

fn kind(value: &str) -> Kind {
    match value {
        "theorem" => Kind::Theorem,
        "axiom" => Kind::Axiom,
        _ => Kind::Definition,
    }
}
fn lexical(snapshot: &Snapshot) -> Result<Graph> {
    let mut nodes = Vec::new();
    let mut references = BTreeMap::new();
    let mut private = BTreeSet::new();
    for (file, source) in &snapshot.files {
        let lines = source.lines().collect::<Vec<_>>();
        for symbol in crate::adapters::source::symbols(
            source,
            std::path::Path::new(file)
                .extension()
                .is_some_and(|e| e == "lean"),
        ) {
            let name = symbol
                .name
                .split('(')
                .next()
                .unwrap_or(&symbol.name)
                .to_owned();
            let id = format!("{file}:{}:{name}", symbol.line);
            if symbol.private {
                private.insert(id.clone());
            }
            references.insert(id.clone(), symbol.references);
            anyhow::ensure!(
                nodes.len() < 5000,
                "Lexical graph exceeds 5000 declarations; no partial graph was published"
            );
            nodes.push(Node {
                id,
                label: name.clone(),
                name,
                kind: kind(&symbol.kind),
                is_main_theorem: symbol.main_theorem,
                statement: lines
                    .get(symbol.line.saturating_sub(1))
                    .copied()
                    .unwrap_or("")
                    .into(),
                source: Location {
                    file: file.clone(),
                    line: Some(u32::try_from(symbol.line)?),
                },
                proof_status: ProofHint::Unknown,
                axioms: Vec::new(),
                semantic: None,
            });
        }
    }
    let edges = lexical_edges(&nodes, &references, &private)?;
    Ok(Graph {
        revision: snapshot.revision.clone(),
        declarations: nodes,
        edges,
    })
}

fn lexical_edges(
    nodes: &[Node],
    references: &BTreeMap<String, Vec<String>>,
    private: &BTreeSet<String>,
) -> Result<Vec<Edge>> {
    let mut named: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut local_named: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        named.entry(&node.name).or_default().push(index);
        local_named
            .entry((
                &node.source.file,
                node.name.rsplit('.').next().unwrap_or(&node.name),
            ))
            .or_default()
            .push(index);
    }
    let mut edges = BTreeSet::new();
    for node in nodes {
        for token in references.get(&node.id).into_iter().flatten() {
            let mut namespace = node.name.rsplit_once('.').map_or("", |(prefix, _)| prefix);
            let mut candidates = Vec::new();
            while !namespace.is_empty() {
                candidates.push(format!("{namespace}.{token}"));
                namespace = namespace.rsplit_once('.').map_or("", |(prefix, _)| prefix);
            }
            candidates.push(token.clone());
            let exact = candidates.iter().find_map(|name| {
                let eligible = named
                    .get(name.as_str())
                    .into_iter()
                    .flatten()
                    .filter_map(|index| nodes.get(*index))
                    .filter(|other| {
                        !private.contains(&other.id) || other.source.file == node.source.file
                    })
                    .collect::<Vec<_>>();
                if eligible.len() == 1 {
                    eligible.first().copied()
                } else {
                    None
                }
            });
            let local = || {
                let eligible = local_named.get(&(node.source.file.as_str(), token.as_str()))?;
                if eligible.len() != 1 {
                    return None;
                }
                eligible.first().and_then(|index| nodes.get(*index))
            };
            if let Some(target) = exact.or_else(local).filter(|other| other.id != node.id) {
                edges.insert((node.id.clone(), target.id.clone()));
                anyhow::ensure!(
                    edges.len() <= 100_000,
                    "Lexical graph exceeds 100000 edges; no partial graph was published"
                );
            }
        }
    }
    Ok(edges
        .into_iter()
        .map(|(source, target)| Edge {
            source,
            target,
            site: Site::Unknown,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(files: &[(&str, &str)]) -> Result<Snapshot> {
        let files = files
            .iter()
            .map(|(p, s)| ((*p).into(), (*s).into()))
            .collect::<BTreeMap<String, String>>();
        Ok(Snapshot {
            revision: format!(
                "sha256:{}",
                crate::atlas::digest(&serde_json::to_vec(&files)?)
            ),
            files,
            atlas: None,
            collection: None,
        })
    }
    fn relations(page: &SourcePage) -> Result<Vec<(String, String)>> {
        let graph = page.graph.as_ref().context("graph")?;
        let names = graph
            .declarations
            .iter()
            .map(|n| (&n.id, &n.name))
            .collect::<BTreeMap<_, _>>();
        Ok(graph
            .edges
            .iter()
            .filter_map(|e| {
                Some((
                    (*names.get(&e.source)?).clone(),
                    (*names.get(&e.target)?).clone(),
                ))
            })
            .collect())
    }
    #[test]
    fn native_projection_preserves_names_lines_comments_and_cross_file_references() -> Result<()> {
        let raw = snapshot(&[("Spec.lean", "namespace Machine\ndef step (s : Nat) := s - 1\nend Machine\n"), ("Check.lean", "namespace Proof\n-- audit: proved\ntheorem bound (s : Nat) : Machine.step s ≤ s := by\n  sorry\nend Proof\n")])?;
        let page = SourcePage::from_snapshot(&raw)?;
        assert_eq!(
            relations(&page)?,
            vec![("Proof.bound".into(), "Machine.step".into())]
        );
        assert!(page.graph.as_ref().is_some_and(|g| g
            .declarations
            .iter()
            .any(|n| n.name == "Proof.bound" && n.source.line == Some(3))));
        let published = serde_json::to_value(PreparedSource::new(raw)?)?;
        assert!(
            published.get("atlas").is_none()
                && published.get("collection").is_none()
                && published.get("evidence").is_none()
        );
        Ok(())
    }
    #[test]
    fn namespace_shadowing_privacy_and_ambiguous_names_remain_conservative() -> Result<()> {
        let page = SourcePage::from_snapshot(&snapshot(&[
            ("A.lean", "namespace A\ndef x := 1\nnamespace B\nprivate def x := 2\ndef chosen := x\nend B\ndef outer := x\nend A\n"),
            ("B.lean", "def x := 3\ndef unknown := chosen\n"), ("C.lean", "def x := 4\n"), ("D.lean", "def ambiguous := x\n"),
        ])?)?;
        let edges = relations(&page)?;
        assert!(edges.contains(&("A.B.chosen".into(), "A.B.x".into())));
        assert!(edges.contains(&("A.outer".into(), "A.x".into())));
        assert!(!edges
            .iter()
            .any(|(name, _)| name == "unknown" || name == "ambiguous"));
        Ok(())
    }
    #[test]
    fn raw_strings_and_comments_never_create_declarations_or_claim_proof() -> Result<()> {
        let page = SourcePage::from_snapshot(&snapshot(&[("src/kernel.rs", "pub struct State { pub stock: u64 }\npub fn step(s: State) -> State { s }\npub fn label() { let _ = r##\"State step\nfn fake() {}\"##; }\n")])?)?;
        let names = page
            .graph
            .as_ref()
            .context("graph")?
            .declarations
            .iter()
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["State", "step", "label"]);
        assert_eq!(relations(&page)?, vec![("step".into(), "State".into())]);
        assert!(page.graph.as_ref().is_some_and(|g| g
            .declarations
            .iter()
            .all(|n| matches!(n.proof_status, ProofHint::Unknown))));
        Ok(())
    }
    #[test]
    fn nested_comments_and_unicode_preserve_the_original_line_numbers() -> Result<()> {
        let page = SourcePage::from_snapshot(&snapshot(&[("Main.lean", "namespace N\n/- outer\n/- theorem fake : True := real -/\n-/\ndef real := 1\ndef text := \"real theorem hidden\"\n-- def ghost := real\ntheorem α : real = 1 := by rfl\nend N\n")])?)?;
        let graph = page.graph.as_ref().context("graph")?;
        assert_eq!(
            graph
                .declarations
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
            vec!["N.real", "N.text", "N.α"]
        );
        assert!(graph
            .declarations
            .iter()
            .any(|n| n.name == "N.α" && n.source.line == Some(8)));
        assert_eq!(relations(&page)?.len(), 1);
        Ok(())
    }
    #[test]
    fn invalid_references_ranges_revisions_and_duplicate_ids_are_rejected_but_cycles_are_valid(
    ) -> Result<()> {
        let raw = snapshot(&[("Spec.lean", "def first := second\ndef second := first\n")])?;
        let page = SourcePage::from_snapshot(&raw)?;
        assert_eq!(relations(&page)?.len(), 2);
        let mut invalid = page.clone();
        invalid
            .files
            .insert("Spec.lean".into(), SourceText::new("changed".into())?);
        assert!(invalid.validate().is_err());
        let mut invalid = page.clone();
        if let Some(g) = &mut invalid.graph {
            if let Some(n) = g.declarations.first_mut() {
                n.source.line = Some(999);
            }
        }
        assert!(invalid.validate().is_err());
        let mut invalid = page.clone();
        if let Some(g) = &mut invalid.graph {
            if let Some(n) = g.declarations.first().cloned() {
                g.declarations.push(n);
            }
        }
        assert!(invalid.validate().is_err());
        let mut invalid = page;
        if let Some(g) = &mut invalid.graph {
            g.edges.push(Edge {
                source: "missing".into(),
                target: "missing".into(),
                site: Site::Unknown,
            });
        }
        assert!(invalid.validate().is_err());
        Ok(())
    }

    #[test]
    fn oversized_utf8_sources_are_refused_without_truncating_published_content() -> Result<()> {
        let text = "α".repeat(524_288);
        let source = SourceText::new(text.clone())?;
        assert_eq!(serde_json::to_value(source)?, serde_json::json!(text));
        let oversized = format!("{text}α");
        assert!(SourceText::new(oversized.clone()).is_err());
        assert!(serde_json::from_value::<SourceText>(serde_json::json!(oversized)).is_err());
        assert!(SourcePage::from_snapshot(&snapshot(&[("Large.lean", &oversized)])?).is_err());
        Ok(())
    }

    #[test]
    fn large_lexical_page_keeps_all_references_and_original_lines() -> Result<()> {
        use std::fmt::Write;
        let mut source = String::from("def base := 1\n");
        for index in 1..4000 {
            writeln!(
                source,
                "def item{index} (x : Unknown.Type) := base + x + Unknown.value"
            )?;
        }
        let raw = snapshot(&[("Large.lean", &source)])?;
        let started = std::time::Instant::now();
        let page = SourcePage::from_snapshot(&raw)?;
        eprintln!(
            "4000-declaration source projection: {:?}",
            started.elapsed()
        );
        let graph = page.graph.context("graph")?;
        assert_eq!(graph.declarations.len(), 4000);
        assert_eq!(
            graph.declarations.last().and_then(|n| n.source.line),
            Some(4000)
        );
        let base = &graph.declarations.first().context("base")?.id;
        assert_eq!(graph.edges.len(), 3999);
        assert!(graph.edges.iter().all(|edge| &edge.target == base));
        Ok(())
    }
}
