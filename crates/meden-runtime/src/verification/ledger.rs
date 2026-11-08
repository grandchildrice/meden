use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::Bundle;
use crate::process::{self, Control, Limits, Output, Termination};

/// Plain-language meaning shown through the host contract review channel.
pub const LEDGER_CONTRACT: &str = "Rust ledger kernel v1: two accounts in unsigned 64-bit integer units. Only actor 0 may transfer from debit to credit. Request IDs must be strictly greater than the last successful ID; zero amounts, old/repeated IDs, insufficient debit and credit overflow leave all state unchanged. Every permitted transfer moves exactly the amount and records its ID. Total balance is conserved; retries cannot transfer twice. The caller must authenticate the actor and use this transition for state updates. This covers the library transition, not authentication, other state mutations, persistence, concurrency, application parsers or financial certification.";

const MANIFEST: &str = include_str!("../../../../verification/ledger/Cargo.toml");
const LOCK: &str = include_str!("../../../../verification/ledger/Cargo.lock");
const LIBRARY: &str = include_str!("../../../../verification/ledger/lib.rs");
const SPEC: &str = include_str!("../../../../verification/ledger/Spec.lean");
const CHECK: &str = include_str!("../../../../verification/ledger/Check.lean");
const SCRIPT: &str = include_str!("../../../../verification/ledger/check.sh");
const ROOTS: [&str; 8] = [
    "LedgerProof.actual_refines",
    "LedgerContract.conservation",
    "LedgerContract.rejection",
    "LedgerContract.valid_transfer",
    "LedgerContract.unauthorized",
    "LedgerContract.old_request",
    "LedgerContract.repeated_request",
    "LedgerContract.bounded_result",
];

/// Exact checked source, checker inputs and produced file hashes. No model or
/// project-file deserialization constructs a successful report.
#[derive(Clone, Serialize)]
pub struct Report {
    /// Supported accepted contract, not an assertion about the whole project.
    pub contract: &'static str,
    /// Candidate Rust module actually extracted, checked and compiled.
    pub source_sha256: String,
    /// Host-selected compiler, extractor and proof-library content binding.
    pub bundle_sha256: String,
    /// Fixed manifest, contract, examples, proof harness and invocation recipe.
    pub checker_sha256: String,
    /// Actual Meden worker binary supplying the confined syntax checker.
    pub worker_sha256: String,
    /// Hash of this artifact's exact file/hash manifest.
    pub artifact_sha256: String,
    /// Actual deliverable file hashes, including the native Rust library.
    pub files: BTreeMap<String, String>,
    /// Axiom audit of the implementation refinement and contract consequences.
    pub audit: Vec<String>,
    /// Boundaries excluded from the claim.
    pub scope: &'static str,
}

/// A successful check's frozen bytes. The candidate workspace never owns these.
pub struct Checked {
    report: Report,
    files: BTreeMap<String, Vec<u8>>,
}

/// A retained host-owned artifact, produced without rerunning a changed build.
#[derive(Serialize)]
pub struct Artifact {
    /// Local directory containing exactly the bound files and report.json.
    pub directory: PathBuf,
    /// Evidence for this exact artifact.
    pub report: Report,
}

impl Checked {
    /// The checked binding, used by the host evidence controller.
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Persist the already checked bytes outside the candidate's writable copy.
    pub fn persist(self, storage: &Path) -> io::Result<Artifact> {
        fs::create_dir_all(storage)?;
        let directory = tempfile::Builder::new()
            .prefix("ledger-")
            .tempdir_in(storage)?;
        for (name, bytes) in &self.files {
            fs::write(directory.path().join(name), bytes)?;
        }
        fs::write(
            directory.path().join("report.json"),
            serde_json::to_vec_pretty(&self.report)?,
        )?;
        Ok(Artifact {
            directory: directory.keep(),
            report: self.report,
        })
    }
}

/// Validate a candidate's supported syntax in an offline worker, then recheck
/// its actual extracted implementation using fixed trusted proof inputs. A
/// passing candidate test/log alone never creates a successful result.
pub fn verify_ledger(
    source: &str,
    bundle: &Bundle,
    worker: &Path,
    control: &Control,
) -> io::Result<Checked> {
    if source.len() > 65_536 {
        return Err(io::Error::other("ledger source exceeds 64 KiB"));
    }
    let job = tempfile::tempdir()?;
    let sandbox = crate::sandbox::Sandbox::for_verification(job.path(), worker, bundle)?;
    let limits = Limits {
        timeout: Duration::from_secs(240),
        capture_bytes: 65_536,
        output_bytes: 16_777_216,
    };
    let syntax = process::run_with_input(
        &mut sandbox.verification_syntax_command()?,
        source.as_bytes(),
        &limits,
        control,
    )?;
    successful(&syntax, "source syntax")?;
    let worker_hash = hash(&read(worker, 268_435_456)?);
    let fingerprint = bundle.fingerprint(control)?;
    let mut files = BTreeMap::new();
    for (name, text) in [
        ("Cargo.toml", MANIFEST),
        ("Cargo.lock", LOCK),
        ("lib.rs", LIBRARY),
        ("ledger.rs", source),
        ("Spec.lean", SPEC),
        ("Check.lean", CHECK),
    ] {
        fs::write(job.path().join(name), text)?;
        files.insert(name.to_owned(), text.as_bytes().to_vec());
    }
    let script = SCRIPT.replace("@LEAN_PATH@", &bundle.lean_path());
    let output = process::run_with_input(
        &mut sandbox.verification_command(bundle),
        script.as_bytes(),
        &limits,
        control,
    )?;
    successful(&output, "project verification")?;
    let audit = audit(&output.stdout)?;
    if bundle.fingerprint(control)? != fingerprint
        || hash(&read(worker, 268_435_456)?) != worker_hash
        || control.flag().load(Ordering::SeqCst)
    {
        return Err(io::Error::other(
            "checker inputs changed or verification was cancelled",
        ));
    }
    for (name, expected) in &files {
        if read(&job.path().join(name), 131_072)? != *expected {
            return Err(io::Error::other(
                "verification inputs changed while checking",
            ));
        }
    }
    for (name, path, limit) in [
        ("Candidate.lean", "Candidate.lean", 2_097_152),
        ("candidate.llbc", "candidate.llbc", 4_194_304),
        (
            "libmeden_ledger.rlib",
            "build/release/libmeden_ledger.rlib",
            16_777_216,
        ),
    ] {
        files.insert(name.to_owned(), read(&job.path().join(path), limit)?);
    }
    files.insert("check.sh".to_owned(), script.into_bytes());
    let hashes: BTreeMap<_, _> = files
        .iter()
        .map(|(name, bytes)| (name.clone(), hash(bytes)))
        .collect();
    let report = Report { contract:LEDGER_CONTRACT, source_sha256:hash(source.as_bytes()),
        bundle_sha256:fingerprint,checker_sha256:ledger_checker_id(), worker_sha256:worker_hash,
        artifact_sha256:hash(&serde_json::to_vec(&hashes)?), files:hashes, audit,
        scope:"Full functional refinement of the supported u64 ledger kernel and the listed contract properties, conditional on the reviewed contract, trusted preparer/checker libraries, Charon/Aeneas models, Lean kernel, compiler/native libraries, OS confinement and SHA-256. Not persistence, concurrent transactions, whole-application correctness or certification." };
    Ok(Checked { report, files })
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Identity of the fixed contract, syntax protocol and checker recipe.
pub fn ledger_checker_id() -> String {
    let mut digest = Sha256::new();
    for bytes in [
        LEDGER_CONTRACT,
        MANIFEST,
        LOCK,
        LIBRARY,
        SPEC,
        CHECK,
        SCRIPT,
    ] {
        digest.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
        digest.update(bytes.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn successful(output: &Output, operation: &str) -> io::Result<()> {
    if output.truncated
        || !matches!(output.termination, Termination::Exited(code) if code.success())
    {
        return Err(io::Error::other(format!(
            "{operation} not verified ({:?}):\n{}\n{}",
            output.termination,
            diagnostic_excerpt(&output.stdout),
            diagnostic_excerpt(&output.stderr)
        )));
    }
    Ok(())
}

fn diagnostic_excerpt(text: &str) -> String {
    const OMITTED: &str = "\n… checker output omitted …\n";
    const SIDE: usize = (6_000 - OMITTED.len()) / 2;
    if text.len() <= 6_000 {
        return text.to_owned();
    }
    // Preserve the native-test/phase prelude as well as the final failed goal
    // and audit. A long proof failure must not hide how far checking reached.
    let mut head = SIDE;
    while !text.is_char_boundary(head) {
        head = head.saturating_sub(1);
    }
    let mut tail = text.len().saturating_sub(SIDE);
    while !text.is_char_boundary(tail) {
        tail = tail.saturating_add(1);
    }
    format!(
        "{}{OMITTED}{}",
        text.get(..head).unwrap_or_default(),
        text.get(tail..).unwrap_or_default()
    )
}

fn read(path: &Path, maximum: u64) -> io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::other("checker output is not a regular file"));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        return Err(io::Error::other("checker output exceeds budget"));
    }
    Ok(bytes)
}

fn audit(stdout: &str) -> io::Result<Vec<String>> {
    let lines: Vec<_> = stdout
        .lines()
        .filter(|l| l.contains("depends on axioms") || l.contains("does not depend on any axioms"))
        .collect();
    if lines.len() != ROOTS.len() {
        return Err(io::Error::other("proof audit missing or duplicated"));
    }
    for name in ROOTS {
        let prefix = format!("'{name}' ");
        let matches: Vec<_> = lines
            .iter()
            .filter_map(|l| l.strip_prefix(&prefix))
            .collect();
        let [result] = matches.as_slice() else {
            return Err(io::Error::other("exact theorem audit missing"));
        };
        if *result == "does not depend on any axioms" {
            continue;
        }
        let Some(list) = result
            .strip_prefix("depends on axioms: [")
            .and_then(|s| s.strip_suffix(']'))
        else {
            return Err(io::Error::other("unrecognized theorem audit"));
        };
        if list
            .split(", ")
            .any(|a| !["propext", "Classical.choice", "Quot.sound"].contains(&a))
        {
            return Err(io::Error::other("unapproved proof axiom"));
        }
    }
    Ok(lines.into_iter().map(str::to_owned).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_unicode_diagnostics_keep_phase_and_final_failure_within_budget() {
        for padding in 0..4 {
            let text = format!(
                "meden-check: tests\ntest result: ok\n{}{}\nCheck.lean: failed\nsorryAx",
                "x".repeat(padding),
                "証明".repeat(2_000)
            );
            let excerpt = diagnostic_excerpt(&text);
            assert!(excerpt.len() <= 6_000);
            assert!(excerpt.starts_with("meden-check: tests\ntest result: ok\n"));
            assert!(excerpt.ends_with("Check.lean: failed\nsorryAx"));
            assert!(excerpt.contains("… checker output omitted …"));
        }
        assert_eq!(diagnostic_excerpt("short failure"), "short failure");
    }

    #[test]
    fn incomplete_or_forged_axiom_audits_never_become_evidence() {
        let complete = ROOTS
            .iter()
            .map(|name| {
                format!("'{name}' depends on axioms: [propext, Classical.choice, Quot.sound]")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit(&complete).is_ok());
        for invalid in [
            complete.replace("Quot.sound", "sorryAx"),
            complete.replace("Quot.sound", "Candidate.assume_correct"),
            complete.replace("LedgerProof.actual_refines", "LedgerContract.conservation"),
            complete.replace("LedgerProof.actual_refines", "Wrong.actual_refines"),
            complete.lines().skip(1).collect::<Vec<_>>().join("\n"),
            format!("{complete}\n'LedgerProof.actual_refines' does not depend on any axioms"),
        ] {
            assert!(audit(&invalid).is_err());
        }
    }
}
