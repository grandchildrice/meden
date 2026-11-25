use super::*;
#[test]
fn saved_metadata_roundtrips_and_method_or_body_changes_refuse() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let store = Store {
        path: directory.path().join("saved.json"),
        method: "original".into(),
    };
    assert!(store.load::<String>()?.is_none());
    let input = Inventory {
        files: vec![File::<String>::unindexed("Original.lean".into(), 7)],
        excluded: vec!["target".into()],
        inventory_only: false,
        generation: 4,
    };
    store.retain(&input)?;
    let saved = store.load::<String>()?.context("Saved inventory missing")?;
    assert_eq!(serde_json::to_value(saved)?, serde_json::to_value(&input)?);
    let changed = Store {
        path: store.path.clone(),
        method: "other".into(),
    };
    assert!(changed.load::<String>().is_err());
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
        method: "original".into(),
    };
    let original = Inventory {
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
        method: "original".into(),
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
