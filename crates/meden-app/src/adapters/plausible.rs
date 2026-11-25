//! Offline Plausible search, independently selected from source compilation.
use crate::{
    adapters::lean::{search::Report, Snapshot},
    explanation::Check,
    search::{Completed, Invocation, Searcher},
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
    type Input = Snapshot;
    type Output = Report;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(Definition {
            id: "lean.plausible",
            role: Role::CounterexampleSearch,
            version: "1",
            formats: vec!["lean-module-v1".into()],
            implementation: &[
                include_bytes!("plausible.rs").as_slice(),
                include_bytes!("../search.rs").as_slice(),
                include_bytes!("lean/search.rs").as_slice(),
                include_bytes!("lean/snapshot.rs").as_slice(),
                include_bytes!("../explanation/cache.rs").as_slice(),
                include_bytes!("../explanation/cache/artifact.rs").as_slice(),
                include_bytes!("../identity/snapshots.rs").as_slice(),
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
            input: crate::search::input_schema::<Snapshot>()?,
            output: crate::search::output_schema::<Snapshot, Report>()?,
        })?)
    }
    fn search(
        &self,
        context: &super::Context<'_>,
        invocation: Invocation<Snapshot, Report>,
    ) -> Result<Completed<Snapshot, Report>> {
        let original = invocation.request().snapshot();
        let collection = original.collection();
        let modules = collection
            .modules
            .iter()
            .map(|m| m.name.replace('.', "/"))
            .collect::<Vec<_>>();
        let graph = original.graph();
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
        let paths = original.source.files.keys().cloned().collect::<Vec<_>>();
        anyhow::ensure!(
            crate::analysis::sources::capture(context.workspace, context.control, &paths)?.files
                == original.source.files,
            "Sources changed during search"
        );
        crate::analysis::collect::check_metadata(
            context.workspace,
            context.control,
            &collection.metadata,
        )?;
        if let Some(expected) = original.prepared() {
            let observed = self.atlas.input_fingerprint(
                context.workspace.path(),
                &collection.modules,
                crate::atlas::InputMode::Fingerprint,
                context.control,
            )?;
            anyhow::ensure!(
                &observed == expected,
                "Prepared dependencies changed during search"
            );
        }
        Ok(invocation.observe(report))
    }
}
