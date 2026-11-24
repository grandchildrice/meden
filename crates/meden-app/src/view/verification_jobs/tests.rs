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
        at(&start, "/data/context/subject/reference")?,
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
                at(&value, "/data/context/subject/reference/contract")?,
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

#[test]
fn actual_proposal_is_review_only_and_duplicate_read_does_not_start_a_job() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = project::Project::new(root.path());
    let reader = project.verification_view();
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let registry = crate::verification::Registry::host();
    let original =
        include_str!("../../../../../verification/examples/finite-counter.contract.json");
    let contract = registry.parse_selected(original, Some("z3.finite.v1"))?;
    let summary = project.propose(&contract, &control)?;
    let (revision, bytes, proposed) = body(&reader)?;
    assert_eq!(at(&proposed, "/data/context/subject/state")?, "proposal");
    assert_eq!(
        at(&proposed, "/data/context/subject/session")?,
        summary.reference.session
    );
    assert_eq!(
        at(&proposed, "/data/context/subject/contract")?,
        summary.reference.contract
    );
    assert_eq!(at(&proposed, "/data/phase/state")?, "proposed");
    assert_eq!(
        at(&proposed, "/data/phase/reference")?,
        serde_json::to_value(super::super::evidence::Reference::new(&summary.reference)?)?
    );
    assert!(!summary.accepted && !summary.verified && summary.artifact.is_none());
    project.propose(&contract, &control)?;
    assert!(Arc::ptr_eq(
        &bytes,
        &reader.bytes(&revision).context("unchanged duplicate")?
    ));
    project.review(&summary.reference)?;
    project.accept(&summary.reference)?;
    let replacement = registry.parse_selected(
        &original.replace("Finite counter", "Replacement counter"),
        Some("z3.finite.v1"),
    )?;
    let replacement = project.propose(&replacement, &control)?;
    let (_, _, next) = body(&reader)?;
    assert_eq!(at(&next, "/data/phase/state")?, "proposed");
    assert_ne!(
        at(&next, "/data/context/job")?,
        at(&proposed, "/data/context/job")?
    );
    assert!(replacement.accepted_reference.as_ref() == Some(&summary.reference));
    assert!(!replacement.accepted && !replacement.verified && replacement.artifact.is_none());
    assert!(reader.bytes(&revision).is_none());
    Ok(())
}

#[test]
fn proposal_completions_preserve_subject_and_cannot_admit_evidence() -> Result<()> {
    let summary = summary()?;
    let contract = summary.contract.as_ref().context("contract")?;
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    let draft = publisher.begin_proposal(&summary.reference.session, contract);
    publisher.source(draft.as_ref(), 1, "a".repeat(64));
    publisher.checking(draft.as_ref(), 2, &inputs());
    publisher.refreshing(draft.as_ref(), 3);
    publisher.retaining(draft.as_ref(), 4);
    assert_eq!(
        at(&body(&reader)?.2, "/data/phase/stage/state")?,
        "checking_proposal"
    );
    let old = body(&reader)?.0;
    publisher.finish(draft, 5, Completion::Admitted(&summary));
    assert!(reader.bytes(&old).is_none());
    assert_eq!(
        at(&serde_json::to_value(&reader)?, "/state")?,
        "unavailable"
    );
    for changed in ["accepted", "identity", "session", "source", "premature"] {
        let mut proposed = summary.clone();
        proposed.accepted = false;
        proposed.verified = false;
        proposed.artifact = None;
        proposed.source_sha256 = None;
        proposed.artifact_sha256 = None;
        match changed {
            "accepted" => proposed.accepted = true,
            "identity" => proposed.reference.contract = "0".repeat(64),
            "session" => proposed.reference.session = "another session".into(),
            "source" => proposed.source_path = "another.model.json".into(),
            _ => {}
        }
        let draft = publisher.begin_proposal(&summary.reference.session, contract);
        if changed != "premature" {
            publisher.recording_proposal(draft.as_ref(), 1);
        }
        let old = body(&reader)?.0;
        publisher.finish(draft, 2, Completion::Proposed(&proposed));
        assert!(reader.bytes(&old).is_none());
        assert_eq!(
            at(&serde_json::to_value(&reader)?, "/state")?,
            "unavailable"
        );
    }
    let accepted = publisher.begin(&summary.reference, contract);
    publisher.recording_proposal(accepted.as_ref(), 1);
    assert_eq!(
        at(&body(&reader)?.2, "/data/phase/stage/state")?,
        "reading_source"
    );
    publisher.finish(accepted, 2, Completion::Proposed(&summary));
    assert_eq!(
        at(&serde_json::to_value(&reader)?, "/state")?,
        "unavailable"
    );
    let old_draft = publisher.begin_proposal(&summary.reference.session, contract);
    let current = publisher.begin(&summary.reference, contract);
    let (revision, original, _) = body(&reader)?;
    publisher.recording_proposal(old_draft.as_ref(), 1);
    publisher.finish(old_draft, 2, Completion::Cancelled("retired draft"));
    assert!(Arc::ptr_eq(
        &original,
        &reader.bytes(&revision).context("original accepted job")?
    ));
    publisher.finish(current, 3, Completion::Cancelled("current host stop"));
    assert_eq!(
        at(&body(&reader)?.2, "/data/context/subject/state")?,
        "accepted"
    );
    Ok(())
}

#[test]
fn cancelled_actual_proposal_stays_unaccepted_and_a_fresh_turn_can_recover() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = project::Project::new(root.path());
    let reader = project.verification_view();
    let contract = crate::verification::Registry::host().parse_selected(
        include_str!("../../../../../verification/examples/finite-counter.contract.json"),
        Some("z3.finite.v1"),
    )?;
    let control = Control::new(Arc::new(AtomicBool::new(true)));
    assert!(project.propose(&contract, &control).is_err());
    let (cancelled_id, _, cancelled) = body(&reader)?;
    assert_eq!(at(&cancelled, "/data/phase/state")?, "cancelled");
    assert_eq!(
        at(&cancelled, "/data/phase/during/state")?,
        "checking_proposal"
    );
    assert!(!project.stored_status()?.proposed);
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    project.begin_turn();
    let summary = project.propose(&contract, &control)?;
    assert!(summary.proposed && !summary.accepted && !summary.verified);
    let (_, _, recovered) = body(&reader)?;
    assert_eq!(at(&recovered, "/data/phase/state")?, "proposed");
    assert_ne!(
        at(&cancelled, "/data/context/job")?,
        at(&recovered, "/data/context/job")?
    );
    assert!(reader.bytes(&cancelled_id).is_none());
    Ok(())
}
