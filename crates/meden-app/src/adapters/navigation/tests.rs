use super::*;
use crate::navigation::Annotation;
use proptest::prelude::*;
use serde_json::json;
fn symbol(name: &str, line: usize) -> Symbol {
    Symbol {
        name: name.into(),
        line,
        kind: "theorem".into(),
        declaration: "theorem".into(),
        private: false,
        references: vec!["original.reference".into()],
        semantic: None,
        main_theorem: false,
    }
}
fn note(file: &str, name: &str, line: usize, main_theorem: bool) -> anyhow::Result<Annotation> {
    Ok(Annotation { file:file.into(),name:name.into(),line,main_theorem,
        semantic:serde_json::from_value(json!({"version":1,"origin":"elaborated","sentence":format!("{file}:{name}:{line}:{main_theorem}"),"refutation":"candidate", "checker":{"engine":"fixture","status":"not_found","candidates":[]}})).map_err(anyhow::Error::from)? })
}
proptest! {
    #[test]
    fn indexed_overlay_matches_original_ordered_scan(
        entries in prop::collection::vec((0u8..12,0u8..12,1usize..20),0..120),
        notes in prop::collection::vec((0u8..14,0u8..14,1usize..22,any::<bool>()),0..120)
    ) {
        use std::collections::BTreeMap;
        let mut actual: BTreeMap<String,Vec<Symbol>> = BTreeMap::new();
        for (f,n,line) in entries { actual.entry(format!("{f}.lean")).or_default().push(symbol(&format!("claim{n}"),line)); }
        let mut expected = actual.clone();
        let notes = notes.into_iter().map(|(f,n,l,m)|note(&format!("{f}.lean"),&format!("claim{n}"),l,m)).collect::<anyhow::Result<Vec<_>>>().map_err(|e| TestCaseError::fail(e.to_string()))?;
        for note in &notes {
            if let Some(symbol) = expected.get_mut(&note.file).and_then(|symbols|symbols.iter_mut().find(|s|s.name==note.name && s.line==note.line)) {
                symbol.semantic=Some(note.semantic.clone()); symbol.main_theorem=note.main_theorem;
            }
        }
        overlay(actual.iter_mut().map(|(p,s)|(p.as_str(),s.as_mut_slice())),&notes);
        prop_assert_eq!(serde_json::to_value(actual).map_err(|e|TestCaseError::fail(e.to_string()))?,serde_json::to_value(expected).map_err(|e|TestCaseError::fail(e.to_string()))?);
    }
}
#[test]
fn indexed_overlay_does_not_attach_another_file_name_or_line() -> anyhow::Result<()> {
    let mut original = vec![symbol("claim", 1), symbol("claim", 2), symbol("other", 1)];
    let notes = vec![
        note("other.lean", "claim", 1, true)?,
        note("this.lean", "claim", 2, true)?,
        note("this.lean", "claim", 99, true)?,
    ];
    overlay([("this.lean", original.as_mut_slice())], &notes);
    let [first, selected, last] = original.as_slice() else {
        anyhow::bail!("Original declaration count changed");
    };
    assert!(first.semantic.is_none());
    assert!(selected.semantic.is_some());
    assert!(last.semantic.is_none());
    assert_eq!(selected.references, vec!["original.reference"]);
    Ok(())
}
