//! Registry/replay conformance fixtures; public TUI/GUI tests run actual Z3.
#![allow(clippy::expect_used)]
use super::*;
use crate::verification::binding_tests;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const DATA: &str = include_str!("../../../../verification/examples/finite-counter.contract.json");
const FIRST: &str = "host.first";
const SECOND: &str = "host.second";

#[derive(Default)]
struct Methods {
    phase: Arc<AtomicU8>,
    first: Arc<AtomicBool>,
    second: Arc<AtomicBool>,
}
impl Methods {
    fn registry(&self, second: bool) -> Result<Verifiers> {
        let mut registry = Verifiers::default();
        binding_tests::register(
            &mut registry,
            FIRST,
            Arc::clone(&self.phase),
            Arc::clone(&self.first),
        )?;
        if second {
            binding_tests::register(
                &mut registry,
                SECOND,
                Arc::clone(&self.phase),
                Arc::clone(&self.second),
            )?;
        }
        Ok(registry)
    }
    fn open(
        &self,
        root: &Path,
        storage: &Path,
        session: &[u8],
        selected: Option<&str>,
        second: bool,
    ) -> Result<Project> {
        Project::open_stored(root, session, selected, self.registry(second)?, storage)
    }
}
fn accepted(project: &Project) -> Result<Summary> {
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let contract = project.parse_contract(DATA)?;
    let summary = project.propose(&contract, &control)?;
    project.review(summary.reference.as_ref().expect("original proposal"))?;
    project.accept(summary.reference.as_ref().expect("original proposal"))
}

#[test]
fn explicit_same_format_selection_survives_restart_and_switch_requires_review() -> Result<()> {
    let root = tempfile::tempdir()?;
    let storage = tempfile::tempdir()?;
    let methods = Methods::default();
    assert!(methods.registry(true)?.parse(DATA).is_err());
    let project = methods.open(root.path(), storage.path(), b"one", Some(SECOND), true)?;
    assert_eq!(project.selected_verifier(), Some(SECOND));
    let initial = accepted(&project)?;
    let original = initial.contract.context("contract")?;
    assert_eq!(original.backend(), SECOND);
    assert!(!methods.first.load(Ordering::SeqCst));
    assert!(methods.second.load(Ordering::SeqCst));
    assert!(initial.review.contains(SECOND));
    methods.second.store(false, Ordering::SeqCst);
    drop(project);

    let restored = methods.open(root.path(), storage.path(), b"two", None, true)?;
    assert_eq!(restored.selected_verifier(), Some(SECOND));
    let summary = restored.stored_status()?;
    assert!(summary.accepted && !summary.verified && !restored.take_autorun());
    assert_ne!(
        summary
            .reference
            .as_ref()
            .expect("original proposal")
            .session,
        initial
            .reference
            .as_ref()
            .expect("original proposal")
            .session
    );
    assert_eq!(summary.contract, Some(original.clone()));
    assert_eq!(restored.parse_contract(DATA)?.backend(), SECOND);
    restored.prepare_access()?;
    assert!(!methods.second.load(Ordering::SeqCst));
    drop(restored);

    let switched = methods.open(root.path(), storage.path(), b"three", Some(FIRST), true)?;
    let old = switched
        .stored_status()?
        .accepted_contract
        .context("accepted")?;
    assert_eq!(old, original);
    assert!(switched.selection(&old).is_err());
    let replacement = switched.parse_contract(DATA)?;
    assert_eq!(replacement.canonical()?, old.canonical()?);
    assert_ne!(replacement.checker_id()?, old.checker_id()?);
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let draft = switched.propose(&replacement, &control)?;
    assert!(
        draft
            .reference
            .as_ref()
            .expect("original proposal")
            .revision
            > initial
                .reference
                .as_ref()
                .expect("original proposal")
                .revision
    );
    assert_eq!(draft.accepted_contract, Some(old.clone()));
    assert!(switched
        .accept(draft.reference.as_ref().expect("original proposal"))
        .is_err());
    switched.reject(draft.reference.as_ref().expect("original proposal"))?;
    assert_eq!(switched.stored_status()?.accepted_contract, Some(old));
    let renewed = switched.propose(&replacement, &control)?;
    assert!(
        renewed
            .reference
            .as_ref()
            .expect("original proposal")
            .revision
            > draft
                .reference
                .as_ref()
                .expect("original proposal")
                .revision
    );
    assert!(switched
        .review(draft.reference.as_ref().expect("original proposal"))
        .is_err());
    switched.review(renewed.reference.as_ref().expect("original proposal"))?;
    let accepted = switched.accept(renewed.reference.as_ref().expect("original proposal"))?;
    assert_eq!(accepted.contract.context("replacement")?.backend(), FIRST);
    assert!(!accepted.verified);
    Ok(())
}

#[test]
fn unknown_selection_and_changed_method_never_fallback_or_relabel_history() -> Result<()> {
    let root = tempfile::tempdir()?;
    let storage = tempfile::tempdir()?;
    let methods = Methods::default();
    let project = methods.open(root.path(), storage.path(), b"one", Some(SECOND), true)?;
    let original = accepted(&project)?.contract.context("original")?;
    drop(project);
    let path = storage.path().join("contracts.json");
    let before = std::fs::read(&path)?;
    assert!(methods
        .open(root.path(), storage.path(), b"two", None, false)
        .is_err());
    assert!(methods
        .open(root.path(), storage.path(), b"two", Some("unknown"), true)
        .is_err());
    assert_eq!(std::fs::read(&path)?, before);

    let unavailable = methods.open(root.path(), storage.path(), b"two", Some(FIRST), false)?;
    assert_eq!(
        unavailable.stored_status()?.accepted_contract,
        Some(original.clone())
    );
    assert!(unavailable.prepare_access().is_err());
    drop(unavailable);

    methods.phase.store(1, Ordering::SeqCst);
    let changed = methods.open(root.path(), storage.path(), b"three", Some(SECOND), true)?;
    assert_eq!(
        changed.stored_status()?.accepted_contract,
        Some(original.clone())
    );
    assert!(changed.stored_status()?.review.contains("@ 1"));
    assert!(changed.prepare_access().is_err());
    assert!(changed.verifiers.check_binding(&original).is_err());
    let current = changed.parse_contract(DATA)?;
    assert_ne!(original.descriptor(), current.descriptor());
    assert_eq!(original.canonical()?, current.canonical()?);
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let replacement = changed.propose(&current, &control)?;
    assert!(replacement.proposed && !replacement.accepted);
    assert_eq!(replacement.accepted_contract, Some(original));
    assert!(changed
        .accept(replacement.reference.as_ref().expect("original proposal"))
        .is_err());
    Ok(())
}

#[test]
fn history_without_method_metadata_stays_unknown_and_recovers_by_new_host_review() -> Result<()> {
    let root = tempfile::tempdir()?;
    let storage = tempfile::tempdir()?;
    let project =
        Project::open_stored(root.path(), b"one", None, Verifiers::host(), storage.path())?;
    let original = accepted(&project)?.contract.context("original")?;
    drop(project);
    let path = storage.path().join("contracts.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    saved.as_object_mut().context("journal")?.remove("selected");
    let legacy_checker =
        meden_runtime::verification::finite::Contract::parse(DATA)?.checker_id()?;
    let proposal = saved
        .pointer_mut("/events/0")
        .and_then(Value::as_object_mut)
        .context("proposal")?;
    proposal.remove("binding");
    *proposal
        .get_mut("required")
        .and_then(|v| v.get_mut("checker"))
        .context("checker")? = serde_json::to_value(legacy_checker.as_bytes())?;
    crate::storage::write(&path, &saved)?;

    let restored = Project::open_stored(
        root.path(),
        b"two",
        Some("z3.finite.v1"),
        Verifiers::host(),
        storage.path(),
    )?;
    let historical = restored.stored_status()?;
    assert!(historical.accepted && !historical.verified);
    let unknown = historical.contract.context("original meaning")?;
    assert_eq!(unknown.canonical()?, original.canonical()?);
    assert!(unknown.descriptor().is_none() && unknown.backend().is_empty());
    assert!(historical
        .review
        .contains("original verifier was not recorded"));
    assert!(restored.prepare_access().is_err());
    let current = restored.parse_contract(DATA)?;
    let control = Control::new(Arc::new(AtomicBool::new(false)));
    let replacement = restored.propose(&current, &control)?;
    assert_eq!(replacement.accepted_contract, Some(unknown));
    assert!(
        replacement
            .reference
            .as_ref()
            .expect("original proposal")
            .revision
            > historical
                .reference
                .as_ref()
                .expect("original proposal")
                .revision
    );
    assert!(restored
        .accept(replacement.reference.as_ref().expect("original proposal"))
        .is_err());
    restored.review(replacement.reference.as_ref().expect("original proposal"))?;
    let accepted = restored.accept(replacement.reference.as_ref().expect("original proposal"))?;
    assert!(accepted.accepted && !accepted.verified);
    restored.prepare_access()?;
    assert!(restored
        .state
        .lock()
        .map_err(|_| anyhow::anyhow!("state"))?
        .events
        .first()
        .is_some_and(|e| matches!(e, journal::Event::Propose { binding: None, .. })));
    Ok(())
}

#[test]
fn stripped_binding_or_changed_source_metadata_never_provides_file_authority() -> Result<()> {
    let root = tempfile::tempdir()?;
    let storage = tempfile::tempdir()?;
    let methods = Methods::default();
    let project = methods.open(root.path(), storage.path(), b"one", Some(SECOND), true)?;
    accepted(&project)?;
    drop(project);
    let path = storage.path().join("contracts.json");
    let original: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    let mut stripped = original.clone();
    stripped
        .pointer_mut("/events/0")
        .and_then(Value::as_object_mut)
        .context("proposal")?
        .remove("binding");
    crate::storage::write(&path, &stripped)?;
    assert!(methods
        .open(root.path(), storage.path(), b"two", None, true)
        .is_err());
    assert_eq!(crate::storage::read::<Value>(&path)?, Some(stripped));

    let mut forged = original;
    *forged
        .pointer_mut("/events/0/binding/description/source")
        .context("source")? = "private/other.json".into();
    crate::storage::write(&path, &forged)?;
    let restored = methods.open(root.path(), storage.path(), b"two", None, true)?;
    let error = restored
        .prepare_access()
        .err()
        .context("binding must fail before file authority")?;
    assert_eq!(
        error
            .downcast_ref::<Failure>()
            .context("typed failure")?
            .kind,
        FailureKind::Unavailable
    );
    assert!(restored
        .verifiers
        .check_binding(
            &restored
                .stored_status()?
                .accepted_contract
                .context("retained")?
        )
        .is_err());
    Ok(())
}
