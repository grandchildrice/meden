use super::*;
fn file(path: &str, imports: &[&str], count: usize) -> File<String> {
    let mut file = File::unindexed(path.into(), 0);
    file.imports = imports.iter().map(|s| (*s).into()).collect();
    file.declarations = count;
    file
}
#[test]
fn literal_solver_paths_preserve_order_cycles_and_import_only_expansion() -> Result<()> {
    let inventory = vec![
        file("A.smt2", &["Bridge.smt2", "Leaf.smt2"], 1),
        file("Bridge.smt2", &["Cycle.smt2", "Target.smt2"], 0),
        file("Cycle.smt2", &["A.smt2"], 0),
        file("Hidden.smt2", &[], 1),
        file("Leaf.smt2", &["Hidden.smt2"], 1),
        file("Target.smt2", &[], 1),
    ];
    let files = read(
        &inventory,
        "A.smt2",
        |_, p| Some(p.into()),
        |p| Ok(inventory.iter().find(|f| f.path == p).cloned()),
    )?;
    assert_eq!(
        files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
        vec![
            "A.smt2",
            "Bridge.smt2",
            "Leaf.smt2",
            "Cycle.smt2",
            "Target.smt2"
        ]
    );
    Ok(())
}
#[test]
fn foreign_resolution_or_loaded_context_returns_no_neighborhood() {
    let inventory = vec![file("A.smt2", &["B"], 1)];
    let mut calls = Vec::new();
    assert!(read(
        &inventory,
        "A.smt2",
        |_, _| Some("outside.smt2".into()),
        |p| {
            calls.push(p.to_owned());
            Ok(inventory.first().cloned())
        }
    )
    .is_err());
    assert_eq!(calls, vec!["A.smt2"]);
    assert!(read(
        &inventory,
        "A.smt2",
        |_, _| None,
        |_| Ok(Some(file("outside.smt2", &[], 1)))
    )
    .is_err());
    assert!(read(&inventory, "absent.smt2", |_, _| None, |_| unreachable!()).is_err());
}
#[test]
fn bounded_expansion_and_missing_cached_neighbor_remain_explicit() -> Result<()> {
    let inventory = (0usize..40)
        .map(|i| {
            file(
                &format!("A{i:02}.smt2"),
                &[&format!("A{:02}.smt2", i.saturating_add(1))],
                0,
            )
        })
        .collect::<Vec<_>>();
    let resolve = |_: &str, p: &str| Some(p.into());
    let loaded = read(&inventory, "A00.smt2", resolve, |p| {
        Ok(inventory.iter().find(|f| f.path == p).cloned())
    })?;
    assert_eq!(loaded.len(), 32);
    assert!(read(&inventory, "A00.smt2", resolve, |p| Ok(inventory
        .iter()
        .find(|f| f.path == p && p != "A01.smt2")
        .cloned()))
    .is_err());
    Ok(())
}
