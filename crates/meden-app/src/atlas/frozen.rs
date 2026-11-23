//! Content-bound compiled modules retained for independently rerunnable search.
use super::{digest, Configuration};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Modules {
    id: String,
    files: BTreeMap<String, String>,
}
fn files(root: &Path) -> Result<BTreeMap<String, String>> {
    anyhow::ensure!(
        fs::symlink_metadata(root)?.file_type().is_dir(),
        "Compiled module root is not a regular directory"
    );
    let mut result = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    let mut entries = 0usize;
    let mut bytes = 0u64;
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            entries = entries.saturating_add(1);
            anyhow::ensure!(
                entries <= 8192,
                "Compiled module cache exceeds 8192 entries"
            );
            let ty = entry.file_type()?;
            if ty.is_dir() {
                pending.push(entry.path());
            } else {
                anyhow::ensure!(
                    ty.is_file(),
                    "Compiled module cache contains a link or special file"
                );
                let name = entry
                    .path()
                    .strip_prefix(root)?
                    .to_str()
                    .context("Compiled path is not UTF-8")?
                    .to_owned();
                anyhow::ensure!(
                    result.len() < 256,
                    "Compiled module cache exceeds 256 files"
                );
                bytes = bytes
                    .checked_add(entry.metadata()?.len())
                    .context("Compiled byte overflow")?;
                anyhow::ensure!(bytes <= 536_870_912, "Compiled modules exceed 512 MiB");
                result.insert(name, super::file_hash(&entry.path())?);
            }
        }
    }
    anyhow::ensure!(!result.is_empty(), "Missing compiled modules");
    Ok(result)
}
impl Configuration {
    fn modules_directory(&self, id: &str) -> Result<PathBuf> {
        anyhow::ensure!(
            id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid compiled module identity"
        );
        Ok(self
            .cache
            .as_ref()
            .context("Atlas host cache unavailable")?
            .join("compiled-modules")
            .join(id))
    }
    fn modules_id(
        &self,
        files: &BTreeMap<String, String>,
        sources: &BTreeMap<String, String>,
        bindings: &[crate::analysis::layout::Module],
    ) -> Result<String> {
        Ok(digest(&serde_json::to_vec(&(
            self.current_resources()?,
            files,
            sources,
            bindings,
        ))?))
    }
    pub(super) fn retain_modules(
        &self,
        job: &tempfile::TempDir,
        sources: &BTreeMap<String, String>,
        bindings: &[crate::analysis::layout::Module],
    ) -> Result<Modules> {
        let files = files(&job.path().join("modules"))?;
        let id = self.modules_id(&files, sources, bindings)?;
        let target = self.modules_directory(&id)?;
        if target.exists()
            && self::files(&target.join("modules")).is_ok_and(|retained| retained == files)
        {
            return Ok(Modules { id, files });
        }
        {
            fs::create_dir_all(target.parent().context("Compiled cache parent")?)?;
            let quarantine =
                tempfile::tempdir_in(target.parent().context("Compiled cache parent")?)?;
            if target.symlink_metadata().is_ok() {
                fs::rename(&target, quarantine.path().join("invalid"))?;
            }
            fs::rename(job.path(), &target)?;
        }
        Ok(Modules { id, files })
    }
    pub(crate) fn frozen_modules(
        &self,
        snapshot: &crate::analysis::sources::Snapshot,
    ) -> Result<PathBuf> {
        let graph = snapshot.atlas.as_ref().context("Missing compiled graph")?;
        let retained = graph
            .compiled_modules
            .as_ref()
            .context("Missing retained compiled modules; recompile the file")?;
        let bindings = &snapshot
            .collection
            .as_ref()
            .context("Missing module bindings")?
            .modules;
        anyhow::ensure!(
            retained.id == self.modules_id(&retained.files, &snapshot.files, bindings)?,
            "Compiled module input binding changed"
        );
        let root = self.modules_directory(&retained.id)?;
        anyhow::ensure!(
            retained.files == files(&root.join("modules"))?,
            "Retained compiled module bytes changed"
        );
        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_file_bytes_and_empty_directory_inventory_are_bounded() -> Result<()> {
        let root = tempfile::tempdir()?;
        fs::write(root.path().join("M.olean"), b"original")?;
        let original = files(root.path())?;
        fs::write(root.path().join("M.olean"), b"changed")?;
        assert_ne!(original, files(root.path())?);
        for n in 0..8192 {
            fs::create_dir(root.path().join(format!("entry-{n}")))?;
        }
        assert!(files(root.path()).is_err());
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn neither_a_root_link_nor_an_internal_link_is_a_compiled_resource() -> Result<()> {
        let root = tempfile::tempdir()?;
        fs::create_dir(root.path().join("modules"))?;
        fs::write(root.path().join("modules/M.olean"), b"module")?;
        std::os::unix::fs::symlink("modules", root.path().join("alias"))?;
        assert!(files(&root.path().join("alias")).is_err());
        std::os::unix::fs::symlink("M.olean", root.path().join("modules/alias"))?;
        assert!(files(&root.path().join("modules")).is_err());
        Ok(())
    }
}
