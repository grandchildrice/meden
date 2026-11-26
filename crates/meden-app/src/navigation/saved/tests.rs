use super::*;
#[test]
fn saved_metadata_retains_original_parser_and_composition_without_relabeling() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store {
        path: directory.path().join("saved.json"),
    };
    assert!(store.load::<String>()?.is_none());
    let input = Inventory {
        indexer: "a".repeat(64),
        method: "b".repeat(64),
        files: vec![File::<String>::unindexed("Original.lean".into(), 7)],
        excluded: vec!["target".into()],
        inventory_only: false,
        generation: 4,
    };
    store.retain(&input)?;
    let saved = store.load::<String>()?.context("Saved inventory missing")?;
    assert_eq!(serde_json::to_value(&saved)?, serde_json::to_value(&input)?);
    assert_eq!(saved.indexer, input.indexer);
    assert_eq!(saved.method, input.method);
    let mut body: serde_json::Value = serde_json::from_slice(&fs::read(&store.path)?)?;
    *body
        .get_mut("inventory")
        .and_then(|v| v.get_mut("generation"))
        .context("Saved generation missing")? = 5.into();
    fs::write(&store.path, serde_json::to_vec(&body)?)?;
    assert!(store.load::<String>().is_err());
    Ok(())
}

#[test]
fn failed_replacement_keeps_the_original_complete_record() -> Result<()> {
    struct Failure;
    impl Serialize for Failure {
        fn serialize<S: serde::Serializer>(&self, _: S) -> std::result::Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("Intentional serializer failure"))
        }
    }
    let directory = tempfile::tempdir()?;
    let store = Store {
        path: directory.path().join("saved.json"),
    };
    let original = Inventory {
        indexer: "a".repeat(64),
        method: "b".repeat(64),
        files: vec![File::<String>::unindexed("original.v".into(), 0)],
        excluded: vec![],
        inventory_only: false,
        generation: 0,
    };
    store.retain(&original)?;
    let before = fs::read(&store.path)?;
    let mut file = File::unindexed("invalid.v".into(), 0);
    file.symbols.push(Failure);
    let invalid = Inventory {
        indexer: "a".repeat(64),
        method: "b".repeat(64),
        files: vec![file],
        excluded: vec![],
        inventory_only: false,
        generation: 0,
    };
    assert!(store.retain(&invalid).is_err());
    assert_eq!(fs::read(&store.path)?, before);
    assert_eq!(
        store
            .load::<String>()?
            .context("Original saved data missing")?
            .files
            .len(),
        1
    );
    Ok(())
}
#[test]
fn malformed_data_and_byte_budget_do_not_publish_a_prefix() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store {
        path: directory.path().join("saved.json"),
    };
    fs::write(&store.path, b"{incomplete")?;
    assert!(store.load::<String>().is_err());
    let mut writer = Bounded {
        inner: Vec::<u8>::new(),
        bytes: LIMIT.saturating_sub(1),
    };
    assert!(writer.write_all(&[1, 2]).is_err());
    assert!(writer.inner.is_empty());
    Ok(())
}

#[test]
fn unsupported_formats_and_invalid_origins_refuse_even_with_a_matching_checksum() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store {
        path: directory.path().join("saved.json"),
    };
    let inventory = Inventory {
        indexer: "a".repeat(64),
        method: "b".repeat(64),
        files: vec![File::<String>::unindexed("original.rs".into(), 7)],
        excluded: vec![],
        inventory_only: true,
        generation: 1,
    };
    store.retain(&inventory)?;
    let before = fs::read(&store.path)?;
    let record: Record<Inventory<String>> = serde_json::from_slice(&before)?;
    let unsupported = Record {
        version: 2,
        ..record
    };
    fs::write(&store.path, serde_json::to_vec(&unsupported)?)?;
    assert!(store.load::<String>().is_err());
    for invalid in [
        String::new(),
        "A".repeat(64),
        "x".repeat(64),
        "a".repeat(65),
    ] {
        for method in [false, true] {
            let mut corrupted = inventory.clone();
            if method {
                corrupted.method = invalid.clone();
            } else {
                corrupted.indexer = invalid.clone();
            }
            let record = Record {
                version: 1,
                sha256: digest(&corrupted)?,
                inventory: &corrupted,
            };
            fs::write(&store.path, serde_json::to_vec(&record)?)?;
            assert!(store.load::<String>().is_err());
            fs::write(&store.path, &before)?;
            assert!(store.retain(&corrupted).is_err());
            assert_eq!(fs::read(&store.path)?, before);
        }
    }
    Ok(())
}
