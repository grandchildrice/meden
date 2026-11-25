//! Import headers parsed by installed Lean inside the same offline analysis profile.
use super::{Configuration, Execution};
use anyhow::{Context, Result};
use meden_runtime::process::Control;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Imports {
    pub file: String,
    pub modules: Vec<Module>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub name: String,
    pub external: bool,
}
impl Configuration {
    pub fn imports(
        &self,
        files: &BTreeMap<String, String>,
        control: &Control,
    ) -> Result<Vec<Imports>> {
        let (lean, library) = self.selected()?;
        let before = Self::validate(&lean, &library)?;
        let job = tempfile::tempdir()?;
        let frozen = tempfile::tempdir()?;
        for (path, source) in files {
            crate::workspace::observation::relative("", path)?;
            let target = frozen.path().join(path);
            fs::create_dir_all(target.parent().context("import source parent missing")?)?;
            fs::write(target, source)?;
        }
        let execution = Execution {
            root: job.path(),
            lean: &lean,
            library: &library,
            sources: Some(frozen.path()),
            observer: self.observer.as_ref(),
        };
        let output = execution
            .execute(
                "imports.sh",
                &files.keys().cloned().collect::<Vec<_>>(),
                control,
                Duration::from_secs(30),
            )
            .context("Offline Lean import inspection failed; no dependency graph admitted")?;
        anyhow::ensure!(
            serde_json::to_vec(&before)? == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
            "Atlas resources changed during import inspection"
        );
        let rows: Vec<Imports> = serde_json::from_str(&output)
            .context("Imports did not return one complete JSON result")?;
        anyhow::ensure!(rows.len() == files.len(), "Incomplete import inspection");
        let mut seen = std::collections::BTreeSet::new();
        for row in &rows {
            anyhow::ensure!(
                files.contains_key(&row.file)
                    && seen.insert(&row.file)
                    && row.modules.len() <= 4096,
                "Invalid import/source correspondence"
            );
        }
        Ok(rows)
    }
}
