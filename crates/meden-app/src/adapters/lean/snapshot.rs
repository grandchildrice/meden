//! Lean-owned source observations and disjoint compiled input owners.
use crate::{
    analysis::{collect::Collection, sources},
    atlas::Extraction,
    identity::snapshots::{PreparedRevision, SourceRevision},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Compiled file resources belong to Lean's observation, never the display graph.
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Inputs {
    Frozen { modules: crate::atlas::Modules },
    Lake { fingerprint: PreparedRevision },
}
#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Compilation {
    graph: Extraction,
    collection: Collection,
    inputs: Inputs,
}

/// A Lean observation always owns one complete compilation: its graph,
/// original collection and exactly one input mode. Plain sources have their own type.
/// ```compile_fail,E0616
/// fn replace(s: &mut meden_app::adapters::lean::Snapshot) { s.compilation = None; }
/// ```
#[derive(Clone, Serialize, schemars::JsonSchema)]
#[schemars(rename = "LeanSnapshot")]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    #[serde(flatten)]
    pub source: sources::Snapshot,
    compilation: Compilation,
}
impl<'de> Deserialize<'de> for Snapshot {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            revision: SourceRevision,
            files: std::collections::BTreeMap<String, String>,
            compilation: Compilation,
        }
        let wire = Wire::deserialize(d)?;
        Ok(Self {
            source: sources::Snapshot {
                revision: wire.revision,
                files: wire.files,
            },
            compilation: wire.compilation,
        })
    }
}
impl Snapshot {
    pub(crate) fn compiled(
        source: sources::Snapshot,
        graph: Extraction,
        collection: Collection,
        inputs: Inputs,
    ) -> Self {
        Self {
            source,
            compilation: Compilation {
                graph,
                collection,
                inputs,
            },
        }
    }
    pub fn graph(&self) -> &Extraction {
        &self.compilation.graph
    }
    pub(crate) fn graph_mut(&mut self) -> &mut Extraction {
        &mut self.compilation.graph
    }
    pub fn collection(&self) -> &Collection {
        &self.compilation.collection
    }
    pub fn prepared(&self) -> Option<&PreparedRevision> {
        match &self.compilation.inputs {
            Inputs::Lake { fingerprint } => Some(fingerprint),
            Inputs::Frozen { .. } => None,
        }
    }
    pub(crate) fn modules(&self) -> Option<&crate::atlas::Modules> {
        match &self.compilation.inputs {
            Inputs::Frozen { modules } => Some(modules),
            Inputs::Lake { .. } => None,
        }
    }
    pub(crate) fn into_graph(self) -> Extraction {
        self.compilation.graph
    }
}
impl sources::Source for Snapshot {
    fn sources(&self) -> &sources::Snapshot {
        &self.source
    }
}
impl crate::compiler::Observation for Snapshot {
    fn validate(&self) -> Result<()> {
        let compiled = &self.compilation;
        anyhow::ensure!(
            compiled
                .graph
                .graph
                .nodes
                .iter()
                .all(|n| n.semantic.as_ref().is_none_or(|s| s.checker.is_none())),
            "Compiler returned a search observation"
        );
        compiled.graph.graph.validate(&self.source.files)?;
        anyhow::ensure!(
            !compiled.collection.roots.is_empty()
                && compiled
                    .collection
                    .roots
                    .iter()
                    .all(|p| self.source.files.contains_key(p)),
            "Source root is outside the retained inputs"
        );
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn fixture(source: sources::Snapshot, graph: Extraction) -> Snapshot {
    let collection = Collection {
        roots: source.files.keys().cloned().collect(),
        external_modules: Vec::new(),
        modules: Vec::new(),
        packages: Vec::new(),
        metadata: crate::analysis::observe::Metadata::default(),
    };
    Snapshot::compiled(
        source,
        graph,
        collection,
        Inputs::Lake {
            fingerprint: PreparedRevision::from_digest([0; 32].into()),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context as _;
    use serde_json::json;
    use sha2::{Digest as _, Sha256};
    fn source() -> Result<sources::Snapshot> {
        let files = std::collections::BTreeMap::from([("M.lean".into(), "def x := 1\n".into())]);
        Ok(sources::Snapshot {
            revision: SourceRevision::from_digest(Sha256::digest(serde_json::to_vec(&files)?)),
            files,
        })
    }
    fn graph() -> Result<Extraction> {
        Ok(serde_json::from_value(
            json!({"revision":format!("sha256:{}","1".repeat(64)),"upstream":"fixture","resources_sha256":"fixture","graph":{"metadata":{"projectName":"fixture","leanVersion":"fixture"},"nodes":[],"edges":[],"externalReferences":[]}}),
        )?)
    }
    #[test]
    fn compiled_modes_round_trip_and_plain_source_is_not_a_compiler_observation() -> Result<()> {
        let source = source()?;
        let plain = serde_json::to_value(&source)?;
        assert!(serde_json::from_value::<Snapshot>(plain).is_err());
        let mut absent = serde_json::to_value(&source)?;
        absent
            .as_object_mut()
            .context("source object")?
            .insert("compilation".into(), serde_json::Value::Null);
        assert!(serde_json::from_value::<Snapshot>(absent).is_err());
        let lake = fixture(source.clone(), graph()?);
        let value = serde_json::to_value(&lake)?;
        let decoded: Snapshot = serde_json::from_value(value.clone())?;
        assert!(decoded.prepared().is_some() && decoded.modules().is_none());
        assert_eq!(serde_json::to_value(decoded)?, value);
        let mut frozen = value;
        *frozen
            .pointer_mut("/compilation/inputs")
            .context("inputs")? = json!({"kind":"frozen","modules":{"id":"0".repeat(64),"files":{"M.olean":"1".repeat(64)}}});
        let decoded: Snapshot = serde_json::from_value(frozen.clone())?;
        assert!(decoded.prepared().is_none() && decoded.modules().is_some());
        assert_eq!(serde_json::to_value(decoded)?, frozen);
        assert!(frozen
            .pointer("/compilation/graph/compiled_modules")
            .is_none());
        Ok(())
    }
    #[test]
    fn decoded_observations_refuse_missing_or_mixed_input_modes_and_retired_fields() -> Result<()> {
        let valid = serde_json::to_value(fixture(source()?, graph()?))?;
        for key in ["graph", "collection", "inputs"] {
            let mut v = valid.clone();
            v.get_mut("compilation")
                .context("compilation")?
                .as_object_mut()
                .context("compilation")?
                .remove(key);
            assert!(serde_json::from_value::<Snapshot>(v).is_err(), "{key}");
        }
        for input in [
            json!({"kind":"lake"}),
            json!({"kind":"frozen"}),
            json!({"kind":"unknown","fingerprint":"0".repeat(64)}),
            json!({"kind":"lake","fingerprint":"0".repeat(64),"modules":{"id":"0".repeat(64),"files":{}}}),
            json!({"kind":"frozen","modules":{"id":"0".repeat(64),"files":{}},"fingerprint":"0".repeat(64)}),
        ] {
            let mut v = valid.clone();
            *v.pointer_mut("/compilation/inputs").context("inputs")? = input;
            assert!(serde_json::from_value::<Snapshot>(v).is_err());
        }
        let mut v = valid.clone();
        v.pointer_mut("/compilation/graph")
            .and_then(serde_json::Value::as_object_mut)
            .context("graph")?
            .insert(
                "compiled_modules".into(),
                json!({"id":"0".repeat(64),"files":{}}),
            );
        assert!(serde_json::from_value::<Snapshot>(v).is_err());
        let mut retired = valid.clone();
        retired
            .pointer_mut("/compilation/collection")
            .and_then(serde_json::Value::as_object_mut)
            .context("collection")?
            .insert("prepared".into(), json!("1".repeat(64)));
        assert!(serde_json::from_value::<Snapshot>(retired).is_err());
        for key in ["atlas", "collection", "compiled_modules"] {
            let mut v = valid.clone();
            v.as_object_mut()
                .context("source")?
                .insert(key.into(), json!({}));
            assert!(serde_json::from_value::<Snapshot>(v).is_err(), "{key}");
        }
        for id in [
            String::new(),
            "../cache".into(),
            "A".repeat(64),
            "0".repeat(63),
            format!("sha256:{}", "0".repeat(64)),
        ] {
            let mut v = valid.clone();
            *v.pointer_mut("/compilation/inputs").context("inputs")? =
                json!({"kind":"frozen","modules":{"id":id,"files":{}}});
            assert!(serde_json::from_value::<Snapshot>(v).is_err());
        }
        Ok(())
    }
}
