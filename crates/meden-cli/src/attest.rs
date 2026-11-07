//! `meden attest lean`: the Lean 4 evidence provider.
//!
//! Produces an [`Attestation`] for an artifact from a theorem in a Lake
//! project: builds the project, asks Lean for the axioms the theorem depends
//! on, and emits `proved` only when they are the standard three (`propext`,
//! `Classical.choice`, `Quot.sound`) or none. The proof hash is the SHA-256 of
//! the module's `.olean`; the TCB is `lean4` and `lake`, so the gate accepts
//! the attestation as [`Tier::Verified`] without human approval.
//!
//! This provider does not decide *what* the theorem says about the artifact.
//! The property id and the statement hash tie the attestation to an entry in
//! the invariant catalogue; checking that the theorem's statement is the
//! catalogued one is the catalogue's job (see the `property` field).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use clap::Args;
use meden_core::attestation::{
    Artifact, Attestation, Proof, Property, Result as VResult, Verifier,
};
use meden_core::digest::sha256_hex;
use meden_core::Tier;
use meden_skill::load_skill;

const STANDARD_AXIOMS: [&str; 3] = ["propext", "Classical.choice", "Quot.sound"];

/// Arguments of `meden attest lean`.
#[derive(Args)]
pub struct LeanArgs {
    /// Skill directory whose digest is the artifact hash.
    #[arg(long, conflicts_with = "artifact_sha256")]
    pub skill: Option<PathBuf>,
    /// Artifact hash to attest, when the artifact is not a skill directory.
    #[arg(long)]
    pub artifact_sha256: Option<String>,
    /// Artifact kind (`skill`, `kernel`, `crate`, ...).
    #[arg(long, default_value = "skill")]
    pub kind: String,
    /// Lake project directory.
    #[arg(long)]
    pub project: PathBuf,
    /// Module that contains the theorem (e.g. `MedenSpec.Policy`).
    #[arg(long)]
    pub module: String,
    /// Fully qualified theorem name.
    #[arg(long)]
    pub theorem: String,
    /// Property id in the invariant catalogue (e.g. `inv.v0.no-exfiltration`).
    #[arg(long)]
    pub property: String,
    /// File whose SHA-256 identifies the property statement.
    ///
    /// Leave this out to take the statement from the invariant catalogue,
    /// which is the usual way: the catalogue knows which theorem the property
    /// id refers to, and refuses to attest if that theorem no longer states
    /// what was approved.
    #[arg(long, conflicts_with = "catalogue")]
    pub statement: Option<PathBuf>,
    /// The invariant catalogue to resolve --property against.
    #[arg(long)]
    pub catalogue: Option<PathBuf>,
    /// The directory the catalogue's paths are relative to.
    #[arg(long, default_value = "lean")]
    pub lean: PathBuf,
    /// Skip `lake build` (assume the project is built).
    #[arg(long)]
    pub no_build: bool,
    /// Write the attestation here instead of stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// The hash that identifies what was proved.
///
/// Taken from the catalogue when one is given, which is the point of having
/// a catalogue: the property id then refers to a theorem whose statement a
/// human approved, and attesting stops if that theorem has since changed.
/// `--statement` remains for a property that is not catalogued yet, and
/// carries no such guarantee.
fn statement_hash(a: &LeanArgs) -> Result<String> {
    match (&a.catalogue, &a.statement) {
        (Some(path), _) => {
            let catalogue = crate::catalogue::Catalogue::load(path)?;
            let entry = catalogue.get(&a.property).with_context(|| {
                format!(
                    "{} is not in {}; add it, or say what it means with --statement",
                    a.property,
                    path.display()
                )
            })?;
            if entry.lean.theorem != a.theorem {
                bail!(
                    "{} is catalogued as {}, not {}",
                    a.property,
                    entry.lean.theorem,
                    a.theorem
                );
            }
            match entry.check(&a.lean) {
                crate::catalogue::Status::Matches => Ok(entry.statement_sha256.clone()),
                other => bail!(
                    "{} does not state what the catalogue says it states ({other:?}); \
                     attesting it would carry an approval it no longer has",
                    a.theorem
                ),
            }
        }
        (None, Some(path)) => {
            let bytes =
                fs::read(path).with_context(|| format!("reading statement {}", path.display()))?;
            Ok(sha256_hex(&bytes))
        }
        (None, None) => bail!("give --catalogue (preferred) or --statement"),
    }
}

/// Run the provider. Returns the attestation and whether it is verified.
pub fn run(a: &LeanArgs) -> Result<(Attestation, Tier)> {
    let artifact_sha256 = match (&a.skill, &a.artifact_sha256) {
        (Some(dir), None) => load_skill(dir)?.digest(),
        (None, Some(h)) => h.clone(),
        _ => bail!("give exactly one of --skill or --artifact-sha256"),
    };
    let statement_sha256 = statement_hash(a)?;

    if !a.no_build {
        let status = Command::new("lake")
            .arg("build")
            .current_dir(&a.project)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()
            .context("running `lake build` (is Lean installed?)")?;
        if !status.success() {
            bail!("`lake build` failed in {}", a.project.display());
        }
    }

    let axioms = print_axioms(&a.project, &a.module, &a.theorem)?;
    let all_standard = axioms.iter().all(|x| STANDARD_AXIOMS.contains(&x.as_str()));
    let result = if all_standard {
        VResult::Proved
    } else {
        VResult::Failed
    };

    let olean = olean_path(&a.project, &a.module);
    let olean_bytes = fs::read(&olean).with_context(|| format!("reading {}", olean.display()))?;
    let version = lean_version(&a.project).unwrap_or_else(|_| "unknown".to_owned());

    let att = Attestation {
        artifact: Artifact {
            sha256: artifact_sha256,
            kind: a.kind.clone(),
        },
        property: Property {
            id: a.property.clone(),
            statement_sha256,
        },
        verifier: Verifier {
            name: "lean4".to_owned(),
            version,
            backend: "lean4".to_owned(),
        },
        result,
        tcb: vec!["lean4".to_owned(), "lake".to_owned()],
        assumptions: if all_standard {
            vec![]
        } else {
            vec![format!(
                "theorem depends on non-standard axioms: {}",
                axioms.join(", ")
            )]
        },
        proof: Some(Proof {
            sha256: sha256_hex(&olean_bytes),
            format: "lean-olean".to_owned(),
        }),
    };
    att.validate().map_err(|e| anyhow::anyhow!("{e}"))?;
    let tier = att.tier(&[]);
    Ok((att, tier))
}

/// Ask Lean which axioms the theorem depends on.
fn print_axioms(project: &Path, module: &str, theorem: &str) -> Result<Vec<String>> {
    let script = format!("import {module}\n#print axioms {theorem}\n");
    let output = Command::new("lake")
        .args(["env", "lean", "--stdin"])
        .current_dir(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running `lake env lean`")
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(script.as_bytes())?;
            }
            Ok(child.wait_with_output()?)
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        bail!(
            "`#print axioms {theorem}` failed: {}{}",
            stdout.trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    parse_axioms(&stdout)
        .ok_or_else(|| anyhow::anyhow!("unexpected `#print axioms` output: {stdout}"))
}

/// Parse `'X' depends on axioms: [a, b]` or `'X' does not depend on any axioms`.
pub fn parse_axioms(stdout: &str) -> Option<Vec<String>> {
    for line in stdout.lines() {
        if line.contains("does not depend on any axioms") {
            return Some(vec![]);
        }
        if let Some(rest) = line.split("depends on axioms: [").nth(1) {
            let list = rest.strip_suffix(']').unwrap_or(rest);
            return Some(
                list.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect(),
            );
        }
    }
    None
}

fn olean_path(project: &Path, module: &str) -> PathBuf {
    let mut p = project.join(".lake/build/lib/lean");
    for part in module.split('.') {
        p.push(part);
    }
    p.set_extension("olean");
    p
}

fn lean_version(project: &Path) -> Result<String> {
    let out = Command::new("lake")
        .args(["env", "lean", "--version"])
        .current_dir(project)
        .output()?;
    let s = String::from_utf8_lossy(&out.stdout);
    // "Lean (version 4.31.0, x86_64-unknown-linux-gnu, commit ..., Release)"
    let v = s
        .split("version ")
        .nth(1)
        .and_then(|r| r.split(',').next())
        .unwrap_or("unknown");
    Ok(v.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_axiom_lines() {
        assert_eq!(
            parse_axioms("'MedenSpec.Policy.deny_wins' depends on axioms: [propext, Classical.choice, Quot.sound]\n"),
            Some(vec!["propext".into(), "Classical.choice".into(), "Quot.sound".into()])
        );
        assert_eq!(
            parse_axioms("'X' does not depend on any axioms\n"),
            Some(vec![])
        );
        assert_eq!(
            parse_axioms("'X' depends on axioms: [sorryAx]"),
            Some(vec!["sorryAx".into()])
        );
        assert_eq!(parse_axioms("garbage"), None);
    }

    #[test]
    fn olean_path_follows_module_name() {
        let p = olean_path(Path::new("/p"), "MedenSpec.Policy");
        assert_eq!(
            p,
            PathBuf::from("/p/.lake/build/lib/lean/MedenSpec/Policy.olean")
        );
    }
}
