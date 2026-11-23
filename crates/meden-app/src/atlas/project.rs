//! Host-prepared Lake inputs; compilation and read-only extraction are distinct profiles.
use super::{Configuration, Execution, Extraction};
use anyhow::{Context as _, Result};
use meden_runtime::process::Control;
use std::{collections::BTreeMap, path::Path, time::Duration};

pub(super) fn build_timeout() -> Result<Duration> {
    let seconds = std::env::var("MEDEN_ATLAS_BUILD_TIMEOUT_SECONDS")
        .map_or_else(|_| Ok(7200), |s| s.parse::<u64>())?;
    anyhow::ensure!(
        (1..=86_400).contains(&seconds),
        "Build timeout must be between 1 and 86400 seconds"
    );
    Ok(Duration::from_secs(seconds))
}

impl Configuration {
    pub fn run_search(
        &self,
        workspace: &Path,
        snapshot: &crate::analysis::sources::Snapshot,
        input: &[u8],
        control: &Control,
        timeout: Duration,
    ) -> Result<String> {
        let (lean, library) = self.selected()?;
        let before = Self::validate(&lean, &library)?;
        let prepared = snapshot
            .collection
            .as_ref()
            .context("Missing original compilation bindings")?
            .prepared
            .as_ref();
        let root = if prepared.is_some() {
            workspace.to_path_buf()
        } else {
            self.frozen_modules(snapshot)?
        };
        let script = if prepared.is_some() {
            "project-search.sh"
        } else {
            "search.sh"
        };
        let execution = Execution {
            root: &root,
            lean: &lean,
            library: &library,
            sources: None,
            observer: self.observer.as_ref(),
        };
        let output = execution.observe_input(script, &[], control, timeout, input)?;
        anyhow::ensure!(
            matches!(output.termination,meden_runtime::process::Termination::Exited(status) if status.success()),
            "Counterexample worker failed: {}",
            output.stderr
        );
        anyhow::ensure!(
            serde_json::to_vec(&before)? == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
            "Search resources changed during execution"
        );
        if prepared.is_none() {
            self.frozen_modules(snapshot)?;
        }
        Ok(output.stdout)
    }
    pub fn project(
        &self,
        workspace: &Path,
        script: &str,
        args: &[String],
        control: &Control,
    ) -> Result<String> {
        let (lean, library) = self.selected()?;
        let before = Self::validate(&lean, &library)?;
        let execution = Execution {
            root: workspace,
            lean: &lean,
            library: &library,
            sources: None,
            observer: self.observer.as_ref(),
        };
        let timeout = if script == "project-atlas-prepare.sh"
            && args.first().is_some_and(|a| a == "prepare")
        {
            self.build_timeout()?
        } else {
            Duration::from_secs(600)
        };
        let output = execution.execute(script, args, control, timeout)?;
        anyhow::ensure!(
            serde_json::to_vec(&before)? == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
            "Atlas resources changed during prepared extraction"
        );
        Ok(output)
    }
    pub fn admit_project(
        &self,
        graph: serde_json::Value,
        files: &BTreeMap<String, String>,
    ) -> Result<Extraction> {
        let (lean, library) = self.selected()?;
        Extraction::new(graph, files, &Self::validate(&lean, &library)?)
    }
}
