use super::*;
use std::sync::{atomic::AtomicU8, Mutex};
struct Source {
    fault: AtomicU8,
    active: AtomicBool,
    stamps: AtomicUsize,
    reads: Mutex<Vec<Vec<String>>>,
}
impl Default for Source {
    fn default() -> Self {
        Self {
            fault: AtomicU8::new(0),
            active: AtomicBool::new(true),
            stamps: AtomicUsize::new(0),
            reads: Mutex::new(Vec::new()),
        }
    }
}
impl Reader for Source {
    type Symbol = String;
    type Stamp = u64;
    fn stamps(&self, paths: &[String]) -> Result<Vec<u64>> {
        self.stamps.fetch_add(paths.len(), Ordering::SeqCst);
        match self.fault.load(Ordering::SeqCst) {
            1 => Ok(vec![]),
            2 => Ok(vec![7; paths.len().saturating_add(1)]),
            _ => Ok(vec![7; paths.len()]),
        }
    }
    fn sources(&self, paths: &[String]) -> Result<Vec<File<String>>> {
        self.reads
            .lock()
            .map_err(|_| anyhow::anyhow!("test reader unavailable"))?
            .push(paths.to_vec());
        let files = paths
            .iter()
            .map(|p| File::<String>::unindexed(p.clone(), 4))
            .collect::<Vec<_>>();
        match self.fault.load(Ordering::SeqCst) {
            3 => Ok(vec![]),
            4 => Ok(vec![File::unindexed("foreign.smt2".into(), 4); paths.len()]),
            5 => anyhow::bail!("Source unavailable"),
            7 => anyhow::bail!(""),
            6 => {
                self.active.store(false, Ordering::SeqCst);
                anyhow::bail!("Stopped source");
            }
            _ => Ok(files),
        }
    }
    fn active(&self) -> Result<()> {
        anyhow::ensure!(self.active.load(Ordering::SeqCst), "Stopped");
        Ok(())
    }
}
fn store(root: &Path) -> Store<String, u64> {
    Store {
        directory: root.into(),
        additional: |_| None,
        types: PhantomData,
    }
}
#[test]
fn original_record_bytes_resume_then_refresh_without_reindexing_unchanged_inputs() -> Result<()> {
    let root = tempfile::tempdir()?;
    let store = store(root.path());
    let source = Source::default();
    let sourcepath = "literal日本語\n.smt2".to_owned();
    let paths = vec![sourcepath.clone()];
    // Independent old record representation, not a serialization of Entry.
    let path_hash = format!("{:x}", Sha256::digest(sourcepath.as_bytes()));
    let path = root
        .path()
        .join(&path_hash[..2])
        .join(format!("{path_hash}.json"));
    std::fs::create_dir_all(path.parent().context("test cache parent")?)?;
    let old = serde_json::json!({"path":sourcepath,"stamp":7,"file":{"path":sourcepath,"sha256":"","bytes":4,"declarations":0,"theorems":0,"imports":[],"symbols":[],"indexed":false},"error":null});
    let original = serde_json::to_vec(&old)?;
    std::fs::write(&path, &original)?;
    let inventory = format!("{:x}", Sha256::digest(serde_json::to_vec(&paths)?));
    crate::storage::write(
        &root.path().join("complete.json"),
        &serde_json::json!({"inventory":inventory,"generation":12}),
    )?;
    store.prepare(&source, &paths, false, Scope::Inventory, |_| {})?;
    assert_eq!(source.stamps.load(Ordering::SeqCst), 0);
    assert_eq!(
        store.load(&sourcepath)?.context("test cache record")?.path,
        sourcepath
    );
    store.prepare(&source, &paths, true, Scope::Inventory, |_| {})?;
    assert_eq!(source.stamps.load(Ordering::SeqCst), 1);
    assert!(source
        .reads
        .lock()
        .map_err(|_| anyhow::anyhow!("test reader unavailable"))?
        .is_empty());
    assert_eq!(store.generation()?, 12);
    assert_eq!(std::fs::read(path)?, original);
    Ok(())
}
#[test]
fn corrupt_empty_foreign_and_failed_entries_recover_without_a_stamp_change() -> Result<()> {
    let root = tempfile::tempdir()?;
    let store = store(root.path());
    let source = Source::default();
    let sourcepath = "Root.smt2".to_owned();
    let paths = vec![sourcepath.clone()];
    store.prepare(&source, &paths, false, Scope::Inventory, |_| {})?;
    let file = root.path().join(store.key(&sourcepath));
    for fault in [
        b"broken json".to_vec(),
        serde_json::to_vec(
            &serde_json::json!({"path":sourcepath,"stamp":7,"file":null,"error":null}),
        )?,
        serde_json::to_vec(
            &serde_json::json!({"path":"foreign.smt2","stamp":7,"file":null,"error":"wrong context"}),
        )?,
    ] {
        std::fs::write(&file, fault)?;
        assert!(store.load(&sourcepath).is_err());
        let previous = store.generation()?;
        store.prepare(&source, &paths, true, Scope::Inventory, |_| {})?;
        assert!(store.load(&sourcepath)?.is_some());
        assert_eq!(store.generation()?, previous + 1);
    }
    std::fs::remove_file(&file)?;
    source.fault.store(5, Ordering::SeqCst);
    store.prepare(&source, &paths, true, Scope::Inventory, |_| {})?;
    assert!(store
        .load(&sourcepath)
        .err()
        .context("expected cached failure")?
        .to_string()
        .contains("Source unavailable"));
    source.fault.store(0, Ordering::SeqCst);
    store.prepare(&source, &paths, true, Scope::Inventory, |_| {})?;
    assert!(store.load(&sourcepath)?.is_some());
    Ok(())
}
#[test]
fn incomplete_or_foreign_provider_outputs_and_stops_cannot_seal_an_inventory() -> Result<()> {
    for fault in [1, 2, 3, 4, 6, 7] {
        let root = tempfile::tempdir()?;
        let store = store(root.path());
        let source = Source::default();
        source.fault.store(fault, Ordering::SeqCst);
        assert!(store
            .prepare(
                &source,
                &["Root.smt2".into()],
                true,
                Scope::Inventory,
                |_| {}
            )
            .is_err());
        assert!(!root.path().join("complete.json").exists());
    }
    Ok(())
}
#[test]
fn bounded_batches_and_selection_preparation_do_not_publish_a_complete_inventory() -> Result<()> {
    let root = tempfile::tempdir()?;
    let store = store(root.path());
    let source = Source::default();
    let paths = (0..1030)
        .map(|i| format!("Group{i}/Root.smt2"))
        .collect::<Vec<_>>();
    store.prepare(&source, &paths, true, Scope::Selection, |_| {})?;
    assert!(!root.path().join("complete.json").exists());
    assert_eq!(source.stamps.load(Ordering::SeqCst), 1030);
    let reads = source
        .reads
        .lock()
        .map_err(|_| anyhow::anyhow!("test reader unavailable"))?;
    assert!(reads.iter().all(|r| r.len() <= 256));
    let all = reads.iter().flatten().collect::<BTreeSet<_>>();
    assert_eq!(all, paths.iter().collect::<BTreeSet<_>>());
    for p in &paths {
        assert!(store.load(p)?.is_some());
    }
    Ok(())
}
#[test]
fn additional_provider_changes_reindex_only_the_original_affected_format() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut store = store(root.path());
    store.additional = |p| {
        Path::new(p)
            .extension()
            .is_some_and(|e| e == "json")
            .then(|| "finite-one".into())
    };
    let source = Source::default();
    let paths = vec!["Demo.lean".into(), "counter.contract.json".into()];
    store.prepare(&source, &paths, false, Scope::Inventory, |_| {})?;
    let legacy_inventory = serde_json::json!([paths, [["counter.contract.json", "finite-one"]]]);
    let seal: serde_json::Value =
        crate::storage::read(&root.path().join("complete.json"))?.context("complete seal")?;
    assert_eq!(
        seal.get("inventory"),
        Some(&serde_json::json!(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&legacy_inventory)?)
        )))
    );
    let old_key = format!("{:x}", Sha256::digest(b"finite-one\0counter.contract.json"));
    let original = root
        .path()
        .join(old_key.get(..2).context("key prefix")?)
        .join(format!("{old_key}.json"));
    let bytes = std::fs::read(&original)?;
    store.additional = |p| {
        Path::new(p)
            .extension()
            .is_some_and(|e| e == "json")
            .then(|| "finite-two".into())
    };
    store.prepare(&source, &paths, false, Scope::Inventory, |_| {})?;
    let reads = source
        .reads
        .lock()
        .map_err(|_| anyhow::anyhow!("test reader unavailable"))?;
    assert_eq!(
        reads.last(),
        Some(&vec!["counter.contract.json".to_owned()])
    );
    assert_eq!(std::fs::read(original)?, bytes);
    assert_eq!(store.generation()?, 2);
    Ok(())
}
