//! Bounded, atomic host checkpoints. No files under the model-writable
//! repository are used to restore conversations or accepted contract meaning.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};

const LIMIT: u64 = 16 * 1024 * 1024;

pub fn directory(root: &Path) -> Result<PathBuf> {
    Ok(meden_runtime::workspace::state_directory(root)?)
}

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let metadata = file.metadata()?;
    anyhow::ensure!(metadata.is_file(), "checkpoint must be a regular file");
    let mut bytes = Vec::new();
    file.take(LIMIT.saturating_add(1)).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "checkpoint exceeds 16 MiB"
    );
    let value = serde_json::from_slice(&bytes)
        .context("Invalid host checkpoint; the saved file has been retained")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        // Opening a repository directly must not expose a writable hard-link
        // alias to its host journal. Publish a fresh inode before using it.
        if metadata.nlink() > 1 {
            write_bytes(path, &bytes)?;
        }
    }
    Ok(Some(value))
}

pub fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    write_bytes(path, &bytes)
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    anyhow::ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "checkpoint exceeds 16 MiB"
    );
    let parent = path.parent().context("checkpoint directory missing")?;
    fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    staged.persist(path).map_err(|error| error.error)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_a_checkpoint_detaches_writable_aliases_before_the_model_can_run() -> Result<()> {
        let root = tempfile::tempdir()?;
        let checkpoint = root.path().join("checkpoint.json");
        let alias = root.path().join("repository-alias.json");
        write(
            &checkpoint,
            &serde_json::json!({"accepted":"original meaning"}),
        )?;
        fs::hard_link(&checkpoint, &alias)?;
        let saved: serde_json::Value = read(&checkpoint)?.context("saved value")?;
        fs::write(&alias, r#"{"accepted":"forged meaning"}"#)?;
        let current: serde_json::Value = read(&checkpoint)?.context("current value")?;
        assert_eq!(saved, current);
        Ok(())
    }
}
