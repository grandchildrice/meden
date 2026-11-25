#![cfg(target_os = "linux")]
//! Real confined metadata checks against an independent host-filesystem oracle.
use anyhow::{Context, Result};
use meden_app::adapters::lean::collect::check_metadata;
use meden_app::workspace::observation::{self, Entry, Metadata};
use meden_runtime::{metrics, process::Control, workspace::Workspace};
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

fn control() -> Control {
    Control::new(Arc::new(AtomicBool::new(false)))
}
fn entries(root: &Path, path: &str) -> Result<Option<Vec<Entry>>> {
    let path = root.join(path);
    if !path.exists() {
        return Ok(None);
    }
    let mut entries = Vec::new();
    for item in fs::read_dir(path)? {
        let item = item?;
        let kind = item.file_type()?;
        let name = item
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("Non UTF-8 fixture"))?;
        let kind = if kind.is_file() {
            "file"
        } else if kind.is_dir() {
            "directory"
        } else if kind.is_symlink() {
            "symlink"
        } else {
            "other"
        };
        if kind != "file"
            || Path::new(&name)
                .extension()
                .is_some_and(|ext| ext == "lean")
        {
            entries.push(Entry {
                name,
                kind: kind.into(),
            });
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Some(entries))
}
fn open(root: &Path) -> Result<Workspace> {
    Ok(Workspace::open(
        root,
        Path::new(env!("CARGO_BIN_EXE_meden")),
    )?)
}
#[test]
fn a_solver_inventory_uses_its_selected_format_without_lean_resources() -> Result<()> {
    let root = tempfile::tempdir()?;
    fs::create_dir(root.path().join("nested"))?;
    for (path, text) in [
        ("a.smt2", "(check-sat)"),
        ("b.lean", "not a solver input"),
        ("solver.cfg", "original"),
    ] {
        fs::write(root.path().join(path), text)?;
    }
    let expected = Metadata {
        files: [("solver.cfg".into(), Some("original".into()))].into(),
        directories: [(
            String::new(),
            Some(vec![
                Entry {
                    name: "a.smt2".into(),
                    kind: "file".into(),
                },
                Entry {
                    name: "nested".into(),
                    kind: "directory".into(),
                },
            ]),
        )]
        .into(),
    };
    let select = |entry: &Entry| {
        entry.kind != "file"
            || Path::new(&entry.name)
                .extension()
                .is_some_and(|ext| ext == "smt2")
    };
    let workspace = open(root.path())?;
    observation::check_metadata(&workspace, &control(), &expected, select)?;
    fs::write(root.path().join("extra.lean"), "unrelated")?;
    observation::check_metadata(&workspace, &control(), &expected, select)?;
    fs::write(root.path().join("new.smt2"), "(check-sat)")?;
    assert!(observation::check_metadata(&workspace, &control(), &expected, select).is_err());
    fs::remove_file(root.path().join("new.smt2"))?;
    fs::write(root.path().join("solver.cfg"), "modified")?;
    assert!(observation::check_metadata(&workspace, &control(), &expected, select).is_err());
    fs::write(root.path().join("solver.cfg"), "original")?;
    let stopped = control();
    stopped.cancel();
    assert!(observation::check_metadata(&workspace, &stopped, &expected, select).is_err());
    Ok(())
}
#[test]
fn complete_batches_preserve_contents_absence_inventory_and_literal_names() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut expected = Metadata::default();
    for group in 0..12 {
        let folder = format!("packs/g{group}/src");
        fs::create_dir_all(root.path().join(&folder))?;
        for index in 0..25 {
            let path = format!("{folder}/日本語\n{index}.lean");
            let text = format!("original {group}:{index} λ𝕏\n");
            fs::write(root.path().join(&path), &text)?;
            expected.files.insert(path, Some(text));
        }
        fs::write(
            root.path().join(&folder).join("ignored.txt"),
            "not a Lean module",
        )?;
        expected
            .directories
            .insert(folder.clone(), entries(root.path(), &folder)?);
    }
    fs::write(root.path().join("real.cfg"), "original")?;
    std::os::unix::fs::symlink("real.cfg", root.path().join("alias.cfg"))?;
    expected
        .files
        .insert("alias.cfg".into(), Some("original".into()));
    expected
        .files
        .insert("absent/deeper/lakefile.lean".into(), None);
    expected
        .files
        .insert("packs/g0/src/absent.toml".into(), None);
    expected.directories.insert("absent/deeper".into(), None);
    let workspace = open(root.path())?;
    let timing = metrics::Span::start("test.metadata.original");
    let context = serde_json::to_value(timing.context())?;
    check_metadata(&workspace, &control(), &expected)?;
    drop(timing);
    let snapshot = serde_json::to_value(metrics::snapshot())?;
    let calls = snapshot
        .get("records")
        .and_then(serde_json::Value::as_array)
        .context("records")?
        .iter()
        .filter(|row| {
            row.pointer("/context/trace") == context.get("trace")
                && row.get("name").and_then(serde_json::Value::as_str) == Some("process.run")
        })
        .count();
    assert!(
        calls > 1 && calls < 20,
        "Hundreds of original inputs should not require one process per read: {calls}"
    );
    let path = root.path().join("packs/g0/src/日本語\n0.lean");
    let original = fs::read(&path)?;
    let mut tampered = original.clone();
    *tampered.first_mut().context("original byte")? = b'x';
    fs::write(&path, tampered)?;
    assert!(check_metadata(&workspace, &control(), &expected).is_err());
    fs::write(&path, &original)?;
    fs::write(root.path().join("packs/g0/src/new.lean"), "new module")?;
    assert!(check_metadata(&workspace, &control(), &expected).is_err());
    fs::remove_file(root.path().join("packs/g0/src/new.lean"))?;
    fs::write(root.path().join("packs/g0/src/unrelated.txt"), "ignored")?;
    check_metadata(&workspace, &control(), &expected)?;
    fs::create_dir_all(root.path().join("absent/deeper"))?;
    fs::write(
        root.path().join("absent/deeper/lakefile.lean"),
        "was absent",
    )?;
    assert!(check_metadata(&workspace, &control(), &expected).is_err());
    Ok(())
}
#[test]
fn failed_or_cancelled_batches_never_accept_a_prefix_or_an_outside_link() -> Result<()> {
    let root = tempfile::tempdir()?;
    fs::create_dir(root.path().join("nested"))?;
    fs::write(root.path().join("nested/config"), "original")?;
    fs::write(root.path().join("a-good"), "good")?;
    let mut expected = Metadata::default();
    expected.files.insert("a-good".into(), Some("good".into()));
    expected
        .files
        .insert("nested/config".into(), Some("original".into()));
    let workspace = open(root.path())?;
    check_metadata(&workspace, &control(), &expected)?;
    for bytes in [vec![0xff, 0xfe], vec![b'x'; 65_537]] {
        fs::write(root.path().join("nested/config"), bytes)?;
        assert!(check_metadata(&workspace, &control(), &expected).is_err());
    }
    fs::remove_file(root.path().join("nested/config"))?;
    assert!(check_metadata(&workspace, &control(), &expected).is_err());
    let outside = tempfile::tempdir()?;
    fs::write(outside.path().join("secret"), "OUTSIDE-CANARY")?;
    std::os::unix::fs::symlink(
        outside.path().join("secret"),
        root.path().join("nested/config"),
    )?;
    let failure = check_metadata(&workspace, &control(), &expected)
        .err()
        .context("outside refused")?;
    assert!(!failure.to_string().contains("OUTSIDE-CANARY"));
    fs::remove_file(root.path().join("nested/config"))?;
    fs::write(root.path().join("nested/config"), "original")?;
    let stopped = control();
    stopped.cancel();
    assert!(check_metadata(&workspace, &stopped, &expected).is_err());
    expected.files.insert("../outside".into(), None);
    assert!(check_metadata(&workspace, &control(), &expected).is_err());
    Ok(())
}
#[test]
fn a_large_filtered_listing_still_obeys_the_original_response_limit() -> Result<()> {
    let root = tempfile::tempdir()?;
    fs::create_dir(root.path().join("huge"))?;
    for index in 0..30_000 {
        let name = format!("{index:05}{}.txt", "x".repeat(246));
        fs::write(root.path().join("huge").join(name), "")?;
    }
    let mut expected = Metadata::default();
    // Filtering would remove every file, but must not admit an oversized raw listing.
    expected.directories.insert("huge".into(), Some(Vec::new()));
    let workspace = open(root.path())?;
    let error = check_metadata(&workspace, &control(), &expected)
        .err()
        .context("limit")?;
    assert!(error.to_string().contains("8 MiB"));
    Ok(())
}
