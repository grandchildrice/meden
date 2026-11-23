//! Reuse deterministic evaluation of the exact disposable, frozen Lake inputs.
//! This cache never skips the prepared compiler's build/input fingerprint check.
use super::{super::Manifest, Metadata};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const OUTPUT_LIMIT: usize = 1_048_576;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    inputs: String,
    output: String,
    output_sha256: String,
}

pub(super) fn key(metadata: &Metadata, args: &[String], resources: &Manifest) -> Result<String> {
    Ok(super::super::digest(&serde_json::to_vec(&(
        "frozen-lake-configuration-v1",
        super::super::digest(include_bytes!("../lake.rs")),
        super::super::digest(include_bytes!("cache.rs")),
        resources,
        args,
        &metadata.files,
    ))?))
}

pub(super) fn read(root: &Path, key: &str) -> Result<Option<String>> {
    let Some(entry): Option<Entry> = crate::storage::read(&slot(root, key)?)? else {
        return Ok(None);
    };
    if entry.inputs != key {
        return Ok(None);
    }
    anyhow::ensure!(
        entry.output.len() <= OUTPUT_LIMIT
            && super::super::digest(entry.output.as_bytes()) == entry.output_sha256,
        "Frozen Lake configuration cache binding changed"
    );
    Ok(Some(entry.output))
}

pub(super) fn write(root: &Path, key: &str, output: &str) -> Result<()> {
    // Cache capacity affects reuse only; a complete valid evaluation remains usable.
    if output.len() > OUTPUT_LIMIT {
        return Ok(());
    }
    crate::storage::write(
        &slot(root, key)?,
        &Entry {
            inputs: key.into(),
            output: output.into(),
            output_sha256: super::super::digest(output.as_bytes()),
        },
    )
}

fn slot(root: &Path, key: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid frozen Lake configuration identity"
    );
    // Exactly 4096 replacement slots, without a shared mutable index or eviction
    // scan. A collision is a miss; the full original identity is checked above.
    Ok(root.join(format!(
        "{}.json",
        key.get(..3).context("Missing cache slot")?
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn exact_frozen_files_options_commands_and_resources_select_the_observation() -> Result<()> {
        let metadata = Metadata {
            files: BTreeMap::from([("lakefile.toml".into(), Some("name = 'original'".into()))]),
            ..Metadata::default()
        };
        let resources = Manifest {
            version: 1,
            engine_sha256: "original engine".into(),
            files: BTreeMap::from([("MedenLake.olean".into(), "original evaluator".into())]),
        };
        let args = vec![
            "package".into(),
            String::new(),
            "lakefile.toml".into(),
            "{}".into(),
        ];
        let original = key(&metadata, &args, &resources)?;
        let root = tempfile::tempdir()?;
        write(root.path(), &original, r#"{"name":"original"}"#)?;
        assert_eq!(
            read(root.path(), &original)?.as_deref(),
            Some(r#"{"name":"original"}"#)
        );
        let mut changed = metadata.clone();
        for value in [None, Some("name = 'changed'".into())] {
            changed.files.insert("lakefile.toml".into(), value);
            let next = key(&changed, &args, &resources)?;
            assert_ne!(original, next);
            assert!(read(root.path(), &next)?.is_none());
        }
        changed = metadata.clone();
        changed.files.insert("missing-config.lean".into(), None);
        assert_ne!(original, key(&changed, &args, &resources)?);
        let mut changed_args = args.clone();
        *changed_args.get_mut(3).context("options")? = r#"{"feature":"enabled"}"#.into();
        assert_ne!(original, key(&metadata, &changed_args, &resources)?);
        *changed_args.first_mut().context("operation")? = "manifest".into();
        assert_ne!(original, key(&metadata, &changed_args, &resources)?);
        let changed_resources = Manifest {
            engine_sha256: "changed engine".into(),
            ..resources
        };
        assert_ne!(original, key(&metadata, &args, &changed_resources)?);
        let changed_resources = Manifest {
            files: BTreeMap::from([("MedenLake.olean".into(), "changed evaluator".into())]),
            ..changed_resources
        };
        assert_ne!(original, key(&metadata, &args, &changed_resources)?);
        Ok(())
    }

    #[test]
    fn changed_output_is_refused_and_colliding_inputs_cannot_reuse_it() -> Result<()> {
        let root = tempfile::tempdir()?;
        let key = "a".repeat(64);
        write(root.path(), &key, "original")?;
        let path = slot(root.path(), &key)?;
        let mut entry: Entry = crate::storage::read(&path)?.context("cached evaluation")?;
        entry.output = "forged".into();
        crate::storage::write(&path, &entry)?;
        assert!(read(root.path(), &key).is_err());
        entry.output = "original".into();
        entry.inputs = format!("aaa{}", "b".repeat(61));
        crate::storage::write(&path, &entry)?;
        assert!(read(root.path(), &key)?.is_none());
        write(root.path(), &entry.inputs, "other complete evaluation")?;
        assert_eq!(
            read(root.path(), &entry.inputs)?.as_deref(),
            Some("other complete evaluation")
        );
        assert!(read(root.path(), &key)?.is_none());
        let oversized_key = "c".repeat(64);
        write(root.path(), &oversized_key, &"x".repeat(OUTPUT_LIMIT + 1))?;
        assert!(read(root.path(), &oversized_key)?.is_none());
        assert!(slot(root.path(), "../../../escape").is_err());
        Ok(())
    }
}
