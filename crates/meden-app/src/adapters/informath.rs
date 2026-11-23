//! Pinned external GF/Informath renderer over its documented Dedukti interchange.
//! The binary/grammar run offline, separately from Lean and native proof admission.
use crate::meaning::{Reading, Renderer};
use anyhow::{Context as _, Result};
use meden_runtime::process::{self, Control, Limits, Termination};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

#[derive(Deserialize, Serialize)]
struct Manifest {
    version: String,
    files: BTreeMap<String, String>,
}

pub struct Informath {
    root: PathBuf,
    fingerprint: String,
}

impl Informath {
    pub fn discover() -> Result<Option<Self>> {
        let root = std::env::var_os("MEDEN_INFORMATH_ROOT")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(|h| PathBuf::from(h).join(".local/share/meden/informath/0.4"))
            });
        let Some(root) = root else { return Ok(None) };
        if !root.exists() && std::env::var_os("MEDEN_INFORMATH_ROOT").is_none() {
            return Ok(None);
        }
        let root = root.canonicalize()?;
        let manifest: Manifest = crate::storage::read(&root.join("manifest.json"))?
            .context("Informath is not prepared; run tools/prepare-informath.py")?;
        anyhow::ensure!(
            manifest.version == "informath-0.4" && manifest.files.len() == 4,
            "Unsupported Informath resource manifest"
        );
        for path in [
            "RunInformath",
            "share/InformathEng.pgf",
            "share/baseconstants.dk",
            "share/baseconstants.dkgf",
        ] {
            let actual = std::fs::read(root.join(path))?;
            anyhow::ensure!(
                manifest.files.get(path) == Some(&crate::atlas::digest(&actual)),
                "Informath resource changed: {path}"
            );
        }
        let fingerprint = crate::atlas::digest(&serde_json::to_vec(&(
            &manifest,
            include_str!("informath.rs"),
        ))?);
        Ok(Some(Self { root, fingerprint }))
    }
    pub fn identity(&self) -> String {
        format!("Informath 0.4 / GF · {}", &self.fingerprint[..12])
    }
}

/// Validated input for the pinned GF/Informath Dedukti interchange.
pub struct Dedukti(String);
impl Dedukti {
    pub fn new(term: &str) -> Result<Self> {
        anyhow::ensure!(
            !term.is_empty() && term.len() <= 1_048_576,
            "Unsupported meaning renderer input"
        );
        Ok(Self(term.into()))
    }
}
impl Renderer for Informath {
    type Expression = Dedukti;
    fn render(&self, control: &Control, input: Dedukti) -> Result<Reading> {
        let scratch = tempfile::tempdir()?;
        let executable = self.root.join("RunInformath");
        let mut command = meden_runtime::sandbox::Sandbox::adapter_command(
            scratch.path(),
            &std::env::current_exe()?,
            &executable,
            &self.root,
            &[("INFORMATH_ROOT", "/adapter")],
            &[
                "-parallel-data",
                "-synonyms=1",
                "-symbolics=0",
                "-nbest=1",
                "-sampling=1",
            ],
        )?;
        let output = process::run_with_input(
            &mut command,
            input.0.as_bytes(),
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 8_388_608,
                output_bytes: 8_388_608,
            },
            control,
        )?;
        anyhow::ensure!(
            matches!(output.termination, Termination::Exited(status) if status.success())
                && !output.truncated,
            "Informath rendering did not complete: {:?}: {}",
            output.termination,
            output.stderr
        );
        let value: serde_json::Value = serde_json::from_str(&output.stdout)?;
        let sentence = value
            .get("InformathEng")
            .and_then(|v| v.as_array())
            .and_then(|v| v.first())
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && s.len() <= 60_000)
            .context("Informath did not realize this expression")?;
        Ok(Reading {
            sentence: sentence
                .strip_prefix("Statement. ")
                .unwrap_or(sentence)
                .into(),
            renderer: self.identity(),
        })
    }
}

pub fn fingerprint() -> Result<String> {
    Ok(Informath::discover()?.map_or_else(|| "structural-english".into(), |i| i.fingerprint))
}

pub fn enrich(graph: &mut crate::atlas::Extraction, control: &Control) -> Result<String> {
    let _timing = meden_runtime::metrics::Span::start("analysis.render");
    let Some(renderer) = Informath::discover()? else {
        anyhow::ensure!(
            fingerprint()? == "structural-english",
            "Meaning renderer resources changed during preparation"
        );
        return Ok("structural-english".into());
    };
    let started = std::time::Instant::now();
    // Each declaration is an independent observation, so an unsupported domain
    // does not replace a valid structural rendering or shift another result.
    for node in &mut graph.graph.nodes {
        let Some(semantic) = node.semantic.as_mut() else {
            continue;
        };
        let Some(intermediate) = semantic.intermediate.as_ref() else {
            continue;
        };
        if started.elapsed() >= Duration::from_secs(30) {
            semantic.rendering_error = Some(
                "Informath's preparation budget was exhausted; the structural reading is retained."
                    .into(),
            );
            continue;
        }
        let result = Dedukti::new(intermediate).and_then(|term| renderer.render(control, term));
        match result {
            Ok(reading) => {
                semantic.sentence = reading.sentence;
                semantic.renderer = Some(reading.renderer);
            }
            Err(error) => {
                control.with_active(|| Ok(()))?;
                semantic.renderer = Some("Structural English (Informath unavailable)".into());
                let detail = format!("{error:#}");
                semantic.rendering_error = Some(if detail.len() <= 65_536 {
                    detail
                } else {
                    "Informath failed; diagnostic exceeds 64 KiB".into()
                });
            }
        }
    }
    anyhow::ensure!(
        fingerprint()? == renderer.fingerprint,
        "Meaning renderer resources changed during preparation"
    );
    Ok(renderer.fingerprint)
}
