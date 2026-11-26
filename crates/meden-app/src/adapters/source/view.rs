//! Lexical source navigation projection, separate from compilation and proof.
use crate::identity::snapshots::GraphRevision;
use crate::view::{Edge, Graph, Kind, Location, Node, ProofHint, Site, SourcePage, SourceText};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

impl crate::view::Projection for crate::analysis::sources::Snapshot {
    fn page(&self) -> Result<SourcePage> {
        let graph = self
            .files
            .keys()
            .any(|p| crate::adapters::source::supports(std::path::Path::new(p)))
            .then(|| lexical(self))
            .transpose()?;
        let description = if graph.is_some() {
            "Lexical reference candidates from retained source; no compiler or verification result is inferred."
        } else {
            "Retained source text; no graph provider is selected for this format."
        };
        Ok(SourcePage {
            revision: self.revision.clone(),
            files: self
                .files
                .iter()
                .map(|(p, s)| Ok((p.clone(), SourceText::new(s.clone())?)))
                .collect::<Result<_>>()?,
            roots: self.files.keys().cloned().collect(),
            graph,
            description: description.into(),
            external: Vec::new(),
            compiled: false,
            inputs_revision: None,
        })
    }
}
fn kind(value: &str) -> Kind {
    match value {
        "theorem" => Kind::Theorem,
        "axiom" => Kind::Axiom,
        _ => Kind::Definition,
    }
}
fn lexical(snapshot: &crate::analysis::sources::Snapshot) -> Result<Graph> {
    let mut nodes = Vec::new();
    let mut references = BTreeMap::new();
    let mut private = BTreeSet::new();
    for (file, source) in &snapshot.files {
        if !crate::adapters::source::supports(std::path::Path::new(file)) {
            continue;
        }
        let lines = source.lines().collect::<Vec<_>>();
        for symbol in crate::adapters::source::index(
            source,
            std::path::Path::new(file)
                .extension()
                .is_some_and(|e| e == "lean"),
        )
        .symbols
        {
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
        revision: GraphRevision::parse(snapshot.revision.as_str())?,
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
