//! Exact host-owned analysis sources, compiled in their dependency order.
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
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
