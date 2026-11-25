use super::*;
use proptest::prelude::*;
#[test]
fn only_actual_shallow_entry_modules_opt_into_preparation() {
    for p in ["Main.lean", "src/Main.lean"] {
        assert!(entry(p));
    }
    for p in [
        "NotMain.lean",
        "src/NotMain.lean",
        "deep/src/Main.lean",
        "Main.rs",
        "Main.smt2",
    ] {
        assert!(!entry(p));
    }
}
// Retired GUI resolver, used only as a differential oracle for Lean inventories.
fn previous(files: &[File<()>], module: &str) -> Option<String> {
    let mut root = files
        .first()
        .and_then(|f| f.path.rsplit_once('/'))
        .map_or("", |(dir, _)| dir)
        .to_owned();
    while !root.is_empty()
        && files
            .iter()
            .any(|f| !f.path.starts_with(&format!("{root}/")))
    {
        root = root.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
    }
    let relative = format!("{}.lean", module.replace('.', "/"));
    for candidate in [format!("{root}/{relative}"), relative.clone()] {
        if files.binary_search_by(|f| f.path.cmp(&candidate)).is_ok() {
            return Some(candidate);
        }
    }
    let suffix = format!("/{relative}");
    let matches = files
        .iter()
        .filter(|f| f.path.ends_with(&suffix))
        .collect::<Vec<_>>();
    (matches.len() == 1)
        .then(|| matches.first().map(|f| f.path.clone()))
        .flatten()
}
proptest! {
    #[test]
    fn original_lean_priorities_and_ambiguity_survive_relocation(paths in proptest::collection::btree_set("(?:src/|vendor/x/|vendor/y/)?[ABC]/[a-z]{1,4}\\.lean",1..150),module in "[ABC]\\.[a-z]{1,4}") {
        let files=paths.into_iter().map(|p|File::<()>::unindexed(p,0)).collect::<Vec<_>>();let resolve=resolver(&files);
        for path in &files {let name=path.path.trim_start_matches("src/").trim_end_matches(".lean").replace('/',".");prop_assert_eq!(resolve(&path.path,&name),previous(&files,&name));}
        prop_assert_eq!(resolve("Root.lean",&module),previous(&files,&module));
        prop_assert_eq!(resolve("Root.smt2",&module),None);
    }
}
