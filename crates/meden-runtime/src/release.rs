//! Meden's own release gate. GitHub Actions and the reviewed build workflow are
//! trusted evidence producers; CI reports do not prove the whole application.
//! This API is not registered as a model tool or exposed as a receipt callback.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use meden_core::evidence::{
    EvidenceKind as Kind, EvidenceOrigin as Origin, Gate, Obligation, Receipt, Subject,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Targets built by the reviewed release workflow. All must be present.
pub const TARGETS: [&str; 2] = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"];
const CHECKS: [(&str, Kind); 12] = [
    ("rust (fmt, clippy, test, doc)", Kind::Empirical),
    (
        "cargo-deny (licenses, advisories, sources)",
        Kind::Empirical,
    ),
    (
        "lean (spec builds, no sorry, standard axioms only)",
        Kind::Refinement,
    ),
    ("kani (glob_literal_is_identity)", Kind::Bounded),
    ("kani (star_matches_everything)", Kind::Bounded),
    ("kani (deny_wins)", Kind::Bounded),
    ("kani (allowed_iff_granted_and_not_denied)", Kind::Bounded),
    ("kani (grants_are_monotone)", Kind::Bounded),
    ("kani (scope_rules)", Kind::Bounded),
    (
        "conformance (rust == lean on random cases)",
        Kind::Empirical,
    ),
    (
        "python adapter (ruff, pytest against the binary)",
        Kind::Empirical,
    ),
    (
        "npm package (launcher checks, packs cleanly)",
        Kind::Empirical,
    ),
];
const CI_PATH: &str = ".github/workflows/ci.yml";
const CI_DEFINITION: &[u8] = include_bytes!("../../../.github/workflows/ci.yml");

/// Bound distribution archive, constructed by reading actual staged files.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Staged archive basename.
    pub name: String,
    /// Actual archive SHA-256, not a model or build-log claim.
    pub sha256: String,
    /// Reviewed build provenance accompanying the archive.
    pub build: Build,
}

/// Build-runner provenance; trusted workflow attribution remains an assumption.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Build {
    /// Repository checked out by the build job.
    pub repository: String,
    /// Exact checkout commit used for this target.
    pub commit: String,
    /// Build target from the fixed matrix.
    pub target: String,
    /// Full rustc version recorded by the pinned build invocation.
    pub rustc: String,
    /// Must be the release profile, with Cargo.lock respected.
    pub locked_release: bool,
}

/// Local evidence manifest published only after the gate permits it.
#[derive(Serialize)]
pub struct Report {
    /// All required scoped CI evidence currently matches this exact subject.
    pub release_eligible: bool,
    /// Repository identity.
    pub repository: String,
    /// Bound source checkout.
    pub commit: String,
    /// Exact CI run URL, constructed from validated identifiers.
    pub ci: String,
    /// Attempt checked without selecting older successful attempts.
    pub attempt: u64,
    /// Workflow definition embedded in this gate's build.
    pub ci_definition_sha256: String,
    /// Every archive and the build provenance used in the decision.
    pub artifacts: Vec<Artifact>,
    /// Accepted check names, evidence kinds and exact checker binding bytes.
    pub obligations: Vec<serde_json::Value>,
    /// Explicit limit of what the green release gate means.
    pub scope: &'static str,
}

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Vec<Run>,
}
#[derive(Clone, Deserialize)]
struct Run {
    id: u64,
    head_sha: String,
    head_branch: String,
    event: String,
    path: String,
    #[serde(rename = "run_attempt")]
    attempt: u64,
    status: String,
    conclusion: Option<String>,
    repository: Repository,
}
#[derive(Clone, Deserialize)]
struct Repository {
    full_name: String,
}
#[derive(Deserialize)]
struct Jobs {
    total_count: usize,
    jobs: Vec<Job>,
}
#[derive(Deserialize)]
struct Job {
    id: u64,
    run_id: u64,
    head_sha: String,
    name: String,
    status: String,
    conclusion: Option<String>,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
struct Step {
    status: String,
    conclusion: Option<String>,
}

struct Prepared {
    repository: String,
    commit: String,
    directory: PathBuf,
    artifacts: Vec<Artifact>,
}

/// Check the latest CI attempt for an exact source and the actual staged
/// archives. Only api.github.com is contacted; no receipt JSON is accepted from
/// stdin, models, files or arbitrary API endpoints. The token is never reported.
pub fn verify(
    repository: &str,
    commit: &str,
    directory: &Path,
    token: Option<&str>,
) -> io::Result<Report> {
    let prepared = Prepared::new(repository, commit, directory)?;
    let client = Client::new(token);
    let definition = client.bytes(
        &format!("/repos/{repository}/contents/{CI_PATH}?ref={commit}"),
        "application/vnd.github.raw+json",
    )?;
    if definition != CI_DEFINITION {
        return Err(io::Error::other(
            "source CI definition differs from the gate's accepted checker configuration",
        ));
    }
    let runs: Runs = client.get(&format!(
        "/repos/{repository}/actions/workflows/ci.yml/runs?head_sha={commit}&branch=main&event=push&per_page=1"
    ))?;
    let run = runs
        .workflow_runs
        .first()
        .ok_or_else(|| io::Error::other("no CI run for this source"))?;
    prepared.validate_run(run)?;
    let jobs: Jobs = client.get(&format!(
        "/repos/{repository}/actions/runs/{}/attempts/{}/jobs?per_page=100",
        run.id, run.attempt
    ))?;
    let current: Run = client.get(&format!("/repos/{repository}/actions/runs/{}", run.id))?;
    prepared.finish(run, &jobs, &current)
}

impl Prepared {
    fn new(repository: &str, commit: &str, directory: &Path) -> io::Result<Self> {
        let components: Vec<_> = repository.split('/').collect();
        if components.len() != 2
            || components.iter().any(|part| {
                part.is_empty()
                    || part.len() > 100
                    || matches!(*part, "." | "..")
                    || !part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            })
            || commit.len() != 40
            || !commit
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(io::Error::other(
                "release requires owner/repository and a lowercase 40-hex commit",
            ));
        }
        Ok(Self {
            repository: repository.to_owned(),
            commit: commit.to_owned(),
            directory: directory.to_owned(),
            artifacts: artifacts(repository, commit, directory)?,
        })
    }

    fn validate_run(&self, run: &Run) -> io::Result<()> {
        if run.repository.full_name != self.repository
            || run.head_sha != self.commit
            || run.head_branch != "main"
            || run.event != "push"
            || run.path != CI_PATH
            || run.status != "completed"
            || run.conclusion.as_deref() != Some("success")
            || run.attempt == 0
        {
            return Err(io::Error::other(
                "latest CI must be a successful main push for the exact source and workflow",
            ));
        }
        Ok(())
    }

    fn finish(self, run: &Run, jobs: &Jobs, current: &Run) -> io::Result<Report> {
        self.validate_run(run)?;
        self.validate_run(current)?;
        if current.id != run.id
            || current.attempt != run.attempt
            || jobs.total_count != CHECKS.len()
            || jobs.jobs.len() != CHECKS.len()
        {
            return Err(io::Error::other(
                "CI attempt changed, job page incomplete, or required check set differs",
            ));
        }
        let mut ids = BTreeSet::new();
        for job in &jobs.jobs {
            if !ids.insert(job.id)
                || job.run_id != run.id
                || job.head_sha != self.commit
                || job.status != "completed"
                || job.conclusion.as_deref() != Some("success")
                || job.steps.is_empty()
                || job.steps.iter().any(|step| {
                    step.status != "completed" || step.conclusion.as_deref() != Some("success")
                })
            {
                return Err(io::Error::other(
                    "CI job/result/step does not match a successful current check",
                ));
            }
        }
        let definition = hash(CI_DEFINITION);
        let obligations: Vec<_> = CHECKS
            .iter()
            .map(|(name, kind)| Obligation {
                statement: name.as_bytes().to_vec(),
                kind: *kind,
                checker: format!(
                    "github-actions:{CI_PATH}:{definition}:{}:{}",
                    run.id, run.attempt
                )
                .into_bytes(),
            })
            .collect();
        let contract = serde_json::to_vec(
            &CHECKS
                .iter()
                .map(|(name, kind)| (*name, kind_name(*kind)))
                .collect::<Vec<_>>(),
        )?;
        let subject = self.subject(&contract, &self.artifacts)?;
        let mut gate = Gate::propose(subject.clone(), obligations.clone());
        if !gate.review(Origin::Host) || !gate.accept(Origin::Host) {
            return Err(io::Error::other("release policy acceptance refused"));
        }
        for (position, obligation) in obligations.iter().enumerate() {
            let name = std::str::from_utf8(&obligation.statement).map_err(io::Error::other)?;
            if jobs.jobs.iter().filter(|job| job.name == name).count() != 1 {
                return Err(io::Error::other("missing or duplicate required CI check"));
            }
            let receipt = Receipt {
                subject: subject.clone(),
                position,
                obligation: obligation.clone(),
                passed: true,
                // This synchronous administration command has no live job cancellation.
                active: true,
            };
            if !gate.record(Origin::Checker, &subject, &receipt) {
                return Err(io::Error::other("evidence correspondence refused"));
            }
        }
        // Re-read staged inputs after obtaining evidence. Changed bytes or
        // provenance invalidate all progress before the eligibility decision.
        let fresh = artifacts(&self.repository, &self.commit, &self.directory)?;
        let current_subject = self.subject(&contract, &fresh)?;
        if !gate.refresh(Origin::Controller, current_subject.clone())
            || !gate.eligible(&current_subject)
        {
            return Err(io::Error::other(
                "staged artifacts changed; release evidence is stale",
            ));
        }
        Ok(Report {
            release_eligible: true, repository:self.repository.clone(),commit:self.commit,
            ci:format!("https://github.com/{}/actions/runs/{}",self.repository,run.id),attempt:run.attempt,
            ci_definition_sha256:definition, artifacts:fresh,
            obligations:obligations.iter().map(|o| serde_json::json!({"statement":String::from_utf8_lossy(&o.statement),"kind":kind_name(o.kind),"checker":String::from_utf8_lossy(&o.checker)})).collect(),
            scope:"Successful scoped CI on the bound source and trusted build provenance, under GitHub/OS/compiler/SHA-256 assumptions; not whole-application correctness or user-project verification.",
        })
    }

    fn subject(&self, contract: &[u8], artifacts: &[Artifact]) -> io::Result<Subject> {
        Ok(Subject {
            session: format!("release:{}:{}", self.repository, self.commit).into_bytes(),
            contract: contract.to_vec(),
            inputs: serde_json::to_vec(
                &serde_json::json!({"source":self.commit,"artifacts":artifacts}),
            )?,
        })
    }
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Empirical => "empirical",
        Kind::Bounded => "bounded",
        Kind::Theorem => "theorem",
        Kind::Refinement => "refinement",
    }
}

fn artifacts(repository: &str, commit: &str, directory: &Path) -> io::Result<Vec<Artifact>> {
    if fs::read_dir(directory)?
        .take(TARGETS.len().saturating_mul(3).saturating_add(1))
        .count()
        != TARGETS.len().saturating_mul(3)
    {
        return Err(io::Error::other(
            "staging requires exactly the configured archives, checksum and source sidecars",
        ));
    }
    let mut artifacts = Vec::new();
    for target in TARGETS {
        let name = format!("meden-{target}.tar.gz");
        let file = regular(&directory.join(&name))?;
        if file.metadata()?.len() > 268_435_456 {
            return Err(io::Error::other("archive exceeds 256 MiB"));
        }
        let mut digest = Sha256::new();
        if io::copy(&mut file.take(268_435_457), &mut digest)? > 268_435_456 {
            return Err(io::Error::other("archive exceeded 256 MiB while reading"));
        }
        let sha256 = format!("{:x}", digest.finalize());
        let build: Build = serde_json::from_slice(&read_bounded(
            &directory.join(format!("{name}.source.json")),
            8192,
        )?)?;
        if build.repository != repository
            || build.commit != commit
            || build.target != target
            || !build.locked_release
            || !build.rustc.starts_with("rustc 1.98.1 ")
        {
            return Err(io::Error::other(
                "artifact build provenance does not match source/target/toolchain",
            ));
        }
        let checksum = read_bounded(&directory.join(format!("{name}.sha256")), 256)?;
        if checksum != format!("{sha256}  {name}\n").as_bytes() {
            return Err(io::Error::other("archive checksum mismatch"));
        }
        artifacts.push(Artifact {
            name,
            sha256,
            build,
        });
    }
    Ok(artifacts)
}

fn regular(path: &Path) -> io::Result<File> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::other(
            "release input must be a regular file without symlinks",
        ));
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
        return Err(io::Error::other("release input changed type"));
    }
    Ok(file)
}
fn read_bounded(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    regular(path)?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(io::Error::other("release metadata exceeds budget"));
    }
    Ok(bytes)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct Client<'a> {
    token: Option<&'a str>,
    agent: ureq::Agent,
}
impl<'a> Client<'a> {
    fn new(token: Option<&'a str>) -> Self {
        Self {
            token,
            agent: ureq::Agent::config_builder()
                .proxy(None)
                .max_redirects(0)
                .timeout_global(Some(Duration::from_secs(20)))
                .build()
                .into(),
        }
    }
    fn get<T: DeserializeOwned>(&self, path: &str) -> io::Result<T> {
        serde_json::from_slice(&self.bytes(path, "application/vnd.github+json")?)
            .map_err(io::Error::other)
    }

    fn bytes(&self, path: &str, media_type: &str) -> io::Result<Vec<u8>> {
        let mut request = self
            .agent
            .get(format!("https://api.github.com{path}"))
            .header("user-agent", "meden-release-gate")
            .header("accept", media_type)
            .header("x-github-api-version", "2026-03-10");
        if let Some(token) = self.token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let mut response = request
            .call()
            .map_err(|_| io::Error::other("GitHub evidence request failed; release refused"))?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(2_097_153)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 2_097_152 {
            return Err(io::Error::other("GitHub evidence exceeds response budget"));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    const REPO: &str = "example/meden";
    const COMMIT: &str = "1111111111111111111111111111111111111111";

    fn fixture() -> (tempfile::TempDir, Prepared, Run, Jobs) {
        let root = tempfile::tempdir().expect("staging");
        for target in TARGETS {
            let name = format!("meden-{target}.tar.gz");
            let bytes = format!("component-test artifact {target}");
            fs::write(root.path().join(&name), bytes.as_bytes()).expect("archive");
            fs::write(
                root.path().join(format!("{name}.sha256")),
                format!("{}  {name}\n", hash(bytes.as_bytes())),
            )
            .expect("checksum");
            let build = Build {
                repository: REPO.into(),
                commit: COMMIT.into(),
                target: target.into(),
                rustc: "rustc 1.98.1 (fixture)".into(),
                locked_release: true,
            };
            fs::write(
                root.path().join(format!("{name}.source.json")),
                serde_json::to_vec(&build).expect("metadata"),
            )
            .expect("provenance");
        }
        let prepared = Prepared::new(REPO, COMMIT, root.path()).expect("prepare");
        let run = Run {
            id: 123,
            head_sha: COMMIT.into(),
            head_branch: "main".into(),
            event: "push".into(),
            path: CI_PATH.into(),
            attempt: 2,
            status: "completed".into(),
            conclusion: Some("success".into()),
            repository: Repository {
                full_name: REPO.into(),
            },
        };
        let jobs = Jobs {
            total_count: CHECKS.len(),
            jobs: CHECKS
                .iter()
                .enumerate()
                .map(|(i, (name, _))| Job {
                    id: u64::try_from(i).expect("ID"),
                    run_id: run.id,
                    head_sha: COMMIT.into(),
                    name: (*name).into(),
                    status: "completed".into(),
                    conclusion: Some("success".into()),
                    steps: vec![Step {
                        status: "completed".into(),
                        conclusion: Some("success".into()),
                    }],
                })
                .collect(),
        };
        (root, prepared, run, jobs)
    }

    #[test]
    fn actual_staged_bytes_and_all_current_jobs_are_bound_to_the_report() {
        let (_root, prepared, run, jobs) = fixture();
        let expected = prepared.artifacts.clone();
        let report = prepared.finish(&run, &jobs, &run).expect("gate");
        assert!(report.release_eligible);
        assert_eq!(report.commit, COMMIT);
        assert_eq!(report.artifacts, expected);
        assert_eq!(report.obligations.len(), CHECKS.len());
        assert!(report.scope.contains("not whole-application correctness"));
    }

    #[test]
    fn wrong_source_attempt_incomplete_duplicate_or_failed_evidence_never_releases() {
        for case in 0..9 {
            let (_root, prepared, run, mut jobs) = fixture();
            let mut current = run.clone();
            match case {
                0 => current.head_sha = "2".repeat(40),
                1 => current.attempt = 3,
                2 => jobs.total_count = 100,
                3 => {
                    jobs.jobs.pop();
                }
                4 => {
                    let name = jobs.jobs.first().expect("job").name.clone();
                    jobs.jobs.get_mut(1).expect("job").name = name;
                }
                5 => jobs.jobs.first_mut().expect("job").run_id = 124,
                6 => jobs.jobs.first_mut().expect("job").head_sha = "2".repeat(40),
                7 => jobs.jobs.first_mut().expect("job").conclusion = Some("skipped".into()),
                _ => {
                    jobs.jobs
                        .first_mut()
                        .expect("job")
                        .steps
                        .first_mut()
                        .expect("step")
                        .conclusion = Some("failure".into());
                }
            }
            assert!(
                prepared.finish(&run, &jobs, &current).is_err(),
                "case {case}"
            );
        }
    }

    #[test]
    fn changed_archives_stay_stale_even_when_the_checksum_is_updated() {
        let (root, prepared, run, jobs) = fixture();
        let target = TARGETS.first().expect("target");
        let name = format!("meden-{target}.tar.gz");
        fs::write(root.path().join(&name), b"CHANGED").expect("replace");
        fs::write(
            root.path().join(format!("{name}.sha256")),
            format!("{}  {name}\n", hash(b"CHANGED")),
        )
        .expect("replace checksum");
        assert!(prepared.finish(&run, &jobs, &run).is_err());
    }

    #[test]
    fn provenance_and_extra_files_cannot_substitute_for_the_reviewed_inputs() {
        let (root, _prepared, _run, _jobs) = fixture();
        let target = TARGETS.first().expect("target");
        let name = format!("meden-{target}.tar.gz");
        fs::write(
            root.path().join(format!("{name}.source.json")),
            b"{\"approved\":true}",
        )
        .expect("replace");
        assert!(Prepared::new(REPO, COMMIT, root.path()).is_err());
        let (root, _prepared, _run, _jobs) = fixture();
        fs::write(root.path().join("extra"), b"unknown").expect("extra");
        assert!(Prepared::new(REPO, COMMIT, root.path()).is_err());
        assert!(Prepared::new("../repo", COMMIT, root.path()).is_err());
        assert!(Prepared::new(REPO, "main", root.path()).is_err());
    }
}
