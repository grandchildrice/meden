//! Common retained checker outputs. Only backend admission constructs Checked.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};
/// Evidence strength carried by the application wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// Observations of named executions.
    Empirical,
    /// Complete checking within the named finite scope.
    Bounded,
    /// A checked theorem under named assumptions.
    Theorem,
    /// Implementation correspondence to a specification.
    Refinement,
}
impl Class {
    /// The existing kernel kind; this conversion does not grant evidence.
    pub fn core(self) -> meden_core::evidence::EvidenceKind {
        use meden_core::evidence::EvidenceKind;
        match self {
            Self::Empirical => EvidenceKind::Empirical,
            Self::Bounded => EvidenceKind::Bounded,
            Self::Theorem => EvidenceKind::Theorem,
            Self::Refinement => EvidenceKind::Refinement,
        }
    }
}
/// Exact checked source, checker inputs and produced file hashes. No model or
/// project-file deserialization constructs a successful report.
#[derive(Clone, Serialize)]
pub struct Report {
    /// Supported accepted contract, not an assertion about the whole project.
    pub contract: serde_json::Value,
    /// Exact canonical accepted bytes, independently of JSON object ordering.
    pub contract_sha256: String,
    /// Host registered implementation identity.
    pub backend: String,
    /// Logical strength, never promoted by the viewer.
    pub class: Class,
    /// Exact candidate source actually checked.
    pub source_sha256: String,
    /// Exact stored source filename in this artifact.
    pub source_file: String,
    /// Host-selected engine and complete prepared resource content binding.
    pub bundle_sha256: String,
    /// Fixed contract, method and invocation recipe.
    pub checker_sha256: String,
    /// Actual Meden binary supplying the confined worker launcher.
    pub worker_sha256: String,
    /// Hash of this artifact's exact file/hash manifest.
    pub artifact_sha256: String,
    /// Exact retained deliverable file hashes.
    pub files: BTreeMap<String, String>,
    /// Ordered audit of output bounds, specification laws/examples and actual
    /// implementation refinement/consequences.
    pub audit: Vec<String>,
    /// Boundaries excluded from the claim.
    pub scope: String,
}

/// A successful check's frozen bytes. The candidate workspace never owns these.
pub struct Checked {
    pub(super) report: Report,
    pub(super) files: BTreeMap<String, Vec<u8>>,
}

/// Exact prepared inputs, before the output artifact exists. The fixed checker
/// identity includes the accepted specification, examples and invocation recipe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Inputs {
    /// Frozen candidate source.
    pub source_sha256: String,
    /// Selected engine and prepared resource content binding.
    pub bundle_sha256: String,
    /// Trusted contract and checker recipe.
    pub checker_sha256: String,
    /// Worker supplying the syntax boundary.
    pub worker_sha256: String,
}

/// A retained host-owned artifact, produced without rerunning a changed build.
#[derive(Serialize)]
pub struct Artifact {
    /// Local directory containing exactly the bound files and report.json.
    pub directory: PathBuf,
    /// Evidence for this exact artifact.
    pub report: Report,
}

/// Validated files staged on disk, removed unless the controller admits them.
pub struct StagedArtifact {
    artifact: Artifact,
    directory: tempfile::TempDir,
}

impl StagedArtifact {
    /// Retain precisely these staged files after the host controller's admission.
    pub fn commit(self) -> Artifact {
        let Self {
            artifact,
            directory,
        } = self;
        let _ = directory.keep();
        artifact
    }
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
            format: "meden-checked-artifact",
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
        if self.report.files.get(&self.report.source_file) != Some(&self.report.source_sha256)
            || self.report.files.get("contract.json") != Some(&self.report.contract_sha256)
            || hash(&serde_json::to_vec(&self.report.files)?) != self.report.artifact_sha256
            || serde_json::from_slice::<serde_json::Value>(&read(
                &self.directory.join("contract.json"),
                65_536,
            )?)? != self.report.contract
        {
            return Err(io::Error::other(
                "artifact source/contract/manifest binding changed",
            ));
        }
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

    /// Write and validate the checked bytes before evidence admission. Dropping
    /// the returned stage rolls it back without retaining an unadmitted artifact.
    pub fn stage(self, storage: &Path) -> io::Result<StagedArtifact> {
        if self.files.len() > 64
            || self.files.keys().any(|name| {
                name.is_empty()
                    || name.len() > 128
                    || name.contains('/')
                    || name.contains('\\')
                    || matches!(name.as_str(), "." | "..")
            })
        {
            return Err(io::Error::other("invalid checked artifact file inventory"));
        }
        fs::create_dir_all(storage)?;
        let directory = tempfile::Builder::new()
            .prefix("checked-")
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
        Ok(StagedArtifact {
            artifact,
            directory,
        })
    }
}

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_artifacts_roll_back_on_refusal_and_retain_exact_bytes_only_on_commit(
    ) -> io::Result<()> {
        let storage = tempfile::tempdir()?;
        let checked = || -> io::Result<Checked> {
            let files = BTreeMap::from([
                ("kernel.rs".to_owned(), b"staged bytes".to_vec()),
                ("contract.json".into(), b"{}".to_vec()),
            ]);
            let hashes = files
                .iter()
                .map(|(name, bytes)| (name.clone(), hash(bytes)))
                .collect::<BTreeMap<_, _>>();
            Ok(Checked {
                report: Report {
                    contract: serde_json::json!({}),
                    contract_sha256: hash(b"{}"),
                    backend: "storage fixture".into(),
                    class: Class::Bounded,
                    source_sha256: hash(b"staged bytes"),
                    source_file: "kernel.rs".into(),
                    bundle_sha256: "test bundle".into(),
                    checker_sha256: "test checker".into(),
                    worker_sha256: "test worker".into(),
                    artifact_sha256: hash(&serde_json::to_vec(&hashes)?),
                    files: hashes,
                    audit: Vec::new(),
                    scope: "storage adapter test, not a verification receipt".into(),
                },
                files,
            })
        };
        let staged = checked()?.stage(storage.path())?;
        let rejected = staged.artifact.directory.clone();
        staged.artifact.validate()?;
        drop(staged);
        assert!(!rejected.exists());
        let artifact = checked()?.stage(storage.path())?.commit();
        assert_eq!(
            fs::read(artifact.directory.join("kernel.rs"))?,
            b"staged bytes"
        );
        artifact.validate()?;
        Ok(())
    }
}
