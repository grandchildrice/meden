//! Pinned external GF/Informath renderer over its documented Dedukti interchange.
//! The binary/grammar run offline, separately from Lean and native proof admission.
use anyhow::{Context as _, Result};
use meden_core::adapter::{Adapter, Artifact, Input};
use meden_runtime::process::{self, Control, Limits, Termination};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

#[derive(Deserialize, Serialize)]
struct Manifest {
    version: String,
    files: BTreeMap<String, String>,
}

pub(crate) struct Informath {
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

impl Adapter<Control> for Informath {
    type Error = anyhow::Error;
    fn run(&self, control: &Control, input: Input) -> Result<Artifact> {
        anyhow::ensure!(
            input.format == "application/dedukti" && input.bytes.len() <= 1_048_576,
            "Unsupported meaning renderer input"
        );
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
            &input.bytes,
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
        Ok(Artifact {
            producer: self.identity(),
            format: "informath/parallel-data+jsonl".into(),
            bytes: output.stdout.into_bytes(),
        })
    }
}

pub(crate) fn fingerprint() -> Result<String> {
    Ok(Informath::discover()?.map_or_else(|| "structural-english".into(), |i| i.fingerprint))
}

pub(crate) fn enrich(graph: &mut crate::atlas::Extraction, control: &Control) -> Result<()> {
    let Some(renderer) = Informath::discover()? else {
        return Ok(());
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
        let result = renderer.run(
            control,
            Input {
                format: "application/dedukti".into(),
                bytes: intermediate.as_bytes().to_vec(),
            },
        );
        let result = result.and_then(|artifact| -> Result<(String, String)> {
            let value: serde_json::Value = serde_json::from_slice(&artifact.bytes)?;
            let sentence = value
                .get("InformathEng")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty() && s.len() <= 60_000)
                .context("Informath did not realize this expression")?;
            Ok((
                sentence
                    .strip_prefix("Statement. ")
                    .unwrap_or(sentence)
                    .into(),
                artifact.producer,
            ))
        });
        match result {
            Ok((sentence, producer)) => {
                semantic.sentence = sentence;
                semantic.renderer = Some(producer);
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
    Ok(())
}
