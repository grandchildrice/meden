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

mod graph;
mod imports;
mod resources;
pub(crate) use graph::Extraction;
pub(crate) const UPSTREAM: &str = "06be1eb76fb926b8cf8bd42a4a51208d128c327d";

#[derive(Clone)]
pub(crate) struct Configuration {
    lean: Option<PathBuf>,
    cache: Option<PathBuf>,
}
#[derive(Serialize)]
pub(crate) struct Status {
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
    pub fn host() -> Self {
        let lean = std::env::var_os("MEDEN_ATLAS_LEAN")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| {
                    PathBuf::from(home).join(".elan/toolchains/leanprover--lean4---v4.35.0")
                })
            });
        let cache = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .filter(|path| path.is_absolute())
            .map(|root| root.join("meden/atlas"));
        Self { lean, cache }
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
        Ok(digest(&serde_json::to_vec(&Self::validate(
            &lean, &library,
        )?)?))
    }
    fn selected(&self) -> Result<(PathBuf, PathBuf)> {
        anyhow::ensure!(
            cfg!(target_os = "linux"),
            "Offline Lean Atlas requires Linux and Bubblewrap"
        );
        let lean = self.lean.as_ref().context("Select installed Lean with MEDEN_ATLAS_LEAN on the host")?.canonicalize().context("Lean analysis toolchain is not installed; set MEDEN_ATLAS_LEAN to an installed distribution")?;
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
        };
        execution.execute("prepare.sh", &modules, control, Duration::from_secs(180))?;
        resources::write(output.path())?;
        let files = resources::modules()
            .iter()
            .map(|module| {
                let name = format!("{module}.olean");
                Ok((name.clone(), file_hash(&output.path().join(name))?))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
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
    fn validate(lean: &Path, library: &Path) -> Result<Manifest> {
        let manifest: Manifest = crate::storage::read(&library.join("manifest.json"))?
            .context("Lean Atlas is not prepared")?;
        anyhow::ensure!(
            manifest.version == 1 && manifest.engine_sha256 == engine_hash(lean)?,
            "Lean analysis engine changed; prepare again"
        );
        anyhow::ensure!(
            manifest.files.len() == resources::modules().len(),
            "Atlas library manifest is incomplete"
        );
        for module in resources::modules() {
            let name = format!("{module}.olean");
            anyhow::ensure!(
                manifest.files.get(&name) == Some(&file_hash(&library.join(&name))?),
                "Atlas compiled library changed; prepare again"
            );
        }
        resources::validate(library)?;
        Ok(manifest)
    }
    pub fn extract(
        &self,
        files: &BTreeMap<String, String>,
        control: &Control,
    ) -> Result<Extraction> {
        let (lean, library) = self.selected()?;
        let before = Self::validate(&lean, &library)?;
        let job = tempfile::tempdir()?;
        let frozen = tempfile::tempdir()?;
        let started = Instant::now();
        let mut modules = Vec::new();
        for (path, source) in files {
            modules.push(module_path(path)?);
            let target = frozen.path().join(path);
            fs::create_dir_all(target.parent().context("source parent missing")?)?;
            fs::write(target, source)?;
        }
        let execution = Execution {
            root: job.path(),
            lean: &lean,
            library: &library,
            sources: Some(frozen.path()),
        };
        execution.compile(&modules, control, started)?;
        let stdout = execution.execute("extract.sh", &modules, control, remaining(started)?)?;
        anyhow::ensure!(
            serde_json::to_vec(&before)? == serde_json::to_vec(&Self::validate(&lean, &library)?)?,
            "Atlas resources changed during extraction"
        );
        let graph: Value = serde_json::from_str(&stdout)
            .context("Atlas did not return one complete JSON result")?;
        Extraction::new(graph, files, &before)
    }
}
fn module_path(path: &str) -> Result<String> {
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
            Some("LeanAtlas" | "MedenAtlas" | "Lean" | "Init" | "Std")
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
}
impl Execution<'_> {
    fn compile(&self, modules: &[String], control: &Control, started: Instant) -> Result<()> {
        let mut pending = modules.to_vec();
        while !pending.is_empty() {
            let previous = pending.len();
            let mut next = Vec::new();
            let mut diagnostics = String::new();
            for module in pending {
                let output = self.observe(
                    "compile.sh",
                    std::slice::from_ref(&module),
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
        let mut command = analysis_command(self, script, args)?;
        let output = process::run(
            &mut command,
            &Limits {
                timeout,
                capture_bytes: if script == "extract.sh" {
                    8_388_608
                } else {
                    32768
                },
                output_bytes: 16_777_216,
            },
            control,
        )?;
        anyhow::ensure!(
            matches!(output.termination, Termination::Exited(_)) && !output.truncated,
            "Offline Lean Atlas stopped: {:?}; output was not admitted",
            output.termination
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
        let output = self.observe(script, args, control, timeout)?;
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
    file_hash(&root.join("bin/lean").canonicalize()?)
}
fn file_hash(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_file() && metadata.len() <= 268_435_456,
        "analysis resource must be a bounded regular file"
    );
    Ok(digest(&fs::read(path)?))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
