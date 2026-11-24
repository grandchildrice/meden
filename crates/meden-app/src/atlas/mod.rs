//! Host-prepared, offline Lean Atlas analysis. No output admits proof authority.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use meden_runtime::process::{self, Control, Limits, Termination};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

mod frozen;
mod graph;
pub use frozen::Modules;
mod imports;
pub mod lake;
pub mod progress;
mod project;
mod resources;
mod toolchain;
pub use graph::{Extraction, SearchKind};
pub const UPSTREAM: &str = "06be1eb76fb926b8cf8bd42a4a51208d128c327d";

#[derive(Clone)]
pub struct Configuration {
    lean: std::result::Result<PathBuf, String>,
    cache: Option<PathBuf>,
    observer: Option<crate::adapters::Observer>,
    build_timeout: std::result::Result<Duration, String>,
}
#[derive(Serialize)]
pub struct Status {
    pub available: bool,
    pub prepared: bool,
    pub detail: String,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u8,
    engine_sha256: String,
    files: BTreeMap<String, String>,
}

impl Configuration {
    pub fn host(workspace: &Path) -> Self {
        let lean = toolchain::discover(workspace).map_err(|error| format!("{error:#}"));
        let cache = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .filter(|path| path.is_absolute())
            .map(|root| root.join("meden/atlas"));
        Self {
            lean,
            cache,
            observer: None,
            build_timeout: project::build_timeout().map_err(|error| format!("{error:#}")),
        }
    }
    #[must_use]
    pub fn with_observer(mut self, observer: Option<crate::adapters::Observer>) -> Self {
        self.observer = observer;
        self
    }
    pub fn status(&self) -> Status {
        let selected = self.selected();
        let available = selected.is_ok();
        let ready = selected.and_then(|(lean, library)| Self::validate(&lean, &library));
        Status {
            available,
            prepared: ready.is_ok(),
            detail: ready.map_or_else(
                |error| format!("{error:#}"),
                |_| "Lean Atlas is prepared for offline extraction".into(),
            ),
        }
    }
    pub fn current_resources(&self) -> Result<String> {
        let (lean, library) = self.selected()?;
        compiler_resources(&Self::validate(&lean, &library)?)
    }
    pub fn search_resources(&self) -> Result<String> {
        self.current_resources()
    }
    pub fn build_timeout(&self) -> Result<Duration> {
        self.build_timeout.clone().map_err(anyhow::Error::msg)
    }
    fn selected(&self) -> Result<(PathBuf, PathBuf)> {
        let _timing = meden_runtime::metrics::Span::start("atlas.resources.select");
        anyhow::ensure!(
            cfg!(target_os = "linux"),
            "Offline Lean Atlas requires Linux and Bubblewrap"
        );
        let lean = self
            .lean
            .as_ref()
            .map_err(|error| anyhow::anyhow!("{error}"))?
            .canonicalize()?;
        anyhow::ensure!(
            lean.join("bin/lean").is_file(),
            "Lean analysis executable is missing"
        );
        let key = digest(&serde_json::to_vec(&(
            UPSTREAM,
            &lean,
            resources::identity(),
            engine_hash(&lean)?,
        ))?);
        Ok((
            lean,
            self.cache
                .as_ref()
                .context("Atlas host cache unavailable")?
                .join(key),
        ))
    }
    pub fn prepare(&self, control: &Control) -> Result<()> {
        let (lean, library) = self.selected()?;
        if Self::validate(&lean, &library).is_ok() {
            return Ok(());
        }
        let parent = library.parent().context("Atlas cache parent missing")?;
        fs::create_dir_all(parent)?;
        let source = tempfile::tempdir()?;
        resources::write(source.path())?;
        let output = tempfile::tempdir_in(parent)?;
        let modules = resources::modules();
        let execution = Execution {
            root: output.path(),
            lean: &lean,
            library: source.path(),
            sources: None,
            observer: self.observer.as_ref(),
        };
        execution.execute("prepare.sh", &modules, control, Duration::from_secs(180))?;
        resources::write(output.path())?;
        let files = compiled_files(output.path())?;
        crate::storage::write(
            &output.path().join("manifest.json"),
            &Manifest {
                version: 1,
                engine_sha256: engine_hash(&lean)?,
                files,
            },
        )?;
        if library.exists() {
            fs::remove_dir_all(&library)?;
        }
        fs::rename(output.path(), &library)?;
        Ok(())
    }
    pub fn ensure_prepared(&self, control: &Control) -> Result<()> {
        let (lean, library) = self.selected()?;
        if library.try_exists()? {
            // Existing resources must still validate. Explicit preparation repairs corruption.
            Self::validate(&lean, &library)?;
            Ok(())
        } else {
            self.prepare(control)
        }
    }
    fn validate(lean: &Path, library: &Path) -> Result<Manifest> {
        let _timing = meden_runtime::metrics::Span::start("atlas.resources.validate");
        let manifest: Manifest = crate::storage::read(&library.join("manifest.json"))?
            .context("Lean Atlas is not prepared")?;
        anyhow::ensure!(
            manifest.version == 1 && manifest.engine_sha256 == engine_hash(lean)?,
            "Lean analysis engine changed; prepare again"
        );
        anyhow::ensure!(
            manifest.files == compiled_files(library)?,
            "Atlas compiled library changed or is incomplete; prepare again"
        );
        resources::validate(library)?;
        Ok(manifest)
    }
    pub fn extract(
        &self,
        files: &BTreeMap<String, String>,
        bindings: &[crate::analysis::layout::Module],
        control: &Control,
    ) -> Result<Extraction> {
        let (lean, library) = self.selected()?;
        let before = Self::validate(&lean, &library)?;
        let cache = self
            .cache
            .as_ref()
            .context("Atlas host cache unavailable")?
            .join("compiled-modules");
        fs::create_dir_all(&cache)?;
        let job = tempfile::tempdir_in(&cache)?;
        let frozen = tempfile::tempdir()?;
        let started = Instant::now();
        anyhow::ensure!(
            bindings.len() == files.len() && bindings.iter().all(|m| files.contains_key(&m.file)),
            "Incomplete Atlas module/source correspondence"
        );
        let mut modules = Vec::new();
        for binding in bindings {
            binding.validate()?;
            modules.push(binding.name.replace('.', "/"));
        }
        for (path, source) in files {
            crate::analysis::observe::relative("", path)?;
            let target = frozen.path().join(path);
            fs::create_dir_all(target.parent().context("source parent missing")?)?;
            fs::write(target, source)?;
        }
        let execution = Execution {
            root: job.path(),
            lean: &lean,
            library: &library,
            sources: Some(frozen.path()),
            observer: self.observer.as_ref(),
        };
        execution.compile(bindings, control, started)?;
        let stdout = execution.execute("extract.sh", &modules, control, remaining(started)?)?;
        anyhow::ensure!(
            serde_json::to_vec(&before)? == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
            "Atlas resources changed during extraction"
        );
        let mut graph: Value = serde_json::from_str(&stdout)
            .context("Atlas did not return one complete JSON result")?;
        for node in graph
            .get_mut("nodes")
            .and_then(Value::as_array_mut)
            .context("Missing Atlas nodes")?
        {
            let logical = node
                .get("filePath")
                .and_then(Value::as_str)
                .context("Missing Atlas source path")?;
            let binding = bindings
                .iter()
                .find(|binding| format!("{}.lean", binding.name.replace('.', "/")) == logical)
                .context("Atlas module is outside the frozen binding")?;
            node["filePath"] = binding.file.clone().into();
        }
        let mut extraction = Extraction::new(graph, files, &before)?;
        extraction.compiled_modules = Some(self.retain_modules(&job, files, bindings)?);
        Ok(extraction)
    }
}

fn compiler_resources(manifest: &Manifest) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&(
        "meden.compute-resources.v1",
        &manifest.engine_sha256,
        &manifest.files,
        resources::identity(),
    ))?))
}

fn compiled_files(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    for module in resources::modules() {
        for suffix in [
            ".olean",
            ".olean.private",
            ".olean.server",
            ".ir",
            ".ir.sig",
        ] {
            let name = format!("{module}{suffix}");
            let path = root.join(&name);
            if suffix == ".olean" || path.try_exists()? {
                files.insert(name, file_hash(&path)?);
            }
        }
    }
    Ok(files)
}
pub fn module_path(path: &str) -> Result<String> {
    let name = path
        .strip_suffix(".lean")
        .context("Atlas requires .lean source files")?;
    anyhow::ensure!(
        !name.is_empty()
            && name.split('/').all(|part| !part.is_empty()
                && part.chars().enumerate().all(|(i, c)| c == '_'
                    || c.is_ascii_alphabetic()
                    || (i > 0 && c.is_ascii_digit()))),
        "Atlas module paths must use ordinary identifier components"
    );
    anyhow::ensure!(
        !matches!(
            name.split('/').next(),
            Some(
                "LeanAtlas"
                    | "MedenAtlas"
                    | "MedenImports"
                    | "MedenLake"
                    | "MedenSemantic"
                    | "MedenSearch"
                    | "MedenSearchMain"
                    | "MedenInterchange"
                    | "MedenPlausible"
                    | "Lake"
                    | "Lean"
                    | "Init"
                    | "Std"
            )
        ),
        "Atlas sources cannot replace prepared library modules"
    );
    Ok(name.into())
}
fn remaining(started: Instant) -> Result<Duration> {
    Duration::from_secs(180)
        .checked_sub(started.elapsed())
        .context("Atlas extraction exceeded its 180 second budget")
}
struct Execution<'a> {
    root: &'a Path,
    lean: &'a Path,
    library: &'a Path,
    sources: Option<&'a Path>,
    observer: Option<&'a crate::adapters::Observer>,
}
impl Execution<'_> {
    fn compile(
        &self,
        modules: &[crate::analysis::layout::Module],
        control: &Control,
        started: Instant,
    ) -> Result<()> {
        let mut pending = modules.to_vec();
        while !pending.is_empty() {
            let previous = pending.len();
            let mut next = Vec::new();
            let mut diagnostics = String::new();
            for module in pending {
                let output = self.observe(
                    "compile.sh",
                    &[
                        module.name.replace('.', "/"),
                        module.file.clone(),
                        module.root.clone(),
                    ],
                    control,
                    remaining(started)?,
                )?;
                if !matches!(output.termination,Termination::Exited(status) if status.success()) {
                    diagnostics.push_str(&output.stdout);
                    diagnostics.push_str(&output.stderr);
                    next.push(module);
                }
            }
            anyhow::ensure!(
                next.len() < previous,
                "Lean source compilation failed:\n{diagnostics}"
            );
            pending = next;
        }
        Ok(())
    }
    fn observe(
        &self,
        script: &str,
        args: &[String],
        control: &Control,
        timeout: Duration,
    ) -> Result<process::Output> {
        self.observe_input(script, args, control, timeout, b"")
    }
    fn observe_input(
        &self,
        script: &str,
        args: &[String],
        control: &Control,
        timeout: Duration,
        input: &[u8],
    ) -> Result<process::Output> {
        let _timing = meden_runtime::metrics::Span::start(match script {
            "lake.sh" => "atlas.lake_configuration",
            "imports.sh" => "atlas.imports",
            "project-atlas-query.sh" => "atlas.project.query",
            "project-atlas-prepare.sh" => "atlas.project.fingerprint",
            "project-atlas-extract.sh" => "atlas.project.extract",
            "compile.sh" => "atlas.compile",
            "search.sh" | "project-search.sh" => "atlas.search",
            _ => "atlas.execute",
        });
        let mut command = analysis_command(self, script, args)?;
        let notify = |output: &process::OutputProgress| {
            if let Some(observer) = self.observer {
                observer(
                    progress::stage(script, args),
                    timeout,
                    Some(&progress::diagnostic(script, output)),
                );
            }
        };
        if let Some(observer) = self.observer {
            observer(progress::stage(script, args), timeout, None);
        }
        let output = process::run_with_input_observed(
            &mut command,
            input,
            &Limits {
                timeout,
                capture_bytes: if script == "project-atlas-query.sh" {
                    134_217_728
                } else if matches!(
                    script,
                    "extract.sh" | "project-atlas-extract.sh" | "search.sh" | "project-search.sh"
                ) {
                    8_388_608
                } else {
                    32768
                },
                output_bytes: if script == "project-atlas-query.sh" {
                    268_435_456
                } else {
                    16_777_216
                },
            },
            control,
            self.observer
                .map(|_| &notify as &dyn Fn(&process::OutputProgress)),
        )?;
        if !matches!(output.termination, Termination::Exited(_)) {
            let reason = match output.termination {
                Termination::TimedOut => {
                    format!("reached the {}-second time limit", timeout.as_secs())
                }
                Termination::Cancelled => "was cancelled".into(),
                _ => "exceeded its output limit".into(),
            };
            anyhow::bail!("{} {reason}. Completed build files are retained; retry continues incrementally. No incomplete graph was saved.\n{}",
                progress::stage(script, args), output.stderr.chars().rev().take(4096).collect::<String>().chars().rev().collect::<String>());
        }
        anyhow::ensure!(
            !output.stdout_truncated,
            "Analysis result exceeded its capture limit; no incomplete graph was saved"
        );
        Ok(output)
    }
    fn execute(
        &self,
        script: &str,
        args: &[String],
        control: &Control,
        timeout: Duration,
    ) -> Result<String> {
        self.execute_input(script, args, control, timeout, &[])
    }
    fn execute_input(
        &self,
        script: &str,
        args: &[String],
        control: &Control,
        timeout: Duration,
        input: &[u8],
    ) -> Result<String> {
        let output = self.observe_input(script, args, control, timeout, input)?;
        anyhow::ensure!(
            matches!(output.termination,Termination::Exited(status) if status.success()),
            "Offline Lean Atlas failed:\n{}\n{}",
            output.stderr,
            output.stdout
        );
        Ok(output.stdout)
    }
}

fn analysis_command(
    execution: &Execution<'_>,
    script: &str,
    args: &[String],
) -> Result<std::process::Command> {
    #[cfg(target_os = "linux")]
    {
        Ok(meden_runtime::sandbox::Sandbox::lean_command(
            execution.root,
            &std::env::current_exe()?,
            execution.lean,
            execution.library,
            execution.sources,
            script,
            args,
        )?)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (
            execution.root,
            execution.lean,
            execution.library,
            execution.sources,
            script,
            args,
        );
        anyhow::bail!("Offline Lean Atlas requires Linux and Bubblewrap")
    }
}

fn engine_hash(root: &Path) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&[
        file_hash(&root.join("bin/lean").canonicalize()?)?,
        file_hash(&root.join("lib/lean/libLake_shared.so").canonicalize()?)?,
    ])?))
}
fn file_hash(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_file() && metadata.len() <= 268_435_456,
        "analysis resource must be a bounded regular file"
    );
    Ok(digest(&fs::read(path)?))
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
