use super::*;
use proptest::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

struct Syntax;
impl Indexer for Syntax {
    type Symbol = String;
    fn supports(&self, path: &str) -> bool {
        matches!(
            std::path::Path::new(path)
                .extension()
                .and_then(|s| s.to_str()),
            Some("smt2" | "v")
        )
    }
    fn index(&self, path: &str, text: &str) -> Result<Indexed<String>> {
        // Opaque producer data: collection may not parse, rewrite or discard it.
        Ok(Indexed {
            declarations: 1,
            theorems: 0,
            imports: vec![format!("import:{path}")],
            symbols: vec![text.into()],
        })
    }
}
#[derive(Default)]
struct Memory {
    directories: BTreeMap<String, Vec<Entry>>,
    contents: BTreeMap<String, Content>,
    reads: Mutex<Vec<Vec<String>>>,
}
impl Reader for Memory {
    fn list(&self, paths: &[String]) -> Result<Vec<Vec<Entry>>> {
        paths
            .iter()
            .map(|p| {
                self.directories
                    .get(p)
                    .cloned()
                    .with_context(|| format!("Missing directory: {p}"))
            })
            .collect()
    }
    fn read(&self, paths: &[String]) -> Result<Vec<Content>> {
        self.reads
            .lock()
            .map_err(|_| anyhow::anyhow!("Reader log unavailable"))?
            .push(paths.to_vec());
        paths
            .iter()
            .map(|p| {
                self.contents
                    .get(p)
                    .cloned()
                    .with_context(|| format!("Missing source: {p}"))
            })
            .collect()
    }
}
fn entry(name: &str, kind: Kind) -> Entry {
    Entry {
        name: name.into(),
        kind,
    }
}
fn text(value: &str) -> Result<Content> {
    Content::from_bytes(value.as_bytes().to_vec())
}

#[test]
fn complete_observations_keep_opaque_data_and_every_unread_size() -> Result<()> {
    let content = text("(assert true)\n")?;
    let digest = content
        .revision()
        .context("Text identity missing")?
        .strip_prefix("sha256:")
        .context("Text identity prefix missing")?
        .to_owned();
    let memory = Memory {
        contents: BTreeMap::from([
            ("b.smt2".into(), content),
            ("empty.v".into(), text("")?),
            ("binary.v".into(), Content::from_bytes(vec![0, 1])?),
            (
                "utf16.v".into(),
                Content::from_bytes(vec![255, 254, 65, 0])?,
            ),
            (
                "large.v".into(),
                Content::SizeLimit {
                    bytes: u64::MAX.to_string(),
                },
            ),
        ]),
        ..Memory::default()
    };
    let paths = memory.contents.keys().cloned().rev().collect::<Vec<_>>();
    let count = AtomicUsize::new(0);
    let actual = sources(&memory, &Syntax, &paths, || {
        count.fetch_add(1, Ordering::SeqCst);
    })?;
    assert_eq!(count.load(Ordering::SeqCst), paths.len());
    let by_path: BTreeMap<_, _> = actual.into_iter().map(|f| (f.path.clone(), f)).collect();
    let file = by_path.get("b.smt2").context("Missing source")?;
    assert_eq!(file.sha256, digest);
    assert_eq!(file.symbols, vec!["(assert true)\n"]);
    assert_eq!(file.imports, vec!["import:b.smt2"]);
    assert_eq!(file.bytes, 14);
    assert!(file.indexed);
    assert!(
        by_path
            .get("empty.v")
            .context("Missing empty source")?
            .indexed
    );
    for (path, bytes) in [
        ("binary.v", 2),
        ("utf16.v", 4),
        ("large.v", usize::try_from(u64::MAX)?),
    ] {
        let file = by_path
            .get(path)
            .context("Missing availability observation")?;
        assert!(!file.indexed && file.sha256.is_empty() && file.symbols.is_empty());
        assert_eq!(file.bytes, bytes);
        // The disk cache retains the existing complete file record shape.
        let encoded = serde_json::to_value(file)?;
        assert_eq!(
            encoded,
            serde_json::json!({
                "path":path,"sha256":"","bytes":bytes,"declarations":0,"theorems":0,
                "imports":[],"symbols":[],"indexed":false
            })
        );
        let decoded: File<String> = serde_json::from_value(encoded)?;
        assert!(decoded == *file);
    }
    Ok(())
}

#[test]
fn fixed_batch_size_and_original_paths_cover_every_source_once() -> Result<()> {
    let memory = Memory {
        contents: (0..97)
            .map(|n| Ok((format!("folder/{n:03}.v"), text(&format!("opaque:{n}"))?)))
            .collect::<Result<_>>()?,
        ..Memory::default()
    };
    let paths = memory.contents.keys().rev().cloned().collect::<Vec<_>>();
    let files = sources(&memory, &Syntax, &paths, || {})?;
    assert_eq!(
        files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
        memory
            .contents
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    let requests = memory
        .reads
        .lock()
        .map_err(|_| anyhow::anyhow!("Reader log unavailable"))?;
    assert_eq!(requests.len(), 4);
    assert!(requests.iter().all(|r| r.len() <= 32));
    assert_eq!(
        requests.iter().flatten().collect::<BTreeSet<_>>(),
        paths.iter().collect()
    );
    assert_eq!(requests.iter().map(Vec::len).sum::<usize>(), 97);
    Ok(())
}

#[test]
fn discovery_preserves_literal_names_and_never_follows_links() -> Result<()> {
    let memory = Memory {
        directories: BTreeMap::from([
            (
                String::new(),
                vec![
                    entry("visible", Kind::Directory),
                    entry("target", Kind::Directory),
                    entry(".private", Kind::Directory),
                    entry("cycle", Kind::Symlink),
                    entry("socket", Kind::Other),
                ],
            ),
            (
                "visible".into(),
                vec![
                    entry("日本語\n😀.smt2", Kind::File),
                    entry("a\\b.v", Kind::File),
                    entry("not-indexed.txt", Kind::File),
                ],
            ),
        ]),
        ..Memory::default()
    };
    assert_eq!(
        paths(&memory, &Syntax)?,
        vec!["visible/a\\b.v", "visible/日本語\n😀.smt2"]
    );
    Ok(())
}

#[test]
fn invalid_components_duplicates_and_all_traversal_limits_refuse_whole_discovery() {
    for name in ["", ".", "..", "a/b", "bad\0"] {
        let memory = Memory {
            directories: BTreeMap::from([(
                String::new(),
                vec![entry("valid.v", Kind::File), entry(name, Kind::File)],
            )]),
            ..Memory::default()
        };
        assert!(paths(&memory, &Syntax).is_err());
    }
    let memory = Memory {
        directories: BTreeMap::from([
            (
                String::new(),
                vec![
                    entry("dir", Kind::Directory),
                    entry("a.v", Kind::File),
                    entry("b.v", Kind::File),
                ],
            ),
            ("dir".into(), vec![entry("nested.v", Kind::File)]),
        ]),
        ..Memory::default()
    };
    for bounds in [
        Bounds {
            directories: 1,
            ..BOUNDS
        },
        Bounds { depth: 0, ..BOUNDS },
        Bounds {
            entries: 3,
            ..BOUNDS
        },
        Bounds { files: 1, ..BOUNDS },
    ] {
        assert!(walk(&memory, &Syntax, bounds).is_err());
    }
    let duplicate = Memory {
        directories: BTreeMap::from([(
            String::new(),
            vec![entry("same.v", Kind::File), entry("same.v", Kind::File)],
        )]),
        ..Memory::default()
    };
    assert!(paths(&duplicate, &Syntax).is_err());
}

struct Mismatch {
    extra: bool,
}
impl Reader for Mismatch {
    fn list(&self, _: &[String]) -> Result<Vec<Vec<Entry>>> {
        Ok(if self.extra {
            vec![vec![], vec![]]
        } else {
            vec![]
        })
    }
    fn read(&self, paths: &[String]) -> Result<Vec<Content>> {
        let count = if self.extra {
            paths.len().saturating_add(1)
        } else {
            paths.len().saturating_sub(1)
        };
        (0..count).map(|_| text("complete")).collect()
    }
}
#[test]
fn wrong_cardinality_never_admits_a_successful_prefix() {
    for extra in [false, true] {
        let reader = Mismatch { extra };
        assert!(paths(&reader, &Syntax).is_err());
        assert!(sources(&reader, &Syntax, &["a.v".into()], || {}).is_err());
    }
}

#[test]
fn source_failure_byte_limits_invalid_text_and_duplicate_requests_refuse_whole_result() -> Result<()>
{
    let memory = Memory {
        contents: BTreeMap::from([("a.v".into(), text("original")?)]),
        ..Memory::default()
    };
    assert!(sources(&memory, &Syntax, &["a.v".into(), "missing.v".into()], || {}).is_err());
    assert!(sources(&memory, &Syntax, &["a.v".into(), "a.v".into()], || {}).is_err());
    for path in ["/absolute.v", "../outside.v", "a//b.v", "bad\0.v"] {
        assert!(sources(&memory, &Syntax, &[path.into()], || {}).is_err());
    }
    assert!(read_all(&memory, &Syntax, &["a.v".into()], &|| {}, 7).is_err());
    assert!(read_all(&memory, &Syntax, &["a.v".into()], &|| {}, 8).is_ok());
    let bad = Content::Text {
        revision: "sha256:invalid".into(),
        bytes: 8,
        text: crate::view::SourceText::new("original".into())?,
    };
    assert!(source(&Syntax, "a.v", bad).is_err());
    Ok(())
}

#[test]
fn no_input_does_not_start_a_reader() -> Result<()> {
    assert!(sources(&Memory::default(), &Syntax, &[], || {})?.is_empty());
    Ok(())
}

proptest! {
    #[test]
    fn generated_forest_matches_flat_original_inventory(
        inputs in prop::collection::vec((0u8..6,0u8..10,any::<bool>()),0..100)
    ) {
        let mut directories=BTreeMap::from([(String::new(),Vec::new())]);
        let mut expected=BTreeSet::new();
        let mut seen=BTreeSet::new();
        for (dir,file,supported) in inputs {
            let directory=match dir {0=>".hidden".to_owned(),1=>"target".to_owned(),_=>format!("folder{dir}")};
            if seen.insert(directory.clone()) {directories.entry(String::new()).or_default().push(entry(&directory,Kind::Directory));}
            let name=format!("{file}.{}",if supported{"v"}else{"txt"});
            let items=directories.entry(directory.clone()).or_default();
            if !items.iter().any(|e:&Entry|e.name==name) {items.push(entry(&name,Kind::File));}
            if supported && !directory.starts_with('.') && directory!="target" {expected.insert(format!("{directory}/{name}"));}
        }
        let memory=Memory{directories,..Memory::default()};
        let actual=paths(&memory,&Syntax).map_err(|e|TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(actual,expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn unsupported_text_stays_unindexed_without_executing_a_parser() -> Result<()> {
    let (file, bytes) = source(&Syntax, "plain.txt", text("opaque contents")?)?;
    assert!(!file.indexed && file.symbols.is_empty() && file.sha256.is_empty());
    assert_eq!(bytes, 15);
    Ok(())
}
#[test]
fn recursive_directory_observations_stop_at_the_original_depth_bound() {
    struct Cycle(AtomicUsize);
    impl Reader for Cycle {
        fn list(&self, paths: &[String]) -> Result<Vec<Vec<Entry>>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(paths
                .iter()
                .map(|_| vec![entry("again", Kind::Directory)])
                .collect())
        }
        fn read(&self, _: &[String]) -> Result<Vec<Content>> {
            anyhow::bail!("No source should be read")
        }
    }
    let reader = Cycle(AtomicUsize::new(0));
    assert!(paths(&reader, &Syntax).is_err());
    assert_eq!(reader.0.load(Ordering::SeqCst), 33);
}
#[test]
fn indexing_failure_drops_an_already_observed_prefix() -> Result<()> {
    struct Failing;
    impl Indexer for Failing {
        type Symbol = String;
        fn supports(&self, _: &str) -> bool {
            true
        }
        fn index(&self, path: &str, text: &str) -> Result<Indexed<String>> {
            anyhow::ensure!(path != "b.v", "Indexer unavailable");
            Syntax.index(path, text)
        }
    }
    let memory = Memory {
        contents: BTreeMap::from([("a.v".into(), text("a")?), ("b.v".into(), text("b")?)]),
        ..Memory::default()
    };
    let completed = AtomicUsize::new(0);
    assert!(
        sources(&memory, &Failing, &["a.v".into(), "b.v".into()], || {
            completed.fetch_add(1, Ordering::SeqCst);
        })
        .is_err()
    );
    assert_eq!(completed.load(Ordering::SeqCst), 1);
    Ok(())
}
