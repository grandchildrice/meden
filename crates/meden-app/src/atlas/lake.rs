//! Evaluate only frozen Lake configuration in a disposable offline analysis job.
use super::{Configuration, Execution};
use crate::analysis::observe::Metadata;
use anyhow::{Context, Result};
use meden_runtime::process::Control;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, time::Duration};

mod cache;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Glob {
    pub directory: String,
    pub name: String,
    pub kind: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LibrarySource {
    pub directory: String,
    pub selectors: Vec<Glob>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub name: String,
    pub kind: String,
    pub path: String,
    pub subdir: String,
    pub url: String,
    pub revision: Option<String>,
    pub options: BTreeMap<String, String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub src_dir: String,
    pub library_sources: Vec<LibrarySource>,
    pub executable_dirs: Vec<String>,
    pub globs: Vec<Glob>,
    pub dependencies: Vec<Dependency>,
    pub packages_dir: String,
    pub manifest: String,
}
impl Configuration {
    pub fn lake_package(
        &self,
        metadata: &Metadata,
        directory: &str,
        config: &str,
        options: &BTreeMap<String, String>,
        control: &Control,
    ) -> Result<Package> {
        self.lake_output(
            metadata,
            &[
                "package".into(),
                directory.into(),
                config.into(),
                serde_json::to_string(options)?,
            ],
            control,
            |output| {
                let package: Package = serde_json::from_str(output)
                    .context("Lake did not return one complete configuration result")?;
                anyhow::ensure!(
                    package.dependencies.len() <= 256
                        && package.library_sources.len() <= 64
                        && package.executable_dirs.len() <= 64
                        && package.globs.len() <= 128,
                    "Lake package configuration exceeds bounds"
                );
                Ok(package)
            },
        )
    }
    pub fn lake_manifest(
        &self,
        metadata: &Metadata,
        path: &str,
        control: &Control,
    ) -> Result<serde_json::Value> {
        self.lake_output(
            metadata,
            &["manifest".into(), path.into()],
            control,
            |output| {
                let value: serde_json::Value = serde_json::from_str(output)
                    .context("Invalid normalized Lake manifest result")?;
                anyhow::ensure!(value.is_object(), "Lake manifest must be an object");
                Ok(value)
            },
        )
    }
    fn lake_output<T>(
        &self,
        metadata: &Metadata,
        args: &[String],
        control: &Control,
        decode: impl Fn(&str) -> Result<T>,
    ) -> Result<T> {
        let _timing = meden_runtime::metrics::Span::start("atlas.lake_configuration.resolve");
        control.with_active(|| Ok(()))?;
        let (lean, library) = self.selected()?;
        let before = Self::validate(&lean, &library)?;
        let key = cache::key(metadata, args, &before)?;
        let cache = self
            .cache
            .as_ref()
            .context("Atlas host cache unavailable")?
            .join("lake-configurations");
        if let Some(output) = cache::read(&cache, &key)? {
            let _reuse = meden_runtime::metrics::Span::start("atlas.lake_configuration.reuse");
            let decoded = decode(&output)?;
            anyhow::ensure!(
                serde_json::to_vec(&before)?
                    == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
                "Atlas resources changed during Lake configuration reuse"
            );
            control.with_active(|| Ok(()))?;
            return Ok(decoded);
        }
        let job = tempfile::tempdir()?;
        let frozen = tempfile::tempdir()?;
        for (path, value) in &metadata.files {
            if let Some(value) = value {
                let path = crate::analysis::observe::relative("", path)?;
                let target = frozen.path().join(path);
                fs::create_dir_all(target.parent().context("Lake input parent missing")?)?;
                fs::write(target, value)?;
            }
        }
        let execution = Execution {
            root: job.path(),
            lean: &lean,
            library: &library,
            sources: Some(frozen.path()),
            observer: self.observer.as_ref(),
        };
        let output = execution
            .execute("lake.sh", args, control, Duration::from_secs(60))
            .context("Offline Lake configuration failed; no project graph admitted")?;
        let decoded = decode(&output)?;
        anyhow::ensure!(
            serde_json::to_vec(&before)? == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
            "Atlas resources changed during Lake configuration"
        );
        control.with_active(|| Ok(()))?;
        cache::write(&cache, &key, &output)?;
        Ok(decoded)
    }
}
