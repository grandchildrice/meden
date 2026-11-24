use super::*;
use meden_runtime::{process::Control, verification::artifact::Inputs as PreparedInputs};
use proptest::prelude::*;
use serde_json::Value;
use std::sync::atomic::AtomicBool;

fn summary() -> Result<project::Summary> {
    let root = tempfile::tempdir()?;
    let project = project::Project::new(root.path());
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let contract = crate::verification::Registry::host().parse_selected(
        include_str!("../../../../../verification/examples/finite-counter.contract.json"),
        Some("z3.finite.v1"),
    )?;
    let proposed = project.propose(&contract, &control)?;
    project.review(&proposed.reference)?;
    let mut summary = project.accept(&proposed.reference)?;
    // Projection fixture only. This does not issue a job or admit actual evidence.
    summary.verified = true;
    summary.source_sha256 = Some("a".repeat(64));
    summary.artifact_sha256 = Some("b".repeat(64));
    summary.artifact = Some("retained/original".into());
    Ok(summary)
}
fn inputs() -> PreparedInputs {
    PreparedInputs {
        registration_sha256: "c".repeat(64),
        source_sha256: "a".repeat(64),
        bundle_sha256: "d".repeat(64),
        checker_sha256: "e".repeat(64),
        worker_sha256: "f".repeat(64),
    }
}
fn body(reader: &Reader) -> Result<(String, Arc<[u8]>, Value)> {
    let status = serde_json::to_value(reader)?;
    let revision = status
        .get("revision")
        .and_then(Value::as_str)
        .context("Ready reference")?
        .to_owned();
    let bytes = reader.bytes(&revision).context("Exact body")?;
    let data = serde_json::from_slice(&bytes)?;
    Ok((revision, bytes, data))
}
fn at(value: &Value, path: &str) -> Result<Value> {
    value.pointer(path).cloned().context("Original field")
}

#[test]
fn original_context_and_inputs_survive_all_actual_stage_boundaries() -> Result<()> {
    let summary = summary()?;
    let publisher = Publisher::new(b"original host session");
    let reader = publisher.reader();
    assert_eq!(
        serde_json::to_value(&reader)?,
        serde_json::json!({"state":"idle"})
    );
    let observation = publisher
        .begin(
            &summary.reference,
            summary.contract.as_ref().context("contract")?,
        )
        .context("Observation")?;
    let (first, bytes, start) = body(&reader)?;
    assert_eq!(at(&start, "/data/phase/stage/state")?, "reading_source");
    assert_eq!(
        at(&start, "/data/context/reference")?,
        serde_json::to_value(super::super::evidence::Reference::new(&summary.reference)?)?
    );
    assert_eq!(at(&start, "/data/context/class")?, "bounded");
    for _ in 0..100 {
        assert!(Arc::ptr_eq(
            &bytes,
            &reader.bytes(&first).context("Frozen body")?
        ));
    }
    publisher.source(Some(&observation), 1, "a".repeat(64));
    publisher.checking(Some(&observation), 2, &inputs());
    publisher.refreshing(Some(&observation), 3);
    publisher.retaining(Some(&observation), 4);
    publisher.finish(Some(observation), 5, Completion::Admitted(&summary));
    let (_, _, completed) = body(&reader)?;
    assert_eq!(
        at(&completed, "/data/context")?,
        at(&start, "/data/context")?
    );
    assert_eq!(at(&completed, "/data/phase/state")?, "admitted");
    assert_eq!(
        at(&completed, "/data/phase/inputs/source_sha256")?,
        "a".repeat(64)
    );
    assert_eq!(
        at(&completed, "/data/phase/inputs/prepared_sha256")?,
        format!("{:x}", Sha256::digest(serde_json::to_vec(&inputs())?))
    );
    assert_eq!(
        at(&completed, "/data/phase/completion_label")?,
        summary.completion_label
    );
    assert!(reader.bytes(&first).is_none());
    Ok(())
}

#[test]
fn replacement_terminal_and_backwards_observations_cannot_revive_old_bytes() -> Result<()> {
    let summary = summary()?;
    let contract = summary.contract.as_ref().context("contract")?;
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    let old = publisher
        .begin(&summary.reference, contract)
        .context("old")?;
    let (old_id, _, _) = body(&reader)?;
    let next = publisher
        .begin(&summary.reference, contract)
        .context("next")?;
    publisher.source(Some(&next), 100, "a".repeat(64));
    let (current, original, _) = body(&reader)?;
    publisher.checking(Some(&old), 101, &inputs());
    publisher.finish(Some(old), 102, Completion::Cancelled("old cancellation"));
    publisher.checking(Some(&next), 99, &inputs());
    assert!(Arc::ptr_eq(
        &original,
        &reader.bytes(&current).context("same")?
    ));
    assert!(reader.bytes(&old_id).is_none());
    let late = Observation {
        job: next.job.clone(),
    }; // Internal fault injection only.
    publisher.finish(
        Some(next),
        101,
        Completion::Failed(FailureKind::Unavailable, "original resource failure"),
    );
    let (terminal, frozen, failed) = body(&reader)?;
    publisher.checking(Some(&late), u32::MAX, &inputs());
    publisher.finish(Some(late), u32::MAX, Completion::Admitted(&summary));
    assert!(Arc::ptr_eq(
        &frozen,
        &reader.bytes(&terminal).context("terminal")?
    ));
    assert_eq!(at(&failed, "/data/phase/state")?, "failed");
    assert_eq!(
        at(&failed, "/data/phase/during/state")?,
        "preparing_checker"
    );
    assert_eq!(at(&failed, "/data/phase/kind")?, "unavailable");
    assert_eq!(
        at(&failed, "/data/phase/during/source_sha256")?,
        "a".repeat(64)
    );
    Ok(())
}

#[test]
fn diagnostic_words_cannot_select_failure_or_cancellation() -> Result<()> {
    let summary = summary()?;
    let contract = summary.contract.as_ref().context("contract")?;
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    let first = publisher.begin(&summary.reference, contract);
    publisher.source(first.as_ref(), 1, "a".repeat(64));
    publisher.checking(first.as_ref(), 2, &inputs());
    publisher.finish(
        first,
        3,
        Completion::Failed(
            FailureKind::Unverified,
            "Cancelled TimedOut success proof passed",
        ),
    );
    let (_, _, failed) = body(&reader)?;
    assert_eq!(at(&failed, "/data/phase/kind")?, "unverified");
    assert_eq!(at(&failed, "/data/phase/state")?, "failed");
    assert_eq!(
        at(&failed, "/data/phase/during/inputs/source_sha256")?,
        "a".repeat(64)
    );
    let second = publisher.begin(&summary.reference, contract);
    publisher.finish(second, 1, Completion::Cancelled(&"λ\0".repeat(10_000)));
    let (_, _, cancelled) = body(&reader)?;
    assert_eq!(at(&cancelled, "/data/phase/state")?, "cancelled");
    let message = at(&cancelled, "/data/phase/message")?;
    let message = message.as_str().context("message")?;
    assert!(message.len() <= 8000 && message.ends_with("(diagnostic shortened)"));
    Ok(())
}

#[test]
fn invalid_or_premature_observations_retire_the_body_and_require_a_new_job() -> Result<()> {
    let summary = summary()?;
    let contract = summary.contract.as_ref().context("contract")?;
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    let first = publisher.begin(&summary.reference, contract);
    publisher.finish(first, 0, Completion::Admitted(&summary));
    assert!(matches!(
        publisher
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("state"))?
            .status,
        Status::Unavailable { .. }
    ));
    let second = publisher.begin(&summary.reference, contract);
    publisher.source(second.as_ref(), 1, "a".repeat(64));
    let (old, _, _) = body(&reader)?;
    let mut changed = inputs();
    changed.source_sha256 = "b".repeat(64);
    publisher.checking(second.as_ref(), 2, &changed);
    assert!(reader.bytes(&old).is_none());
    publisher.checking(second.as_ref(), 3, &inputs());
    publisher.finish(second, 4, Completion::Cancelled("late"));
    assert!(matches!(
        publisher
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("state"))?
            .status,
        Status::Unavailable { .. }
    ));
    let third = publisher
        .begin(&summary.reference, contract)
        .context("recovery")?;
    publisher.finish(
        Some(third),
        1,
        Completion::Failed(FailureKind::Inconclusive, "Original failure"),
    );
    assert_eq!(at(&body(&reader)?.2, "/data/phase/kind")?, "inconclusive");
    let last = body(&reader)?.0;
    publisher
        .0
        .lock()
        .map_err(|_| anyhow::anyhow!("state"))?
        .sequence = u64::MAX;
    assert!(publisher.begin(&summary.reference, contract).is_none());
    assert!(reader.bytes(&last).is_none());
    Ok(())
}

proptest! {
    #[test]
    fn finite_observer_traces_preserve_current_context_and_terminal_stops(events in prop::collection::vec((0u8..8, 0u8..6, 0u8..100), 1..120)) {
        trace(&events).map_err(|error| TestCaseError::fail(error.to_string()))?;
    }
}
fn trace(events: &[(u8, u8, u8)]) -> Result<()> {
    let summary = summary()?;
    let contract = summary.contract.as_ref().context("contract")?;
    let publisher = Publisher::new(b"trace host");
    let reader = publisher.reader();
    let mut handles: Vec<Option<Observation>> = Vec::new();
    let mut current = None;
    let mut phase = "idle";
    let mut elapsed = 0u32;
    for &(action, selection, time) in events {
        let time = u32::from(time);
        if action == 0 {
            handles.push(publisher.begin(&summary.reference, contract));
            current = handles.len().checked_sub(1);
            phase = "reading_source";
            elapsed = 0;
        } else if !handles.is_empty() {
            let index = usize::from(selection)
                .checked_rem(handles.len())
                .context("nonempty handles")?;
            let active = Some(index) == current
                && time >= elapsed
                && !matches!(phase, "failed" | "cancelled" | "admitted");
            let handle = handles.get_mut(index).context("handle")?;
            let present = handle.is_some();
            match action {
                1 => {
                    publisher.source(handle.as_ref(), time, "a".repeat(64));
                    if active && present && phase == "reading_source" {
                        phase = "preparing_checker";
                        elapsed = time;
                    }
                }
                2 => {
                    publisher.checking(handle.as_ref(), time, &inputs());
                    if active && present && phase == "preparing_checker" {
                        phase = "checking";
                        elapsed = time;
                    }
                }
                3 => {
                    publisher.refreshing(handle.as_ref(), time);
                    if active && present && phase == "checking" {
                        phase = "refreshing_source";
                        elapsed = time;
                    }
                }
                4 => {
                    publisher.retaining(handle.as_ref(), time);
                    if active && present && phase == "refreshing_source" {
                        phase = "retaining_evidence";
                        elapsed = time;
                    }
                }
                5 | 6 => {
                    let completion = if action == 5 {
                        Completion::Failed(FailureKind::Unverified, "Original failure")
                    } else {
                        Completion::Cancelled("Original stop")
                    };
                    publisher.finish(handle.take(), time, completion);
                    if active && present {
                        phase = if action == 5 { "failed" } else { "cancelled" };
                        elapsed = time;
                    }
                }
                _ => {
                    let _ = serde_json::to_value(&reader)?;
                }
            }
        }
        if current.is_some() {
            let (_, _, value) = body(&reader)?;
            let observed = at(&value, "/data/phase/state")?;
            let observed = if observed == "running" {
                at(&value, "/data/phase/stage/state")?
            } else {
                observed
            };
            assert_eq!(observed, phase);
            assert_eq!(at(&value, "/data/elapsed_ms")?, elapsed);
            assert_eq!(
                at(&value, "/data/context/reference/contract")?,
                summary.reference.contract
            );
        } else {
            assert_eq!(
                serde_json::to_value(&reader)?,
                serde_json::json!({"state":"idle"})
            );
        }
    }
    Ok(())
}
