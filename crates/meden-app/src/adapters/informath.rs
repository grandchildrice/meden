//! Pinned external GF/Informath renderer over its documented Dedukti interchange.
//! The binary/grammar run offline, separately from Lean and native proof admission.
use crate::meaning::{Bound, Reading, Renderer};
use anyhow::{Context as _, Result};
use meden_runtime::plugin::{Definition, Descriptor, Role};
use meden_runtime::process::{self, Control, Limits, Termination};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

const ARGUMENTS: &[&str] = &[
    "-parallel-data",
    "-synonyms=1",
    "-symbolics=0",
    "-nbest=1",
    "-sampling=1",
];
const TIMEOUT_SECONDS: u64 = 30;
const OUTPUT_BYTES: usize = 8_388_608;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: String,
    files: BTreeMap<String, String>,
}

pub struct Informath {
    root: PathBuf,
    definition: Descriptor,
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
        Ok(Some(Self::at(root.canonicalize()?)?))
    }
    fn at(root: PathBuf) -> Result<Self> {
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
        let definition = descriptor::<Dedukti>(
            "informath.gf",
            serde_json::json!({"resources":manifest,"arguments":ARGUMENTS,"timeout_seconds":TIMEOUT_SECONDS,"batch_budget_seconds":TIMEOUT_SECONDS,"capture_bytes":OUTPUT_BYTES,"output_bytes":OUTPUT_BYTES}),
        )?;
        Ok(Self { root, definition })
    }
    pub fn identity(&self) -> String {
        format!("Informath 0.4 / GF · {:.12}", self.definition.identity())
    }
}

/// Validated input for the pinned GF/Informath Dedukti interchange.
#[derive(Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct Dedukti(#[schemars(length(min = 1, max = 1_048_576))] String);
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
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(self.definition.clone())
    }
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            Self::at(self.root.clone())?.definition == self.definition,
            "Originally selected Informath resources changed"
        );
        Ok(())
    }
    fn render(&self, control: &Control, input: Dedukti) -> Result<Reading> {
        let scratch = tempfile::tempdir()?;
        let executable = self.root.join("RunInformath");
        let mut command = meden_runtime::sandbox::Sandbox::adapter_command(
            scratch.path(),
            &std::env::current_exe()?,
            &executable,
            &self.root,
            &[("INFORMATH_ROOT", "/adapter")],
            ARGUMENTS,
        )?;
        let output = process::run_with_input(
            &mut command,
            input.0.as_bytes(),
            &Limits {
                timeout: Duration::from_secs(TIMEOUT_SECONDS),
                capture_bytes: OUTPUT_BYTES,
                output_bytes: u64::try_from(OUTPUT_BYTES)?,
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

fn descriptor<E: schemars::JsonSchema>(
    id: &str,
    configuration: serde_json::Value,
) -> Result<Descriptor> {
    let implementation = [
        include_bytes!("informath.rs").as_slice(),
        include_bytes!("../meaning.rs").as_slice(),
    ]
    .concat();
    Ok(Descriptor::new(Definition {
        id,
        role: Role::MeaningRenderer,
        version: "1",
        formats: vec![if id == "informath.gf" {
            "dedukti-v1"
        } else {
            "compiled-explanation-v1"
        }
        .into()],
        implementation: &implementation,
        configuration,
        input: serde_json::to_value(schemars::schema_for!(E))?,
        output: serde_json::to_value(schemars::schema_for!(Reading))?,
    })?)
}

struct Structural {
    definition: Descriptor,
}
impl Renderer for Structural {
    type Expression = crate::explanation::Description;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(self.definition.clone())
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
    fn render(&self, _: &Control, expression: Self::Expression) -> Result<Reading> {
        anyhow::ensure!(
            expression.valid() && expression.checker.is_none(),
            "Invalid structural explanation"
        );
        Ok(Reading {
            sentence: expression.sentence,
            renderer: expression
                .renderer
                .unwrap_or_else(|| "Structural English".into()),
        })
    }
}

enum Selected {
    Informath(Bound<Informath>),
    Structural(Bound<Structural>),
}
/// One host-selected renderer retained through cache lookup and publication.
pub struct Prepared(Selected);
impl Prepared {
    pub fn discover() -> Result<Self> {
        match Informath::discover()? {
            Some(renderer) => Ok(Self(Selected::Informath(Bound::new(renderer)?))),
            None => Ok(Self(Selected::Structural(Bound::new(Structural {
                definition: descriptor::<<Structural as Renderer>::Expression>(
                    "structural.english",
                    serde_json::json!({"method":"retain-original-compiler-reading"}),
                )?,
            })?))),
        }
    }
    pub fn definition(&self) -> &Descriptor {
        match &self.0 {
            Selected::Informath(r) => r.definition(),
            Selected::Structural(r) => r.definition(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        match &self.0 {
            Selected::Informath(r) => r.validate(),
            Selected::Structural(r) => r.validate(),
        }
    }
    pub fn enrich(&self, graph: &mut crate::atlas::Extraction, control: &Control) -> Result<()> {
        let _timing = meden_runtime::metrics::Span::start("analysis.render");
        match &self.0 {
            Selected::Structural(renderer) => renderer.batch(control, |batch| {
                for node in &mut graph.graph.nodes {
                    if let Some(semantic) = node.semantic.as_mut() {
                        let mut expression = semantic.clone();
                        expression.checker = None;
                        let reading = batch.render(expression)?;
                        semantic.sentence = reading.sentence;
                        semantic.renderer = Some(reading.renderer);
                    }
                }
                Ok(())
            }),
            Selected::Informath(renderer) => renderer.batch(control, |batch| {
                let started = std::time::Instant::now();
                for node in &mut graph.graph.nodes {
                    let Some(semantic) = node.semantic.as_mut() else { continue };
                    let Some(intermediate) = semantic.intermediate.as_ref() else { continue };
                    if started.elapsed() >= Duration::from_secs(TIMEOUT_SECONDS) {
                        semantic.rendering_error = Some("Informath's preparation budget was exhausted; the structural reading is retained.".into());
                        continue;
                    }
                    let result = Dedukti::new(intermediate).and_then(|term| batch.render(term));
                    match result {
                        Ok(reading) => { semantic.sentence = reading.sentence; semantic.renderer = Some(reading.renderer); }
                        Err(error) => {
                            control.with_active(|| Ok(()))?;
                            semantic.renderer = Some("Structural English (Informath unavailable)".into());
                            let detail = format!("{error:#}");
                            semantic.rendering_error = Some(if detail.len() <= 65_536 { detail } else { "Informath failed; diagnostic exceeds 64 KiB".into() });
                        }
                    }
                }
                Ok(())
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_preflight_never_relabels_a_valid_resource_replacement_as_the_original() -> Result<()>
    {
        // Resource preflight only: no fixture bytes are executed as a GF engine.
        let root = tempfile::tempdir()?;
        std::fs::create_dir(root.path().join("share"))?;
        let mut manifest = Manifest {
            version: "informath-0.4".into(),
            files: BTreeMap::new(),
        };
        for path in [
            "RunInformath",
            "share/InformathEng.pgf",
            "share/baseconstants.dk",
            "share/baseconstants.dkgf",
        ] {
            std::fs::write(root.path().join(path), path)?;
            manifest
                .files
                .insert(path.into(), crate::atlas::digest(path.as_bytes()));
        }
        crate::storage::write(&root.path().join("manifest.json"), &manifest)?;
        let original = Bound::new(Informath::at(root.path().into())?)?;
        let identity = original.definition().identity().to_owned();
        original.validate()?;
        let changed = b"different, independently valid resource bytes";
        std::fs::write(root.path().join("share/baseconstants.dk"), changed)?;
        manifest.files.insert(
            "share/baseconstants.dk".into(),
            crate::atlas::digest(changed),
        );
        crate::storage::write(&root.path().join("manifest.json"), &manifest)?;
        assert!(original.validate().is_err());
        let replacement = Bound::new(Informath::at(root.path().into())?)?;
        assert_ne!(identity, replacement.definition().identity());
        assert_eq!(original.definition().identity(), identity);
        Ok(())
    }
}
