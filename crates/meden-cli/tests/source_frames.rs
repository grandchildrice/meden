#![cfg(target_os = "linux")]
#![allow(clippy::expect_used)]
//! Actual confined source frames: metadata changes batching, never content authority.
use meden_app::{
    adapters::{navigation::Syntax, navigation_io},
    navigation::collection::{self, Reader as _},
};
use meden_runtime::{metrics, process::Control, workspace::Workspace};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

fn worker(root: &Path) -> Workspace {
    Workspace::open(root, Path::new(env!("CARGO_BIN_EXE_meden"))).expect("actual workspace")
}
fn control() -> Control {
    Control::new(Arc::new(AtomicBool::new(false)))
}
fn process_calls(trace: metrics::Context) -> usize {
    let trace = serde_json::to_value(trace).expect("context");
    let metrics = serde_json::to_value(metrics::snapshot()).expect("public metrics");
    assert_eq!(metrics.get("evicted"), Some(&json!(0)));
    metrics
        .get("records")
        .expect("records")
        .as_array()
        .expect("records")
        .iter()
        .filter(|r| {
            r.get("context").and_then(|c| c.get("trace")) == trace.get("trace")
                && r.get("name").and_then(Value::as_str) == Some("process.run")
        })
        .count()
}

#[test]
fn size_hints_reduce_actual_dispatches_without_changing_a_single_source_field() {
    let root = tempfile::tempdir().expect("root");
    let paths = (0..128)
        .map(|n| {
            let path = format!("name日本語\"\n{n:03}.lean");
            std::fs::write(
                root.path().join(&path),
                format!("def item{n} : Nat := {n}\n"),
            )
            .expect("source");
            path
        })
        .rev()
        .collect::<Vec<_>>();
    let workspace = worker(root.path());
    let control = control();
    let plain = navigation_io::Reader::new(&workspace, &control, Instant::now());
    let hints = plain.stamps(&paths).expect("actual metadata");
    let hinted = navigation_io::Reader::new(&workspace, &control, Instant::now())
        .with_hints(&paths, &hints.iter().collect::<Vec<_>>())
        .expect("bound hints");
    let original_scope = metrics::Span::start("test.source_frames.original");
    let original = collection::sources(&plain, &Syntax, &paths, || {}).expect("ordinary sources");
    let original_calls = process_calls(original_scope.context());
    drop(original_scope);
    let hinted_scope = metrics::Span::start("test.source_frames.hinted");
    let actual = collection::sources(&hinted, &Syntax, &paths, || {}).expect("hinted sources");
    let hinted_calls = process_calls(hinted_scope.context());
    assert_eq!(original_calls, 4);
    assert_eq!(hinted_calls, 2);
    assert!(actual == original);
    assert_eq!(actual.len(), paths.len());
    for file in actual {
        assert!(file.indexed && file.declarations == 1);
        assert_eq!(
            file.bytes,
            std::fs::read(root.path().join(file.path))
                .expect("source bytes")
                .len()
        );
    }
}

#[test]
fn escaped_frames_missing_hints_and_false_hints_preserve_complete_results_or_refuse() {
    let root = tempfile::tempdir().expect("root");
    let text = "\u{1}".repeat(200_000);
    let paths = (0..64)
        .map(|n| {
            let path = format!("source{n:03}.lean");
            std::fs::write(root.path().join(&path), &text).expect("escaped source");
            path
        })
        .collect::<Vec<_>>();
    let workspace = worker(root.path());
    let control = control();
    let plain = navigation_io::Reader::new(&workspace, &control, Instant::now());
    let hints = plain.stamps(&paths).expect("actual hints");
    let original = paths
        .chunks(32)
        .flat_map(|chunk| plain.read(chunk).expect("original bounded frame"))
        .map(|c| serde_json::to_value(c).expect("content"))
        .collect::<Vec<_>>();
    let hinted = navigation_io::Reader::new(&workspace, &control, Instant::now())
        .with_hints(&paths, &hints.iter().collect::<Vec<_>>())
        .expect("hints");
    let actual = hinted.read(&paths).expect("complete escaped frames");
    assert_eq!(
        serde_json::to_value(&actual).expect("content"),
        Value::Array(original)
    );
    assert_eq!(actual.len(), paths.len());
    let false_hints = vec![json!({"bytes":0}); paths.len()];
    let false_reader = navigation_io::Reader::new(&workspace, &control, Instant::now())
        .with_hints(&paths, &false_hints.iter().collect::<Vec<_>>())
        .expect("false hints");
    assert!(
        false_reader.read(&paths).is_err(),
        "host output limit must reject the whole frame"
    );
    let unknown = vec![json!({"bytes":"not a byte count"}); paths.len()];
    let unknown_reader = navigation_io::Reader::new(&workspace, &control, Instant::now())
        .with_hints(&paths, &unknown.iter().collect::<Vec<_>>())
        .expect("unknown hints");
    assert_eq!(
        serde_json::to_value(
            unknown_reader
                .read(&paths)
                .expect("conservative unknown hints")
        )
        .expect("content"),
        serde_json::to_value(actual).expect("content")
    );
    let mut invalid = paths.clone();
    *invalid.last_mut().expect("last source") = "../outside.lean".into();
    let invalid_hints = hints.iter().collect::<Vec<_>>();
    let invalid_reader = navigation_io::Reader::new(&workspace, &control, Instant::now())
        .with_hints(&invalid, &invalid_hints)
        .expect("untrusted logical paths");
    let scope = metrics::Span::start("test.source_frames.invalid");
    assert!(invalid_reader.read(&invalid).is_err());
    assert_eq!(process_calls(scope.context()), 0);
    drop(scope);
    let cancelled = Control::new(Arc::new(AtomicBool::new(true)));
    let cancelled_reader = navigation_io::Reader::new(&workspace, &cancelled, Instant::now())
        .with_hints(&paths, &hints.iter().collect::<Vec<_>>())
        .expect("hints");
    assert!(cancelled_reader.read(&paths).is_err());
}
