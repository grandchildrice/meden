//! Exact host-owned analysis sources, compiled in their dependency order.
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};
const SOURCES: &[(&str, &str)] = &[
    (
        "MedenPlausible/Random",
        include_str!("vendor/MedenPlausible/Random.lean"),
    ),
    (
        "MedenPlausible/Gen",
        include_str!("vendor/MedenPlausible/Gen.lean"),
    ),
    (
        "MedenPlausible/Shrinkable",
        include_str!("vendor/MedenPlausible/Shrinkable.lean"),
    ),
    (
        "MedenPlausible/Arbitrary",
        include_str!("vendor/MedenPlausible/Arbitrary.lean"),
    ),
    (
        "MedenPlausible/Sampleable",
        include_str!("vendor/MedenPlausible/Sampleable.lean"),
    ),
    (
        "MedenPlausible/Testable",
        include_str!("vendor/MedenPlausible/Testable.lean"),
    ),
    ("MedenSemantic", include_str!("MedenSemantic.lean")),
    ("MedenSearch", include_str!("MedenSearch.lean")),
    ("MedenSearchMain", include_str!("MedenSearchMain.lean")),
    ("MedenInterchange", include_str!("MedenInterchange.lean")),
    (
        "LeanAtlas/Config/Types",
        include_str!("vendor/LeanAtlas/Config/Types.lean"),
    ),
    (
        "LeanAtlas/Metadata/Core",
        include_str!("vendor/LeanAtlas/Metadata/Core.lean"),
    ),
    (
        "LeanAtlas/Metadata/Attribute/Confidence",
        include_str!("vendor/LeanAtlas/Metadata/Attribute/Confidence.lean"),
    ),
    (
        "LeanAtlas/Metadata/Attribute/ProofProgress",
        include_str!("vendor/LeanAtlas/Metadata/Attribute/ProofProgress.lean"),
    ),
    (
        "LeanAtlas/Metadata/Attribute/DefProgress",
        include_str!("vendor/LeanAtlas/Metadata/Attribute/DefProgress.lean"),
    ),
    (
        "LeanAtlas/Metadata/Attribute/Meta",
        include_str!("vendor/LeanAtlas/Metadata/Attribute/Meta.lean"),
    ),
    (
        "LeanAtlas/Metadata/Extensions",
        include_str!("vendor/LeanAtlas/Metadata/Extensions.lean"),
    ),
    (
        "LeanAtlas/Metadata/Linter",
        include_str!("vendor/LeanAtlas/Metadata/Linter.lean"),
    ),
    (
        "LeanAtlas/Config/Parse",
        include_str!("vendor/LeanAtlas/Config/Parse.lean"),
    ),
    (
        "LeanAtlas/Config/Lakefile",
        include_str!("vendor/LeanAtlas/Config/Lakefile.lean"),
    ),
    (
        "LeanAtlas/Config/Load",
        include_str!("vendor/LeanAtlas/Config/Load.lean"),
    ),
    ("LeanAtlas", include_str!("vendor/LeanAtlas.lean")),
    (
        "LeanAtlas/Deps/Core",
        include_str!("vendor/LeanAtlas/Deps/Core.lean"),
    ),
    (
        "LeanAtlas/GraphData/Core",
        include_str!("vendor/LeanAtlas/GraphData/Core.lean"),
    ),
    (
        "LeanAtlas/GraphData/Json",
        include_str!("vendor/LeanAtlas/GraphData/Json.lean"),
    ),
    ("MedenAtlas", include_str!("MedenAtlas.lean")),
    ("MedenImports", include_str!("MedenImports.lean")),
    ("MedenLake", include_str!("MedenLake.lean")),
];
const SCRIPTS: &[(&str, &str)] = &[
    (
        "PLAUSIBLE-LICENSE",
        include_str!("vendor/MedenPlausible/LICENSE"),
    ),
    ("LEAN-ATLAS-LICENSE", include_str!("vendor/LICENSE")),
    ("search-atlas.py", include_str!("search-atlas.py")),
    ("search.sh", include_str!("search.sh")),
    ("project-search.sh", include_str!("project-search.sh")),
    (
        "project-atlas-query.sh",
        include_str!("project-atlas-query.sh"),
    ),
    (
        "project-atlas-extract.sh",
        include_str!("project-atlas-extract.sh"),
    ),
    ("compile.sh", include_str!("compile.sh")),
    ("prepare.sh", include_str!("prepare.sh")),
    ("extract.sh", include_str!("extract.sh")),
    ("imports.sh", include_str!("imports.sh")),
    ("lake.sh", include_str!("lake.sh")),
];
pub(super) fn modules() -> Vec<String> {
    SOURCES.iter().map(|(name, _)| (*name).into()).collect()
}
pub(super) fn identity() -> String {
    let mut hash = Sha256::new();
    for (name, source) in SOURCES.iter().chain(SCRIPTS) {
        hash.update(name);
        hash.update(source);
    }
    format!("{:x}", hash.finalize())
}
/// Metadata validation never executes project code and is not mounted in its
/// environment. Updating it must still revalidate the complete current inputs.
pub(super) fn write_metadata(root: &Path) -> Result<()> {
    fs::write(
        root.join("project-atlas.py"),
        include_str!("project-atlas.py"),
    )?;
    fs::write(
        root.join("project-atlas-prepare.sh"),
        include_str!("project-atlas-prepare.sh"),
    )?;
    Ok(())
}
pub(super) fn write(root: &Path) -> Result<()> {
    for (name, source) in SOURCES {
        let path = root.join(format!("{name}.lean"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, source)?;
    }
    for (name, source) in SCRIPTS {
        fs::write(root.join(name), source)?;
    }
    Ok(())
}
pub(super) fn validate(root: &Path) -> Result<()> {
    // Project metaprograms can read every mounted file, so all compute files
    // participate in identity. Reject unbound extras, links and special files.
    let mut allowed = SOURCES
        .iter()
        .flat_map(|(name, _)| {
            [
                ".lean",
                ".olean",
                ".olean.private",
                ".olean.server",
                ".ir",
                ".ir.sig",
            ]
            .map(|suffix| format!("{name}{suffix}"))
        })
        .chain(SCRIPTS.iter().map(|(name, _)| (*name).into()))
        .chain(std::iter::once("manifest.json".into()))
        .collect::<BTreeSet<_>>();
    let files = allowed.clone();
    for file in &files {
        for parent in Path::new(file).ancestors().skip(1) {
            if !parent.as_os_str().is_empty() {
                allowed.insert(parent.to_string_lossy().into_owned());
            }
        }
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.strip_prefix(root)?.to_string_lossy();
            let kind = entry.file_type()?;
            anyhow::ensure!(
                allowed.contains(name.as_ref())
                    && if files.contains(name.as_ref()) {
                        kind.is_file()
                    } else {
                        kind.is_dir()
                    },
                "Unexpected Atlas resource; prepare again"
            );
            if kind.is_dir() {
                pending.push(path);
            }
        }
    }
    for (name, source) in SOURCES {
        anyhow::ensure!(
            fs::read_to_string(root.join(format!("{name}.lean")))? == *source,
            "Atlas source changed; prepare again"
        );
    }
    for (name, source) in SCRIPTS {
        anyhow::ensure!(
            fs::read_to_string(root.join(name))? == *source,
            "Atlas script changed; prepare again"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{fs, validate, write, write_metadata, Result};

    #[test]
    fn metadata_or_unbound_resources_cannot_enter_the_project_environment() -> Result<()> {
        let compute = tempfile::tempdir()?;
        write(compute.path())?;
        validate(compute.path())?;
        for name in [
            "project-atlas.py",
            "project-atlas-prepare.sh",
            "unbound.olean",
        ] {
            let path = compute.path().join(name);
            fs::write(&path, "unexpected")?;
            assert!(validate(compute.path()).is_err());
            fs::remove_file(path)?;
        }
        let directory = compute.path().join("unbound");
        fs::create_dir(&directory)?;
        assert!(validate(compute.path()).is_err());
        fs::remove_dir(directory)?;
        #[cfg(unix)]
        {
            let path = compute.path().join("MedenAtlas.olean");
            std::os::unix::fs::symlink(compute.path().join("MedenAtlas.lean"), &path)?;
            assert!(validate(compute.path()).is_err());
            fs::remove_file(path)?;
        }
        validate(compute.path())?;
        let metadata = tempfile::tempdir()?;
        write_metadata(metadata.path())?;
        assert_eq!(fs::read_dir(metadata.path())?.count(), 2);
        assert!(!metadata.path().join("MedenAtlas.lean").exists());
        Ok(())
    }
}
