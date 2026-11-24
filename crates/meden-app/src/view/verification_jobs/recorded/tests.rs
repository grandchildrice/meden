use super::*;
use crate::view::verification_jobs::{Reader, Role};
use crate::view::{publication::ArtifactId, verification_jobs::Artifact};
use serde_json::{json, Value};
use std::sync::Arc;

fn work() -> Work {
    Work {
        id: "proof".into(),
        title: "Original theorem".into(),
        requirement: "Every natural number equals itself; preserve this requirement.".into(),
        module: "Pilot".into(),
        theorems: vec!["portable_identity".into()],
        next: "Inspect the same theorem.".into(),
    }
}
fn record(outcome: Outcome, success: bool) -> store::Observation {
    let work = work();
    store::Observation {
        event: format!("event_{}", "c".repeat(64)),
        outcome,
        record: Check {
            engine_sha256: format!("sha256:{}", "a".repeat(64)),
            requirement: work.requirement,
            theorems: work.theorems,
            inspection: "Original named-theorem observation".into(),
            source_revision: format!("sha256:{}", "b".repeat(64)),
            after_revision: Some(format!("sha256:{}", "b".repeat(64))),
            module: work.module,
            success,
            termination: "Original host status".into(),
            stdout: "Cancelled unavailable proof passed".into(),
            stderr: String::new(),
        },
    }
}
fn body(reader: &Reader) -> Result<(ArtifactId<Artifact>, Arc<[u8]>, Value)> {
    let status = serde_json::to_value(reader)?;
    let revision =
        serde_json::from_value(status.get("revision").context("Ready reference")?.clone())?;
    let bytes = reader.bytes(&revision).context("Frozen original body")?;
    let value = serde_json::from_slice(&bytes)?;
    Ok((revision, bytes, value))
}
fn at(value: &Value, pointer: &str) -> Result<Value> {
    value.pointer(pointer).cloned().context("Original field")
}
fn start(publisher: &Publisher, origin: Origin) -> Result<Observation> {
    let observation = publisher
        .begin_recorded(b"original session", "proof", origin)
        .context("New check")?;
    publisher.recorded_work(Some(&observation), 1, &work());
    publisher.recorded_engine(Some(&observation), 2, &format!("sha256:{}", "a".repeat(64)));
    publisher.recorded_source(Some(&observation), 3, &format!("sha256:{}", "b".repeat(64)));
    publisher.recorded_step(Some(&observation), 4, Step::Building);
    Ok(observation)
}
fn retain(publisher: &Publisher, observation: &Observation) {
    publisher.recorded_step(Some(observation), 5, Step::Inspecting);
    publisher.recorded_step(Some(observation), 6, Step::Refreshing);
    publisher.recorded_step(Some(observation), 7, Step::Persisting);
}

#[test]
fn original_historical_inputs_and_saved_result_never_become_project_evidence() -> Result<()> {
    // Projection fixture only: no compiler or evidence admission is simulated.
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    let observation = start(&publisher, Origin::Model)?;
    let (old, _, running) = body(&reader)?;
    assert_eq!(at(&running, "/data/kind")?, "recorded");
    assert_eq!(at(&running, "/data/context/origin")?, "model");
    assert_eq!(
        at(&running, "/data/phase/stage/inputs/work")?,
        serde_json::to_value(work())?
    );
    retain(&publisher, &observation);
    publisher.finish(
        Some(observation),
        8,
        Completion::Recorded(&record(Outcome::Completed, true)),
    );
    let (revision, bytes, completed) = body(&reader)?;
    assert_eq!(at(&completed, "/data/phase/state")?, "historical");
    assert_eq!(at(&completed, "/data/phase/result")?, "checked");
    assert_eq!(
        at(&completed, "/data/phase/inputs")?,
        at(&running, "/data/phase/stage/inputs")?
    );
    assert!(completed.pointer("/data/phase/accepted").is_none());
    assert!(completed.pointer("/data/phase/verified").is_none());
    assert!(reader.bytes(&old).is_none());
    for _ in 0..100 {
        assert!(Arc::ptr_eq(
            &bytes,
            &reader.bytes(&revision).context("Repeated immutable read")?
        ));
    }
    assert_eq!(
        at(&serde_json::to_value(&reader)?, "/role")?,
        serde_json::to_value(Role::RecordedModel)?
    );
    Ok(())
}

#[test]
fn original_outcome_not_diagnostic_words_selects_saved_history_result() -> Result<()> {
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    for (outcome, expected) in [
        (Outcome::Completed, "rejected"),
        (Outcome::Cancelled, "cancelled"),
        (Outcome::Inconclusive, "inconclusive"),
        (Outcome::Unavailable, "unavailable"),
    ] {
        let observation = start(&publisher, Origin::Host)?;
        retain(&publisher, &observation);
        publisher.finish(
            Some(observation),
            8,
            Completion::Recorded(&record(outcome, false)),
        );
        assert_eq!(at(&body(&reader)?.2, "/data/phase/result")?, expected);
    }
    Ok(())
}

#[test]
fn premature_or_foreign_record_completion_retires_bytes() -> Result<()> {
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    for fault in [
        "premature",
        "requirement",
        "engine",
        "source",
        "module",
        "theorem",
        "event",
        "after",
        "outcome",
        "rejected_changed_source",
        "inconclusive_missing_source",
    ] {
        let observation = start(&publisher, Origin::Host)?;
        if fault != "premature" {
            retain(&publisher, &observation);
        }
        let mut value = record(Outcome::Completed, true);
        match fault {
            "requirement" => value.record.requirement = "Replacement meaning".into(),
            "engine" => value.record.engine_sha256 = format!("sha256:{}", "0".repeat(64)),
            "source" => value.record.source_revision = format!("sha256:{}", "0".repeat(64)),
            "module" => value.record.module = "Another".into(),
            "theorem" => value.record.theorems = vec!["another".into()],
            "event" => value.event = "not retained".into(),
            "after" => value.record.after_revision = None,
            "outcome" => value.outcome = Outcome::Cancelled,
            "rejected_changed_source" => {
                value.record.success = false;
                value.record.after_revision = Some(format!("sha256:{}", "0".repeat(64)));
            }
            "inconclusive_missing_source" => {
                value.record.success = false;
                value.outcome = Outcome::Inconclusive;
                value.record.after_revision = None;
            }
            _ => {}
        }
        let (old, _, _) = body(&reader)?;
        publisher.finish(Some(observation), 8, Completion::Recorded(&value));
        assert_eq!(
            at(&serde_json::to_value(&reader)?, "/state")?,
            "unavailable"
        );
        assert!(reader.bytes(&old).is_none());
    }
    Ok(())
}

#[test]
fn replacement_backwards_and_terminal_notifications_cannot_restore_recorded_jobs() -> Result<()> {
    let publisher = Publisher::new(b"host");
    let reader = publisher.reader();
    let old = start(&publisher, Origin::Model)?;
    let current = start(&publisher, Origin::Model)?;
    let (revision, bytes, _) = body(&reader)?;
    publisher.recorded_step(Some(&old), 20, Step::Refreshing);
    publisher.finish(
        Some(old),
        21,
        Completion::Recorded(&record(Outcome::Completed, true)),
    );
    publisher.recorded_step(Some(&current), 3, Step::Refreshing);
    assert!(Arc::ptr_eq(
        &bytes,
        &reader.bytes(&revision).context("Original current state")?
    ));
    let late = Observation {
        job: current.job.clone(),
    };
    publisher.finish(
        Some(current),
        5,
        Completion::Cancelled("Original host cancellation"),
    );
    let (revision, bytes, cancelled) = body(&reader)?;
    assert_eq!(at(&cancelled, "/data/phase/state")?, "cancelled");
    assert_eq!(at(&cancelled, "/data/phase/during/state")?, "building");
    publisher.recorded_step(Some(&late), 100, Step::Inspecting);
    publisher.finish(
        Some(late),
        101,
        Completion::Recorded(&record(Outcome::Completed, true)),
    );
    assert!(Arc::ptr_eq(
        &bytes,
        &reader.bytes(&revision).context("Original terminal state")?
    ));
    Ok(())
}

#[test]
fn explicit_host_check_preserves_conversation_publication_and_has_a_distinct_job() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = crate::project::Project::new(root.path()).expect("valid fixture workspace");
    let chat = project.check_observations(Origin::Model);
    let host = project.check_observations(Origin::Host);
    let chat_observation = start(chat, Origin::Model)?;
    let (chat_revision, chat_bytes, _) = body(&project.verification_view())?;
    let host_observation = start(host, Origin::Host)?;
    assert_ne!(host_observation.job, chat_observation.job);
    retain(host, &host_observation);
    host.finish(
        Some(host_observation),
        8,
        Completion::Recorded(&record(Outcome::Completed, true)),
    );
    assert!(Arc::ptr_eq(
        &chat_bytes,
        &project
            .verification_view()
            .bytes(&chat_revision)
            .context("Chat state retained through host work")?
    ));
    assert_eq!(
        at(&body(&project.work_view())?.2, "/data/phase/state")?,
        "historical"
    );
    assert_eq!(
        serde_json::to_value(Publisher::new(b"fresh host").reader())?,
        json!({"state":"idle"})
    );
    Ok(())
}
