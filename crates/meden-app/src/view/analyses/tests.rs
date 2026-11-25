#![allow(clippy::expect_used)]
use super::*;
use anyhow::Context as _;
use serde_json::{json, Value};

fn record(sequence: u64) -> analysis::Record {
    analysis::Record {
        version: 2,
        sequence,
        id: format!("analysis-{sequence:064x}"),
        requirement_id: format!("requirement-{}", "ab".repeat(32)),
        finding_id: None,
        source_artifact: Some(format!("sha256:{}", "cd".repeat(32))),
        engine: "candidate.observation.v1".into(),
        model: "original-model".into(),
        processor: "original-destination".into(),
        judge: None,
        model_definition: Some(json!({"original": [null, "λ\n\"", 41]})),
        judge_definition: None,
        input: semantic::Input {
            source_revision: crate::identity::snapshots::SourceRevision::parse(&format!(
                "sha256:{}",
                "ef".repeat(32)
            ))
            .expect("canonical fixture"),
            graph_revision: None,
            target: semantic::Target {
                file: "source 日本語.rs".into(),
                line: 2,
                name: "declaration_λ".into(),
            },
            requirement: "Original requirement λ\n<&>".into(),
        },
        state: analysis::State::Complete,
        assessment: None,
        partial_translation: None,
        error: None,
    }
}
fn body(analyses: &Analyses, history: bool) -> Result<(String, Arc<[u8]>, Value)> {
    let status = serde_json::to_value(analyses)?;
    let revision = status
        .get(if history { "history" } else { "current" })
        .and_then(|s| s.get("revision"))
        .context("reference")?
        .as_str()
        .context("ready revision")?
        .to_owned();
    let bytes = if history {
        analyses.history_bytes(&ArtifactId::parse(&revision)?)
    } else {
        analyses.current_bytes(&ArtifactId::parse(&revision)?)
    }
    .context("complete body")?;
    let parsed = serde_json::from_slice(&bytes)?;
    Ok((revision, bytes, parsed))
}
#[test]
fn preserves_every_original_field_and_lossless_sequence_while_polling_only_references() -> Result<()>
{
    let original = record(u64::MAX);
    let mut analyses = Analyses::new(Records {
        current: Some(original.clone()),
        history: vec![original.clone()],
    });
    let (revision, bytes, value) = body(&analyses, false)?;
    let (history_revision, history_bytes, history) = body(&analyses, true)?;
    let mut expected = serde_json::to_value(&original)?;
    expected
        .as_object_mut()
        .context("record")?
        .insert("sequence".into(), Value::String(u64::MAX.to_string()));
    assert_eq!(value.get("data"), Some(&expected));
    assert_eq!(history.pointer("/data/history"), Some(&json!([expected])));
    let status = serde_json::to_string(&analyses)?;
    assert!(status.len() < 350);
    assert!(!status.contains("original-model"));
    assert!(!status.contains("requirement"));
    for _ in 0..100 {
        assert_eq!(serde_json::to_string(&analyses)?, status);
        assert!(Arc::ptr_eq(
            &bytes,
            &analyses
                .current_bytes(&ArtifactId::parse(&revision)?)
                .context("retained")?
        ));
    }
    analyses.select(Some(original));
    assert!(Arc::ptr_eq(
        &bytes,
        &analyses
            .current_bytes(&ArtifactId::parse(&revision)?)
            .context("identical")?
    ));
    analyses.invalidate();
    assert!(analyses
        .current_bytes(&ArtifactId::parse(&revision)?)
        .is_none());
    let (stale_revision, _, stale) = body(&analyses, false)?;
    assert_ne!(stale_revision, revision);
    assert_eq!(stale.pointer("/data/state"), Some(&json!("stale")));
    assert_eq!(stale.pointer("/data/input"), value.pointer("/data/input"));
    assert!(Arc::ptr_eq(
        &history_bytes,
        &analyses
            .history_bytes(&ArtifactId::parse(&history_revision)?)
            .context("unchanged history")?
    ));
    analyses.select(Some(record(12)));
    assert!(analyses
        .current_bytes(&ArtifactId::parse(&stale_revision)?)
        .is_none());
    assert_eq!(
        body(&analyses, false)?.2.pointer("/data/sequence"),
        Some(&json!("12"))
    );
    assert!(Arc::ptr_eq(
        &history_bytes,
        &analyses
            .history_bytes(&ArtifactId::parse(&history_revision)?)
            .context("selection retains history")?
    ));
    assert!(analyses
        .history_bytes(&ArtifactId::parse(&body(&analyses, false)?.0)?)
        .is_none());
    assert!(analyses
        .current_bytes(&ArtifactId::parse(&history_revision)?)
        .is_none());
    Ok(())
}
#[test]
fn large_complete_history_has_constant_poll_size_and_never_admits_a_prefix() -> Result<()> {
    let history = (1..=400)
        .map(|sequence| {
            let mut row = record(sequence);
            row.error = Some("Original retained diagnostic 𝕏\n".repeat(100));
            row
        })
        .collect::<Vec<_>>();
    let mut analyses = Analyses::new(Records {
        current: history.last().cloned(),
        history,
    });
    let (revision, bytes, value) = body(&analyses, true)?;
    assert!(bytes.len() > 1_000_000);
    assert_eq!(
        value
            .pointer("/data/history")
            .and_then(Value::as_array)
            .context("history")?
            .len(),
        400
    );
    assert_eq!(
        value.pointer("/data/history/399/sequence"),
        Some(&json!("400"))
    );
    assert!(serde_json::to_vec(&analyses)?.len() < 350);
    let mut too_many = analyses.data().history.clone();
    too_many.resize(1001, record(1001));
    analyses.replace_history(too_many);
    assert!(matches!(
        analyses.status.history,
        HistoryStatus::Unavailable { .. }
    ));
    assert!(analyses
        .history_bytes(&ArtifactId::parse(&revision)?)
        .is_none());
    assert_eq!(analyses.data().history.len(), 1001);
    let mut restored = analyses.data().history.clone();
    restored.truncate(400);
    analyses.replace_history(restored);
    assert!(matches!(
        analyses.status.history,
        HistoryStatus::Ready { .. }
    ));
    // Each envelope fits on its own, but together they exceed 64 MiB. Refuse
    // only the changed role, retaining the independently valid current body.
    let mut shared = record(88);
    shared.error = Some("\0".repeat(6_291_456));
    analyses.select(Some(shared.clone()));
    assert!(matches!(analyses.status.current, Current::Ready { .. }));
    let current_status = serde_json::to_value(&analyses)?;
    let current_revision = current_status
        .pointer("/current/revision")
        .and_then(Value::as_str)
        .context("current reference")?
        .to_owned();
    analyses.replace_history(vec![shared]);
    assert!(matches!(
        analyses.status.history,
        HistoryStatus::Unavailable { .. }
    ));
    assert!(analyses
        .current_bytes(&ArtifactId::parse(&current_revision)?)
        .is_some());
    analyses.select(None);
    analyses.replace_history(analyses.data().history.clone());
    assert!(matches!(
        analyses.status.history,
        HistoryStatus::Ready { .. }
    ));
    let before = body(&analyses, true)?.0;
    let mut huge = record(99);
    huge.error = Some("\0".repeat(12_582_912));
    analyses.select(Some(huge.clone()));
    analyses.replace_history(vec![huge]);
    assert!(matches!(
        analyses.status.current,
        Current::Unavailable { .. }
    ));
    assert!(matches!(
        analyses.status.history,
        HistoryStatus::Unavailable { .. }
    ));
    assert!(analyses
        .history_bytes(&ArtifactId::parse(&before)?)
        .is_none());
    assert_eq!(
        analyses
            .data()
            .history
            .first()
            .context("retained record")?
            .error
            .as_ref()
            .context("original")?
            .len(),
        12_582_912
    );
    Ok(())
}
#[test]
fn invalid_or_unreadable_observations_are_explicit_without_restoring_old_results() -> Result<()> {
    let mut analyses = Analyses::new(Records {
        current: Some(record(1)),
        history: vec![],
    });
    let original = body(&analyses, false)?.0;
    let mut invalid = record(1);
    invalid.input.target.line = 0;
    analyses.select(Some(invalid));
    assert!(analyses
        .current_bytes(&ArtifactId::parse(&original)?)
        .is_none());
    assert!(matches!(
        analyses.status.current,
        Current::Unavailable { .. }
    ));
    assert!(matches!(
        analyses.status.history,
        HistoryStatus::Ready { .. }
    ));
    assert_eq!(
        analyses
            .data()
            .current
            .as_ref()
            .context("retained")?
            .input
            .target
            .line,
        0
    );
    analyses.select(Some(record(2)));
    assert_eq!(
        body(&analyses, false)?.2.pointer("/data/sequence"),
        Some(&json!("2"))
    );
    analyses.select(None);
    assert!(matches!(analyses.status.current, Current::Empty));
    let unavailable = Analyses::unavailable(&"𝕏".repeat(8000));
    let status = serde_json::to_value(unavailable)?;
    let message = status
        .pointer("/history/message")
        .context("diagnostic")?
        .as_str()
        .context("diagnostic")?;
    assert!(message.len() <= 8000);
    assert!(message.contains("diagnostic shortened"));
    assert_eq!(
        status.pointer("/history/state"),
        Some(&json!("unavailable"))
    );
    Ok(())
}
