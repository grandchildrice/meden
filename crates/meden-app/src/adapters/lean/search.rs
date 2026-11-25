//! Lean declaration coverage and report projection; no evidence admission.
use super::Snapshot;
use crate::explanation::Check;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub checks: BTreeMap<String, Check>,
}
impl crate::search::Observation<Snapshot> for Report {
    fn validate(&self, original: &Snapshot) -> Result<()> {
        let graph = original.graph();
        let expected = graph
            .graph
            .nodes
            .iter()
            .filter(|node| node.semantic.is_some())
            .map(|node| node.id.as_str())
            .collect::<BTreeSet<_>>();
        anyhow::ensure!(
            self.checks.len() == expected.len()
                && self
                    .checks
                    .keys()
                    .all(|name| expected.contains(name.as_str()))
                && self.checks.values().all(Check::valid),
            "Missing, foreign or invalid search observations"
        );
        Ok(())
    }
}
impl Report {
    pub(crate) fn project(&self, mut snapshot: Snapshot) -> Result<Snapshot> {
        crate::search::Observation::validate(self, &snapshot)?;
        for node in &mut snapshot.graph_mut().graph.nodes {
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
