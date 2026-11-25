//! Lean's decoded graph-to-view projection; no compilation or model I/O.
use super::Snapshot;
use crate::view::{Edge, Graph, Kind, Location, Node, ProofHint, Site, SourcePage, SourceText};
use anyhow::Result;

impl crate::view::Projection for Snapshot {
    fn page(&self) -> Result<SourcePage> {
        let (graph, description) = if let Some(extraction) = &self.atlas {
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
                        semantic: node
                            .semantic
                            .clone()
                            .map(crate::explanation::Description::complete)
                            .transpose()?,
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
            let mut page = crate::view::Projection::page(&self.source)?;
            if let Some(collection) = &self.collection {
                page.roots.clone_from(&collection.roots);
                page.external.clone_from(&collection.external_modules);
                page.inputs_revision.clone_from(&collection.prepared);
            }
            return Ok(page);
        };
        let page = SourcePage {
            revision: self.source.revision.clone(),
            files: self
                .source
                .files
                .iter()
                .map(|(path, text)| Ok((path.clone(), SourceText::new(text.clone())?)))
                .collect::<Result<_>>()?,
            roots: self.collection.as_ref().map_or_else(
                || self.source.files.keys().cloned().collect(),
                |c| c.roots.clone(),
            ),
            external: self
                .collection
                .as_ref()
                .map_or_else(Vec::new, |c| c.external_modules.clone()),
            inputs_revision: self.collection.as_ref().and_then(|c| c.prepared.clone()),
            compiled: self.atlas.is_some(),
            graph,
            description,
        };
        Ok(page)
    }
}

fn kind(value: &str) -> Kind {
    match value {
        "theorem" => Kind::Theorem,
        "axiom" => Kind::Axiom,
        _ => Kind::Definition,
    }
}
