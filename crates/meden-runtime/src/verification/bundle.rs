use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use crate::process::Control;
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Host-prepared public checker resources. Never load this from a model tool
/// argument or a candidate's configuration. The worker mounts only these
/// selected tool/library paths, not the surrounding home/cache directories.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub(crate) version: u32,
    pub(crate) rust: PathBuf,
    pub(crate) lean: PathBuf,
    pub(crate) charon: PathBuf,
    pub(crate) driver: PathBuf,
    pub(crate) aeneas: PathBuf,
    pub(crate) libraries: Vec<PathBuf>,
}

impl Bundle {
    /// Read a bounded, trusted host manifest produced before confidential work.
    pub fn load(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("checker manifest must be a regular file"));
        }
        let mut bytes = Vec::new();
        file.take(16_385).read_to_end(&mut bytes)?;
        if bytes.len() > 16_384 {
            return Err(io::Error::other("checker manifest exceeds budget"));
        }
        let mut bundle: Self = serde_json::from_slice(&bytes)?;
        if bundle.version != 1 || bundle.libraries.len() > 32 {
            return Err(io::Error::other("unsupported checker bundle"));
        }
        for root in [&mut bundle.rust, &mut bundle.lean]
            .into_iter()
            .chain(&mut bundle.libraries)
        {
            *root = root.canonicalize()?;
            if !root.is_dir() {
                return Err(io::Error::other("checker root must be a directory"));
            }
        }
        for binary in [&mut bundle.charon, &mut bundle.driver, &mut bundle.aeneas] {
            *binary = binary.canonicalize()?;
            if !binary.is_file() {
                return Err(io::Error::other(
                    "checker executable must be a regular file",
                ));
            }
        }
        for path in bundle.executables() {
            if !path.is_file() {
                return Err(io::Error::other("prepared checker executable missing"));
            }
        }
        Ok(bundle)
    }

    pub(crate) fn executables(&self) -> Vec<PathBuf> {
        vec![
            self.rust.join("bin/rustc"),
            self.rust.join("bin/cargo"),
            self.lean.join("bin/lean"),
            self.charon.clone(),
            self.driver.clone(),
            self.aeneas.clone(),
        ]
    }

    pub(crate) fn bindings(&self) -> Vec<(PathBuf, String)> {
        let mut bindings = vec![
            (self.rust.clone(), "/verification/rust".into()),
            (self.lean.clone(), "/verification/lean".into()),
            (self.charon.clone(), "/verification/charon".into()),
            (self.driver.clone(), "/verification/charon-driver".into()),
            (self.aeneas.clone(), "/verification/aeneas".into()),
        ];
        bindings.extend(
            self.libraries
                .iter()
                .enumerate()
                .map(|(i, p)| (p.clone(), format!("/verification/library/{i}"))),
        );
        bindings
    }

    pub(crate) fn lean_path(&self) -> String {
        std::iter::once(".".to_owned())
            .chain((0..self.libraries.len()).map(|i| format!("/verification/library/{i}")))
            .collect::<Vec<_>>()
            .join(":")
    }

    /// Content binding for the compiler/sysroot, Lean runtime/libraries and
    /// extraction executables actually selected. Native OS libraries, the
    /// trusted preparer and host immutability remain explicit assumptions.
    pub fn fingerprint(&self, control: &Control) -> io::Result<String> {
        let mut digest = Sha256::new();
        let mut seen = BTreeSet::new();
        for path in self
            .executables()
            .into_iter()
            .chain([self.rust.join("lib"), self.lean.join("lib")])
            .chain(self.libraries.clone())
        {
            hash_path(&path, &mut digest, &mut seen, 0, control)?;
        }
        Ok(format!("{:x}", digest.finalize()))
    }
}

fn hash_path(
    path: &Path,
    digest: &mut Sha256,
    seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    control: &Control,
) -> io::Result<()> {
    if control.flag().load(Ordering::SeqCst) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "checker hashing cancelled",
        ));
    }
    if depth > 64 || seen.len() > 200_000 {
        return Err(io::Error::other("checker closure exceeds traversal budget"));
    }
    let canonical = path.canonicalize()?;
    let label = path.as_os_str().as_encoded_bytes();
    digest.update(u64::try_from(label.len()).unwrap_or(u64::MAX).to_le_bytes());
    digest.update(label);
    let target = canonical.as_os_str().as_encoded_bytes();
    digest.update(
        u64::try_from(target.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    digest.update(target);
    if !seen.insert(canonical.clone()) {
        return Ok(());
    }
    let metadata = fs::metadata(&canonical)?;
    if metadata.is_dir() {
        digest.update(b"directory");
        let mut children = fs::read_dir(&canonical)?
            .map(|entry| entry.map(|e| e.path()))
            .collect::<io::Result<Vec<_>>>()?;
        children.sort();
        for child in children {
            hash_path(&child, digest, seen, depth.saturating_add(1), control)?;
        }
    } else if metadata.is_file() {
        digest.update(b"file");
        digest.update(metadata.len().to_le_bytes());
        let mut file = File::open(canonical)?;
        let mut buffer = vec![0_u8; 65_536];
        loop {
            if control.flag().load(Ordering::SeqCst) {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "checker hashing cancelled",
                ));
            }
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            digest.update(
                buffer
                    .get(..n)
                    .ok_or_else(|| io::Error::other("invalid resource read length"))?,
            );
        }
    } else {
        return Err(io::Error::other("nonregular checker resource"));
    }
    Ok(())
}
