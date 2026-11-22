//! Host-prepared Lake inputs; compilation and read-only extraction are distinct profiles.
use super::{Configuration, Execution, Extraction};
use anyhow::Result;
use meden_runtime::process::Control;
use std::{collections::BTreeMap, path::Path, time::Duration};

impl Configuration {
    pub(crate) fn project(
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
        };
        let output = execution.execute(script, args, control, Duration::from_secs(600))?;
        anyhow::ensure!(
            serde_json::to_vec(&before)? == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
            "Atlas resources changed during prepared extraction"
        );
        Ok(output)
    }
    pub(crate) fn admit_project(
        &self,
        graph: serde_json::Value,
        files: &BTreeMap<String, String>,
    ) -> Result<Extraction> {
        let (lean, library) = self.selected()?;
        Extraction::new(graph, files, &Self::validate(&lean, &library)?)
    }
}
