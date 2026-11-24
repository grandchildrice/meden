//! Offline Plausible search, independently selected from source compilation.
use crate::{
    explanation::Check,
    search::{Completed, Invocation, Report, Searcher},
};
use anyhow::{Context as _, Result};
use meden_runtime::plugin::{Definition, Descriptor, Role};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub seed: u64,
    pub declaration_seconds: u64,
    pub file_seconds: u64,
}
impl Settings {
    pub fn host() -> Result<Self> {
        let get = |name, default| std::env::var(name).map_or(Ok(default), |s| s.parse::<u64>());
        let value = Self {
            seed: get("MEDEN_SEARCH_SEED", 0)?,
            declaration_seconds: get("MEDEN_SEARCH_DECLARATION_SECONDS", 10)?,
            file_seconds: get("MEDEN_SEARCH_FILE_SECONDS", 120)?,
        };
        anyhow::ensure!(
            u32::try_from(value.seed).is_ok()
                && (1..=30).contains(&value.declaration_seconds)
                && (1..=600).contains(&value.file_seconds),
            "Invalid bounded search settings"
        );
        Ok(value)
    }
}
pub struct Plausible {
    atlas: crate::atlas::Configuration,
    settings: Settings,
}
impl Plausible {
    pub fn new(atlas: crate::atlas::Configuration) -> Result<Self> {
        Ok(Self {
            atlas,
            settings: Settings::host()?,
        })
    }
}
impl Searcher for Plausible {
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(Definition {
            id: "lean.plausible",
            role: Role::CounterexampleSearch,
            version: "1",
            formats: vec!["lean-module-v1".into()],
            implementation: &[
                include_bytes!("plausible.rs").as_slice(),
                include_bytes!("../search.rs").as_slice(),
                include_bytes!("../atlas/project.rs").as_slice(),
                include_bytes!("../atlas/mod.rs").as_slice(),
                include_bytes!("../atlas/resources.rs").as_slice(),
                include_bytes!("../../../meden-runtime/src/sandbox.rs").as_slice(),
                include_bytes!("../atlas/frozen.rs").as_slice(),
                include_bytes!("../explanation.rs").as_slice(),
                include_bytes!("../atlas/search-atlas.py").as_slice(),
                include_bytes!("../atlas/MedenSearch.lean").as_slice(),
                include_bytes!("../atlas/MedenSearchMain.lean").as_slice(),
            ]
            .concat(),
            configuration: json!({"settings":self.settings,"instances":64,"retries":10,"max_size":32,"heartbeats":10000,"instance_heartbeats":2000,"result_bytes":8_388_608,"resources":self.atlas.search_resources()?}),
            input: crate::search::input_schema()?,
            output: crate::search::output_schema()?,
        })?)
    }
    fn search(&self, context: &super::Context<'_>, invocation: Invocation) -> Result<Completed> {
        let original = invocation.request().snapshot();
        let collection = original
            .collection
            .as_ref()
            .context("Missing original module bindings")?;
        let modules = collection
            .modules
            .iter()
            .map(|m| m.name.replace('.', "/"))
            .collect::<Vec<_>>();
        let graph = original
            .atlas
            .as_ref()
            .context("Missing compiled search input")?;
        let targets = graph
            .graph
            .nodes
            .iter()
            .filter(|n| {
                n.semantic.is_some() && n.search_kind != Some(crate::atlas::SearchKind::Value)
            })
            .map(|n| n.id.clone())
            .collect::<Vec<_>>();
        let request = serde_json::to_vec(
            &json!({"settings":self.settings,"modules":modules,"targets":targets}),
        )?;
        anyhow::ensure!(request.len() <= 8_388_608, "Search input exceeds 8 MiB");
        let output = self.atlas.run_search(
            context.workspace.path(),
            original,
            &request,
            context.control,
            Duration::from_secs(self.settings.file_seconds.saturating_add(30)),
        )?;
        let mut report: Report =
            serde_json::from_str(&output).context("Search did not return a complete report")?;
        for node in &graph.graph.nodes {
            if node.semantic.is_some() && node.search_kind == Some(crate::atlas::SearchKind::Value)
            {
                anyhow::ensure!(
                    report
                        .checks
                        .insert(
                            node.id.clone(),
                            Check {
                                engine: "Plausible".into(),
                                status: "not_proposition".into(),
                                candidates: vec![],
                                diagnostic: None,
                                seed: None,
                                num_inst: None,
                                num_retries: None,
                                max_size: None,
                                mode: None
                            }
                        )
                        .is_none(),
                    "Search returned an unrequested declaration"
                );
            }
        }
        let paths = original.files.keys().cloned().collect::<Vec<_>>();
        anyhow::ensure!(
            crate::analysis::sources::capture(context.workspace, context.control, &paths)?.files
                == original.files,
            "Sources changed during search"
        );
        crate::analysis::collect::check_metadata(
            context.workspace,
            context.control,
            &collection.metadata,
        )?;
        if let Some(expected) = &collection.prepared {
            let output = self.atlas.project(
                context.workspace.path(),
                "project-atlas-prepare.sh",
                &[
                    "fingerprint".into(),
                    serde_json::to_string(&collection.modules)?,
                ],
                context.control,
            )?;
            let observed: serde_json::Value = serde_json::from_str(&output)?;
            anyhow::ensure!(
                observed
                    .get("fingerprint")
                    .and_then(serde_json::Value::as_str)
                    == Some(expected),
                "Prepared dependencies changed during search"
            );
        }
        Ok(invocation.observe(report))
    }
}
