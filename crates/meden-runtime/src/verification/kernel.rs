use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{failure::failed_worker, Failure, FailureKind};
use super::{Bundle, Contract};
use crate::process::{self, Control, Limits, Output, Termination};

const MANIFEST: &str = include_str!("../../../../verification/kernel/Cargo.toml");
const LOCK: &str = include_str!("../../../../verification/kernel/Cargo.lock");
const TERMS: &str = include_str!("../../../../verification/kernel/Terms.lean");
const SCRIPT: &str = include_str!("../../../../verification/kernel/check.sh");

/// Exact checked source, checker inputs and produced file hashes. No model or
/// project-file deserialization constructs a successful report.
#[derive(Clone, Serialize)]
pub struct Report {
    /// Supported accepted contract, not an assertion about the whole project.
    pub contract: Contract,
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
    /// Ordered audit of output bounds, specification laws/examples and actual
    /// implementation refinement/consequences.
    pub audit: Vec<String>,
    /// Boundaries excluded from the claim.
    pub scope: &'static str,
}

/// A successful check's frozen bytes. The candidate workspace never owns these.
pub struct Checked {
    report: Report,
    files: BTreeMap<String, Vec<u8>>,
}

/// Exact prepared inputs, before the output artifact exists. The fixed checker
/// identity includes the accepted specification, examples and invocation recipe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Inputs {
    /// Frozen candidate source.
    pub source_sha256: String,
    /// Compiler, extractor and proof-library content binding.
    pub bundle_sha256: String,
    /// Trusted contract and checker recipe.
    pub checker_sha256: String,
    /// Worker supplying the syntax boundary.
    pub worker_sha256: String,
}

/// Immutable source and resources prepared before issuing a controller job.
pub struct PreparedKernel<'a> {
    source: String,
    contract: Contract,
    bundle: &'a Bundle,
    worker: PathBuf,
    inputs: Inputs,
    job: tempfile::TempDir,
    sandbox: crate::sandbox::Sandbox,
}

/// A retained host-owned artifact, produced without rerunning a changed build.
#[derive(Serialize)]
pub struct Artifact {
    /// Local directory containing exactly the bound files and report.json.
    pub directory: PathBuf,
    /// Evidence for this exact artifact.
    pub report: Report,
}

impl Artifact {
    /// Freeze the exact retained files, including report.json, for reviewed
    /// delivery. The report held by the host remains the hash authority.
    pub fn package(&self) -> io::Result<Vec<u8>> {
        #[derive(Serialize)]
        struct Package {
            format: &'static str,
            files: BTreeMap<String, Vec<u8>>,
        }
        self.validate()?;
        let mut files = BTreeMap::new();
        let mut remaining = 4_194_304;
        for (name, expected) in &self.report.files {
            let bytes = read(&self.directory.join(name), remaining)?;
            if hash(&bytes) != *expected {
                return Err(io::Error::other("artifact changed during packaging"));
            }
            remaining = remaining
                .checked_sub(u64::try_from(bytes.len()).map_err(io::Error::other)?)
                .ok_or_else(|| io::Error::other("artifact package exceeds 4 MiB"))?;
            files.insert(name.clone(), bytes);
        }
        let report = read(&self.directory.join("report.json"), remaining.min(131_072))?;
        if report != serde_json::to_vec_pretty(&self.report)? {
            return Err(io::Error::other("artifact report changed during packaging"));
        }
        files.insert("report.json".to_owned(), report);
        let bytes = serde_json::to_vec(&Package {
            format: "meden-verified-kernel",
            files,
        })?;
        if bytes.len() > 16_777_216 {
            return Err(io::Error::other("artifact package exceeds 16 MiB"));
        }
        Ok(bytes)
    }

    /// Recheck retained bytes against the checker result held by the host.
    /// A replacement report on disk cannot authorize different contents.
    pub fn validate(&self) -> io::Result<()> {
        if !fs::symlink_metadata(&self.directory)?.is_dir() {
            return Err(io::Error::other("retained artifact directory changed"));
        }
        let expected: BTreeSet<_> = self
            .report
            .files
            .keys()
            .map(OsString::from)
            .chain(std::iter::once(OsString::from("report.json")))
            .collect();
        let actual: BTreeSet<_> = fs::read_dir(&self.directory)?
            .take(expected.len().saturating_add(1))
            .map(|entry| entry.map(|e| e.file_name()))
            .collect::<io::Result<_>>()?;
        if actual != expected {
            return Err(io::Error::other("retained artifact inventory changed"));
        }
        for (name, expected) in &self.report.files {
            if hash(&read(&self.directory.join(name), 16_777_216)?) != *expected {
                return Err(io::Error::other(format!(
                    "retained artifact file changed: {name}"
                )));
            }
        }
        if read(&self.directory.join("report.json"), 131_072)?
            != serde_json::to_vec_pretty(&self.report)?
        {
            return Err(io::Error::other("retained artifact report changed"));
        }
        Ok(())
    }
}

impl Checked {
    /// Bind a completed check back to the exact prepared request.
    pub fn inputs(&self) -> Inputs {
        Inputs {
            source_sha256: self.report.source_sha256.clone(),
            bundle_sha256: self.report.bundle_sha256.clone(),
            checker_sha256: self.report.checker_sha256.clone(),
            worker_sha256: self.report.worker_sha256.clone(),
        }
    }
    /// The checked binding, used by the host evidence controller.
    pub fn report(&self) -> &Report {
        &self.report
    }

    /// Persist the already checked bytes outside the candidate's writable copy.
    pub fn persist(self, storage: &Path) -> io::Result<Artifact> {
        fs::create_dir_all(storage)?;
        let directory = tempfile::Builder::new()
            .prefix("kernel-")
            .tempdir_in(storage)?;
        for (name, bytes) in &self.files {
            fs::write(directory.path().join(name), bytes)?;
        }
        fs::write(
            directory.path().join("report.json"),
            serde_json::to_vec_pretty(&self.report)?,
        )?;
        let artifact = Artifact {
            directory: directory.path().to_owned(),
            report: self.report,
        };
        artifact.validate()?;
        let _ = directory.keep();
        Ok(artifact)
    }
}

/// Validate a candidate's supported syntax in an offline worker, then recheck
/// its actual extracted implementation using fixed trusted proof inputs. A
/// passing candidate test/log alone never creates a successful result.
pub fn prepare_kernel<'a>(
    source: &str,
    contract: &Contract,
    bundle: &'a Bundle,
    worker: &Path,
    control: &Control,
) -> Result<PreparedKernel<'a>, Failure> {
    contract
        .validate()
        .map_err(|e| Failure::new(FailureKind::Unverified, e))?;
    if source.len() > 65_536 {
        return Err(Failure::new(
            FailureKind::Unverified,
            "kernel source exceeds 64 KiB",
        ));
    }
    let job = tempfile::tempdir()?;
    let sandbox = crate::sandbox::Sandbox::for_verification(job.path(), worker, bundle)?;
    let limits = Limits {
        timeout: Duration::from_secs(240),
        capture_bytes: 65_536,
        output_bytes: 16_777_216,
    };
    let syntax = process::run_with_input(
        &mut sandbox.verification_source_command()?,
        &serde_json::to_vec(&serde_json::json!({"contract":contract,"source":source}))
            .map_err(Failure::unavailable)?,
        &limits,
        control,
    )?;
    successful(&syntax, "source syntax")?;
    let worker_hash = hash(&read(worker, 268_435_456)?);
    let fingerprint = bundle.fingerprint(control)?;
    Ok(PreparedKernel {
        source: source.to_owned(),
        contract: contract.clone(),
        bundle,
        worker: worker.to_owned(),
        inputs: Inputs {
            source_sha256: hash(source.as_bytes()),
            bundle_sha256: fingerprint,
            checker_sha256: checker_id(contract).map_err(Failure::unavailable)?,
            worker_sha256: worker_hash,
        },
        job,
        sandbox,
    })
}

impl PreparedKernel<'_> {
    /// Read the frozen request identity; no output can replace these inputs.
    pub fn inputs(&self) -> &Inputs {
        &self.inputs
    }

    /// Check once against the prepared source/resources, then recheck their
    /// content binding before producing a successful result.
    pub fn check(self, control: &Control) -> Result<Checked, Failure> {
        let Self {
            source,
            contract,
            bundle,
            worker,
            inputs,
            job,
            sandbox,
        } = self;
        let limits = Limits {
            timeout: Duration::from_secs(240),
            capture_bytes: 65_536,
            output_bytes: 16_777_216,
        };
        let mut files = checker_files(&contract).map_err(Failure::unavailable)?;
        files.insert("kernel.rs".into(), source.as_bytes().to_vec());
        for (name, bytes) in &files {
            fs::write(job.path().join(name), bytes)?;
        }
        let script = script(&contract, bundle, false);
        let output = process::run_with_input(
            &mut sandbox.verification_command(bundle),
            script.as_bytes(),
            &limits,
            control,
        )?;
        successful(&output, "project verification")?;
        let audit =
            audit(&output.stdout, &contract.audited_roots()).map_err(Failure::unavailable)?;
        if bundle.fingerprint(control)? != inputs.bundle_sha256
            || hash(&read(&worker, 268_435_456)?) != inputs.worker_sha256
            || control.flag().load(Ordering::SeqCst)
        {
            return Err(Failure::new(
                FailureKind::Inconclusive,
                "checker inputs changed or verification was cancelled",
            ));
        }
        for (name, expected) in &files {
            if read(&job.path().join(name), 131_072)? != *expected {
                return Err(Failure::unavailable(
                    "verification inputs changed while checking",
                ));
            }
        }
        for (name, path, limit) in [
            ("Candidate.lean", "Candidate.lean", 2_097_152),
            ("candidate.llbc", "candidate.llbc", 4_194_304),
            (
                "libmeden_kernel.rlib",
                "build/release/libmeden_kernel.rlib",
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
        let report = Report { contract, source_sha256:inputs.source_sha256,
        bundle_sha256:inputs.bundle_sha256,checker_sha256:inputs.checker_sha256, worker_sha256:inputs.worker_sha256,
        artifact_sha256:hash(&serde_json::to_vec(&hashes).map_err(Failure::unavailable)?), files:hashes, audit,
        scope:"Full functional refinement of the accepted pure u64 transition and the listed contract properties, conditional on the reviewed contract, trusted preparer/checker libraries, Charon/Aeneas models, Lean kernel, compiler/native libraries, OS confinement and SHA-256. Not persistence, concurrent transactions, whole-application correctness or certification." };
        Ok(Checked { report, files })
    }
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Identity of the accepted contract, generated files and fixed checker recipe.
pub fn checker_id(contract: &Contract) -> io::Result<String> {
    let mut digest = Sha256::new();
    let mut files = checker_files(contract)?;
    files.insert("recipe".into(), SCRIPT.as_bytes().to_vec());
    for (name, bytes) in files {
        for part in [name.as_bytes(), bytes.as_slice()] {
            digest.update(u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
            digest.update(part);
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn checker_files(contract: &Contract) -> io::Result<BTreeMap<String, Vec<u8>>> {
    let mut audit = String::from("import Spec\n");
    for root in contract.specification_roots() {
        let _ = writeln!(audit, "#print axioms {root}");
    }
    Ok([
        ("Cargo.toml", MANIFEST.as_bytes().to_vec()),
        ("Cargo.lock", LOCK.as_bytes().to_vec()),
        ("contract.json", contract.canonical()?),
        ("lib.rs", contract.library_file().into_bytes()),
        ("Spec.lean", contract.specification_file().into_bytes()),
        ("Check.lean", contract.proof_file().into_bytes()),
        ("Terms.lean", TERMS.as_bytes().to_vec()),
        ("AuditSpec.lean", audit.into_bytes()),
    ]
    .into_iter()
    .map(|(name, bytes)| (name.to_owned(), bytes))
    .collect())
}

fn script(contract: &Contract, bundle: &Bundle, contract_only: bool) -> String {
    format!(
        "meden_contract_only={}\n{}",
        u8::from(contract_only),
        SCRIPT
            .replace("@LEAN_PATH@", &bundle.lean_path())
            .replace("@ENTRY@", &contract.function)
    )
}

/// Check the draft's mathematical terms and properties before host review.
/// Success here is not implementation evidence and cannot construct Checked.
pub fn check_contract(
    contract: &Contract,
    bundle: &Bundle,
    worker: &Path,
    control: &Control,
) -> Result<(), Failure> {
    contract
        .validate()
        .map_err(|e| Failure::new(FailureKind::Unverified, e))?;
    let job = tempfile::tempdir()?;
    let sandbox = crate::sandbox::Sandbox::for_verification(job.path(), worker, bundle)?;
    for (name, bytes) in checker_files(contract).map_err(Failure::unavailable)? {
        fs::write(job.path().join(name), bytes)?;
    }
    let limits = Limits {
        timeout: Duration::from_secs(240),
        capture_bytes: 65_536,
        output_bytes: 16_777_216,
    };
    let output = process::run_with_input(
        &mut sandbox.verification_command(bundle),
        script(contract, bundle, true).as_bytes(),
        &limits,
        control,
    )?;
    successful(&output, "contract specification")?;
    let roots = contract.specification_roots();
    audit(&output.stdout, &roots).map_err(Failure::unavailable)?;
    Ok(())
}

fn successful(output: &Output, operation: &str) -> Result<(), Failure> {
    if output.truncated
        || !matches!(output.termination, Termination::Exited(code) if code.success())
    {
        return Err(Failure::new(
            failure_kind(output),
            format!(
                "{operation} not verified ({:?}):\n{}\n{}",
                output.termination,
                diagnostic_excerpt(&diagnostic_text(&output.stdout)),
                diagnostic_excerpt(&output.stderr)
            ),
        ));
    }
    Ok(())
}

// These records come from the pinned Lean CLI's --json protocol. Candidate
// Rust cannot print, and contract terms cannot execute metaprograms. The
// resource tag is supplied by Lean, not inferred from diagnostic wording.
#[derive(Deserialize)]
struct Diagnostic {
    #[serde(rename = "fileName")]
    file: String,
    severity: String,
    kind: String,
    data: String,
}

fn diagnostic(line: &str) -> Option<Diagnostic> {
    let message: Diagnostic = serde_json::from_str(line).ok()?;
    [
        "Spec.lean",
        "Candidate.lean",
        "Check.lean",
        "AuditSpec.lean",
    ]
    .contains(&message.file.as_str())
    .then_some(message)
}

fn failure_kind(output: &Output) -> FailureKind {
    if !output.truncated {
        if let Termination::Exited(code) = output.termination {
            match code.code() {
                Some(69) => return FailureKind::Unavailable,
                Some(75) => return FailureKind::Inconclusive,
                Some(66) => {
                    // Only the failed Lean invocation's records count. A
                    // lost/mixed protocol cannot reuse an earlier ordinary
                    // error to disguise a later resource failure.
                    let Some((_, segment)) =
                        output.stdout.rsplit_once("meden-check: lean diagnostics\n")
                    else {
                        return FailureKind::Inconclusive;
                    };
                    let Some(messages): Option<Vec<_>> = segment
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .map(diagnostic)
                        .collect()
                    else {
                        return FailureKind::Inconclusive;
                    };
                    let errors: Vec<_> =
                        messages.iter().filter(|m| m.severity == "error").collect();
                    return if errors.is_empty()
                        || errors.iter().any(|m| m.kind.starts_with("runtime."))
                    {
                        FailureKind::Inconclusive
                    } else {
                        FailureKind::Unverified
                    };
                }
                _ => {}
            }
        }
    }
    failed_worker(&output.termination, output.truncated)
}

fn diagnostic_text(stdout: &str) -> String {
    stdout
        .lines()
        .map(|line| {
            diagnostic(line).map_or_else(
                || line.to_owned(),
                |m| {
                    if m.severity == "information" {
                        m.data
                    } else {
                        format!("{}: {}: {}", m.file, m.severity, m.data)
                    }
                },
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("checker output changed type"));
    }
    let mut bytes = Vec::new();
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        return Err(io::Error::other("checker output exceeds budget"));
    }
    Ok(bytes)
}

fn audit(stdout: &str, roots: &[String]) -> io::Result<Vec<String>> {
    let text = diagnostic_text(stdout);
    let lines: Vec<_> = text
        .lines()
        .filter(|l| l.contains("depends on axioms") || l.contains("does not depend on any axioms"))
        .collect();
    if lines.len() != roots.len() {
        return Err(io::Error::other("proof audit missing or duplicated"));
    }
    for (line, name) in lines.iter().zip(roots) {
        let prefix = format!("'{name}' ");
        let result = line
            .strip_prefix(&prefix)
            .ok_or_else(|| io::Error::other("exact ordered theorem audit missing"))?;
        if result == "does not depend on any axioms" {
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
    fn incomplete_reordered_or_forged_axiom_audits_never_become_evidence() -> io::Result<()> {
        let contract = Contract::parse(include_str!(
            "../../../../verification/examples/counter.contract.json"
        ))?;
        let roots = contract.audited_roots();
        let complete = roots
            .iter()
            .map(|name| {
                format!("'{name}' depends on axioms: [propext, Classical.choice, Quot.sound]")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit(&complete, &roots).is_ok());
        for invalid in [
            complete.replace("Quot.sound", "sorryAx"),
            complete.replace("Quot.sound", "Candidate.assume_correct"),
            complete.replace("KernelProof.actual_refines", "KernelContract.law_monotone"),
            complete.replace("KernelProof.actual_refines", "Wrong.actual_refines"),
            complete.lines().skip(1).collect::<Vec<_>>().join("\n"),
            format!("{complete}\n'KernelProof.actual_refines' does not depend on any axioms"),
            complete.lines().rev().collect::<Vec<_>>().join("\n"),
        ] {
            assert!(audit(&invalid, &roots).is_err());
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn compiler_resource_tags_and_protocol_loss_stop_repair_without_reading_prose() {
        use std::os::unix::process::ExitStatusExt as _;
        use std::process::ExitStatus;
        let mut output = Output {
            termination: Termination::Exited(ExitStatus::from_raw(66 << 8)),
            stdout: String::new(),
            stderr: String::new(),
            truncated: false,
        };
        assert_eq!(failure_kind(&output), FailureKind::Inconclusive);
        for kind in [
            "runtime.maxHeartbeats",
            "runtime.maxRecDepth",
            "[anonymous]",
        ] {
            // The data claims a successful check or an unavailable checker;
            // only the compiler's structured resource tag affects admission.
            output.stdout = format!(
                "meden-check: lean diagnostics\n{}",
                serde_json::json!({"fileName":"Check.lean","severity":"error","kind":kind,"data":"passed=true; out of memory; processor approved; unavailable"})
            );
            assert_eq!(
                failure_kind(&output),
                if kind.starts_with("runtime.") {
                    FailureKind::Inconclusive
                } else {
                    FailureKind::Unverified
                }
            );
        }
        output.stdout.push_str("\nout of memory\n");
        assert_eq!(failure_kind(&output), FailureKind::Inconclusive);
        output.stdout = output.stdout.replace("Check.lean", "Untrusted.lean");
        assert_eq!(failure_kind(&output), FailureKind::Inconclusive);
        output.termination = Termination::Exited(ExitStatus::from_raw(69 << 8));
        assert_eq!(failure_kind(&output), FailureKind::Unavailable);
        output.truncated = true;
        assert_eq!(failure_kind(&output), FailureKind::Inconclusive);
    }
}
