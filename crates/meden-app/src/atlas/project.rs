//! Host-prepared Lake inputs; compilation and read-only extraction are distinct profiles.
use super::{Configuration, Execution, Extraction};
use anyhow::{Context as _, Result};
use meden_runtime::process::Control;
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn build_timeout() -> Result<Duration> {
    let seconds = std::env::var("MEDEN_ATLAS_BUILD_TIMEOUT_SECONDS")
        .map_or_else(|_| Ok(7200), |s| s.parse::<u64>())?;
    anyhow::ensure!(
        (1..=86_400).contains(&seconds),
        "Build timeout must be between 1 and 86400 seconds"
    );
    Ok(Duration::from_secs(seconds))
}

/// Fixed prepared-Lake input operation, selected by trusted application code.
#[derive(Clone, Copy)]
pub enum InputMode {
    /// Prepare/build the originally selected modules before observation.
    Prepare,
    /// Reobserve the original complete input binding for freshness.
    Fingerprint,
}

impl Configuration {
    /// Decode the metadata worker's original digest once at its backend boundary.
    /// This does not change query/build permissions or admit a verification result.
    pub fn input_fingerprint(
        &self,
        workspace: &Path,
        modules: &[crate::adapters::lean::layout::Module],
        mode: InputMode,
        control: &Control,
    ) -> Result<crate::identity::snapshots::PreparedRevision> {
        #[derive(serde::Deserialize)]
        struct Inputs {
            fingerprint: crate::identity::snapshots::PreparedRevision,
        }
        let mode = match mode {
            InputMode::Prepare => "prepare",
            InputMode::Fingerprint => "fingerprint",
        };
        let observed: Inputs = serde_json::from_str(&self.project(
            workspace,
            "project-atlas-prepare.sh",
            &[mode.into(), serde_json::to_string(modules)?],
            control,
        )?)?;
        Ok(observed.fingerprint)
    }

    pub fn run_search(
        &self,
        workspace: &Path,
        snapshot: &crate::adapters::lean::Snapshot,
        input: &[u8],
        control: &Control,
        timeout: Duration,
    ) -> Result<String> {
        let (lean, library) = self.selected()?;
        let before = Self::validate(&lean, &library)?;
        let prepared = snapshot.prepared();
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
        let output = if script == "project-atlas-prepare.sh" {
            let [mode, encoded] = args else {
                anyhow::bail!("Expected preparation mode and original module bindings")
            };
            let bindings: Vec<crate::adapters::lean::layout::Module> =
                serde_json::from_str(encoded)?;
            anyhow::ensure!(
                matches!(mode.as_str(), "prepare" | "fingerprint")
                    && (1..=32).contains(&bindings.len()),
                "Expected prepare/fingerprint and between 1 and 32 roots"
            );
            let mut targets = Vec::new();
            for binding in bindings {
                binding.validate()?;
                for facet in ["", ":lean", ":olean", ":setup"] {
                    targets.push(format!("+{}{facet}", binding.name));
                }
            }
            let started = Instant::now();
            // Admit a complete successful query before the metadata-only worker
            // can inspect its output. Project code never sees the fingerprinter.
            let resolved = execution
                .execute("project-atlas-query.sh", &targets, control, timeout)
                .context("Prepared Lake compilation failed")?;
            let metadata = tempfile::tempdir()?;
            super::resources::write_metadata(metadata.path())?;
            let validation = Execution {
                library: metadata.path(),
                ..execution
            };
            validation.execute_input(
                script,
                args,
                control,
                timeout
                    .checked_sub(started.elapsed())
                    .context("Prepared input query exhausted its time limit")?,
                resolved.as_bytes(),
            )?
        } else {
            execution.execute(script, args, control, timeout)?
        };
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
