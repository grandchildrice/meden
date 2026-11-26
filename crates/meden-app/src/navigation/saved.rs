//! Disposable saved navigation for viewing before a new observation completes.
//! A saved inventory contains no backend execution context or proof receipt.
use super::collection::File;
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Complete metadata retained independently of the current observation operation.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Inventory<S> {
    /// Original lexical implementation, never replaced when restoring a view.
    pub indexer: String,
    /// Original navigation composition, independent of this storage format.
    pub method: String,
    /// Original ordered file metadata and navigation headings.
    pub files: Vec<File<S>>,
    /// Original explicitly excluded names.
    pub excluded: Vec<String>,
    /// Original geometry weighting policy.
    pub inventory_only: bool,
    /// Original metadata-cache generation; never proof freshness.
    pub generation: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record<T> {
    version: u8,
    sha256: String,
    inventory: T,
}
const LIMIT: u64 = 67_108_864;
struct Bounded<W> {
    inner: W,
    bytes: u64,
}
impl<W: Write> Write for Bounded<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .saturating_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            > LIMIT
        {
            return Err(std::io::Error::other("Saved navigation exceeds 64 MiB"));
        }
        let count = self.inner.write(bytes)?;
        self.bytes = self
            .bytes
            .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
struct Hash(Sha256);
impl Write for Hash {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn digest(value: &impl Serialize) -> Result<String> {
    let mut writer = std::io::BufWriter::with_capacity(
        65_536,
        Bounded {
            inner: Hash(Sha256::new()),
            bytes: 0,
        },
    );
    serde_json::to_writer(&mut writer, value)?;
    writer.flush()?;
    let writer = writer
        .into_inner()
        .map_err(std::io::IntoInnerError::into_error)?;
    Ok(format!("{:x}", writer.inner.0.finalize()))
}
/// Host-scoped, bounded navigation storage, independent of accepted/session state.
pub struct Store {
    path: PathBuf,
}
impl Store {
    /// Select the host's saved presentation, independently of parser updates.
    pub fn host(root: &Path) -> Result<Self> {
        let path = crate::storage::directory(root)?
            .join("navigation")
            .join("inventory.json");
        Ok(Self { path })
    }
    /// Restore complete saved metadata only. Callers validate their own projection
    /// before exposing it and retain a non-current observation state.
    pub fn load<S: DeserializeOwned + Serialize>(&self) -> Result<Option<Inventory<S>>> {
        let _timing = meden_runtime::metrics::Span::start("navigation.saved.load");
        let file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        anyhow::ensure!(
            file.metadata()?.is_file(),
            "Saved navigation is not a regular file"
        );
        let mut bytes = Vec::new();
        file.take(LIMIT.saturating_add(1)).read_to_end(&mut bytes)?;
        anyhow::ensure!(
            u64::try_from(bytes.len())? <= LIMIT,
            "Saved navigation exceeds 64 MiB"
        );
        let saved: Record<Inventory<S>> = serde_json::from_slice(&bytes)?;
        anyhow::ensure!(saved.version == 1, "Unsupported saved navigation format");
        saved.inventory.validate()?;
        anyhow::ensure!(
            saved.sha256 == digest(&saved.inventory)?,
            "Saved navigation content changed"
        );
        Ok(Some(saved.inventory))
    }
    /// Atomically replace a disposable saved view after complete preparation.
    /// Failure leaves the previous complete record, never a successful prefix.
    pub fn retain<S: Serialize>(&self, inventory: &Inventory<S>) -> Result<()> {
        let _timing = meden_runtime::metrics::Span::start("navigation.saved.retain");
        inventory.validate()?;
        let record = Record {
            version: 1,
            sha256: digest(inventory)?,
            inventory,
        };
        let directory = self
            .path
            .parent()
            .context("Saved navigation directory missing")?;
        fs::create_dir_all(directory)?;
        let mut staged = tempfile::NamedTempFile::new_in(directory)?;
        {
            let mut writer = std::io::BufWriter::with_capacity(
                65_536,
                Bounded {
                    inner: staged.as_file_mut(),
                    bytes: 0,
                },
            );
            serde_json::to_writer(&mut writer, &record)?;
            writer.flush()?;
        }
        staged.persist(&self.path).map_err(|e| e.error)?;
        Ok(())
    }
}
impl<S> Inventory<S> {
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            [&self.method, &self.indexer].iter().all(|id| {
                id.len() == 64
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            }),
            "Invalid saved navigation origin"
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests;
