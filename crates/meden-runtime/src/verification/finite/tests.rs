use super::*;
use std::sync::{atomic::AtomicBool, Arc};
fn control() -> Control {
    Control::new(Arc::new(AtomicBool::new(false)))
}
const CONTRACT: &str =
    include_str!("../../../../../verification/examples/finite-counter.contract.json");
const MODEL: &str = include_str!("../../../../../verification/examples/finite-counter.model.json");
const BROKEN: &str =
    include_str!("../../../../../verification/examples/finite-counter-broken.model.json");

#[test]
fn independent_enumeration_detects_wrap_and_refuses_incorrect_solver_verdicts() -> io::Result<()> {
    let c = Contract::parse(CONTRACT)?;
    let control = control();
    assert!(enumerate(&c, &c.model(MODEL)?, &control)?.is_none());
    let witness = enumerate(&c, &c.model(BROKEN)?, &control)?
        .ok_or_else(|| invalid("missing wrap witness"))?;
    assert_eq!(witness.before, vec![15]);
    assert_eq!(witness.arguments, vec![1]);
    assert_eq!(witness.after, vec![0]);
    assert_eq!(
        admit(Verdict::Unsat, Some(&witness)).err().map(|e| e.kind),
        Some(FailureKind::Unavailable)
    );
    assert_eq!(
        admit(Verdict::Sat, None).err().map(|e| e.kind),
        Some(FailureKind::Unavailable)
    );
    assert_eq!(
        admit(Verdict::Sat, Some(&witness)).err().map(|e| e.kind),
        Some(FailureKind::Unverified)
    );
    for text in [
        "",
        "unsat\nsat",
        "success\nunsat",
        "{\"verified\":true}",
        "(error \"unsupported\")\nunsat",
    ] {
        assert!(verdict(text).is_err());
    }
    assert_eq!(
        verdict("unknown").err().map(|e| e.kind),
        Some(FailureKind::Inconclusive)
    );
    Ok(())
}

#[test]
fn finite_contract_and_source_are_bounded_typed_data_not_commands() -> io::Result<()> {
    let c = Contract::parse(CONTRACT)?;
    for (pointer, value) in [
        ("/width", serde_json::json!(0)),
        ("/width", serde_json::json!(64)),
        ("/source_path", serde_json::json!("../host.json")),
        ("/format", serde_json::json!("run-command")),
        ("/properties", serde_json::json!([])),
    ] {
        let mut data: serde_json::Value = serde_json::from_str(CONTRACT)?;
        *data
            .pointer_mut(pointer)
            .ok_or_else(|| invalid("pointer"))? = value;
        assert!(Contract::parse(&data.to_string()).is_err());
    }
    for text in [
        "{\"version\":1,\"transition\":{\"value\":{\"op\":\"output\",\"field\":\"value\"}}}",
        "{\"version\":1,\"transition\":{\"value\":{\"op\":\"constant\",\"value\":16}}}",
        "{\"version\":1,\"transition\":{\"value\":{\"op\":\"include\",\"file\":\"/host\"}}}",
        "{\"version\":1,\"transition\":{\"value\":{\"op\":\"constant\",\"value\":0},\"value\":{\"op\":\"constant\",\"value\":1}}}",
    ] {assert!(c.model(text).is_err(),"{text}");}
    let program = c.program(&c.model(MODEL)?)?;
    assert!(program.contains("(set-logic QF_BV)"));
    assert!(!program.contains("counter.model.json"));
    Ok(())
}

// Real-tool prerequisites, never an inert worker or a successful skip. Workspace
// tests build the CLI integration executable; a focused runtime run can select
// its installed/pinned CLI explicitly through this host-only test variable.
#[cfg(target_os = "linux")]
fn worker() -> io::Result<PathBuf> {
    let path = if let Some(path) = std::env::var_os("MEDEN_TEST_WORKER") {
        PathBuf::from(path)
    } else {
        std::env::current_exe()?
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| invalid("Test executable has no target profile directory"))?
            .join("meden")
    };
    if !path.is_file() {
        return Err(invalid(
            "Build meden-cli first or set MEDEN_TEST_WORKER to the real CLI executable",
        ));
    }
    path.canonicalize()
}

#[cfg(target_os = "linux")]
#[test]
fn real_solver_output_faults_cannot_create_checked_output() -> Result<(), Failure> {
    let worker = worker()?;
    let c = Contract::parse(CONTRACT)?;
    let control = control();
    // Establish that this exact worker/profile actually runs the real solver.
    let checked = prepare(MODEL, &c, &worker, &control)?.check(&control)?;
    assert_eq!(checked.report().class, Class::Bounded);
    for (source, query, expected) in [
        (
            MODEL,
            "(echo \"unsat\")\n(echo \"sat\")\n(exit)\n",
            FailureKind::Unavailable,
        ),
        (
            MODEL,
            "(set-logic QF_BV)\n(not-a-command)\n(exit)\n",
            FailureKind::Unavailable,
        ),
        (
            MODEL,
            "(echo \"unknown\")\n(exit)\n",
            FailureKind::Inconclusive,
        ),
        // A successful real solver verdict for a wrong query must not validate
        // the original candidate. Both disagreement directions are exercised.
        (
            BROKEN,
            "(set-logic QF_BV)\n(assert false)\n(check-sat)\n(exit)\n",
            FailureKind::Unavailable,
        ),
        (
            MODEL,
            "(set-logic QF_BV)\n(check-sat)\n(exit)\n",
            FailureKind::Unavailable,
        ),
    ] {
        let mut prepared = prepare(source, &c, &worker, &control)?;
        // Deliberate internal adapter corruption. No public mutation API or
        // model/workspace configuration can supply or select these queries.
        prepared.program = query.into();
        assert_eq!(
            prepared.check(&control).err().map(|e| e.kind),
            Some(expected)
        );
    }
    let mut prepared = prepare(MODEL, &c, &worker, &control)?;
    prepared.program = format!("(echo \"{}\")\n(exit)\n", "x".repeat(200_000));
    assert_eq!(
        prepared.check(&control).err().map(|e| e.kind),
        Some(FailureKind::Inconclusive)
    );
    Ok(())
}

#[cfg(target_os = "linux")]
fn timeout_probe() -> String {
    use std::fmt::Write as _;
    // Unsatisfiable pigeonhole instance; the marker proves Z3 has entered the
    // query before timeout/cancellation. Generated names carry no user data.
    let mut query = String::from("(set-logic QF_BV)\n(echo \"SOLVER-ENTERED\")\n");
    for pigeon in 0..30 {
        for hole in 0..29 {
            let _ = writeln!(query, "(declare-const p{pigeon}_{hole} Bool)");
        }
        let _ = write!(query, "(assert (or");
        for hole in 0..29 {
            let _ = write!(query, " p{pigeon}_{hole}");
        }
        query.push_str("))\n");
    }
    for hole in 0..29 {
        for left in 0_u32..30 {
            for right in left.saturating_add(1)..30 {
                let _ = writeln!(
                    query,
                    "(assert (or (not p{left}_{hole}) (not p{right}_{hole})))"
                );
            }
        }
    }
    query.push_str("(check-sat)\n(exit)\n");
    query
}

#[cfg(target_os = "linux")]
#[test]
fn real_started_solver_timeout_and_cancellation_are_inconclusive() -> Result<(), Failure> {
    let worker = worker()?;
    let c = Contract::parse(CONTRACT)?;
    let control = control();
    let mut prepared = prepare(MODEL, &c, &worker, &control)?;
    prepared.program = timeout_probe();
    prepared.limits.timeout = Duration::from_secs(1);
    let failure = prepared
        .check(&control)
        .err()
        .ok_or_else(|| Failure::unavailable("Timeout probe unexpectedly completed"))?;
    assert_eq!(failure.kind, FailureKind::Inconclusive);
    assert!(failure.to_string().contains("TimedOut"));
    assert!(
        failure.to_string().contains("SOLVER-ENTERED"),
        "Z3 must actually start before the timeout"
    );

    let mut prepared = prepare(MODEL, &c, &worker, &control)?;
    prepared.program = timeout_probe();
    let observed = AtomicBool::new(false);
    let stop = |output: &process::OutputProgress| {
        if output.stdout.contains("SOLVER-ENTERED") {
            observed.store(true, std::sync::atomic::Ordering::SeqCst);
            control.cancel();
        }
    };
    let failure = prepared
        .check_observed(&control, Some(&stop))
        .err()
        .ok_or_else(|| Failure::unavailable("Cancellation probe unexpectedly completed"))?;
    assert!(observed.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(failure.kind, FailureKind::Inconclusive);
    assert!(failure.to_string().contains("Cancelled"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn changed_prepared_worker_is_refused_before_solver_dispatch() -> Result<(), Failure> {
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt as _;
    let original = worker()?;
    let directory = tempfile::tempdir()?;
    let worker = directory.path().join("meden-worker");
    std::fs::copy(original, &worker)?;
    // The original can be an immutable deployment. Only this owned mutation
    // fixture needs write permission; never relax the original worker's mode.
    std::fs::set_permissions(&worker, std::fs::Permissions::from_mode(0o700))?;
    let c = Contract::parse(CONTRACT)?;
    let control = control();
    let prepared = prepare(MODEL, &c, &worker, &control)?;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&worker)?
        .write_all(b"changed host object")?;
    let dispatched = AtomicBool::new(false);
    let observe = |_: &process::OutputProgress| {
        dispatched.store(true, std::sync::atomic::Ordering::SeqCst);
    };
    let failure = prepared
        .check_observed(&control, Some(&observe))
        .err()
        .ok_or_else(|| Failure::unavailable("Changed worker admitted"))?;
    assert_eq!(failure.kind, FailureKind::Unavailable);
    assert!(failure
        .to_string()
        .contains("resource resolution/content changed"));
    assert!(!dispatched.load(std::sync::atomic::Ordering::SeqCst));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn real_checked_output_cannot_complete_a_job_after_a_source_edit() -> Result<(), Failure> {
    use meden_core::{
        check::Preparation,
        evidence::{Contracts, EvidenceKind, EvidenceOrigin as Origin, Obligation},
    };
    let worker = worker()?;
    let control = control();
    let contract = Contract::parse(CONTRACT)?;
    let root = tempfile::tempdir()?;
    let source = root.path().join("counter.model.json");
    std::fs::write(&source, MODEL)?;
    let prepared = prepare(
        &std::fs::read_to_string(&source)?,
        &contract,
        &worker,
        &control,
    )?;
    let method_identity = prepared.inputs().checker_sha256.as_bytes().to_vec();
    let session = b"finite-stale-source";
    let mut current = Contracts::new(session.to_vec());
    assert!(current.propose(
        Origin::Controller,
        contract.canonical()?,
        vec![Obligation {
            statement: contract.canonical()?,
            kind: EvidenceKind::Bounded,
            checker: method_identity.clone()
        }]
    ));
    assert!(current.review(Origin::Host, session, 1));
    assert!(current.accept(Origin::Host, session, 1));
    let accepted = current
        .accepted()
        .ok_or_else(|| Failure::unavailable("No accepted view"))?;
    let running = Preparation::begin(&mut current, accepted, ())
        .and_then(|prepared_job| {
            prepared_job.issue(&mut current, serde_json::to_vec(prepared.inputs()).ok()?)
        })
        .ok_or_else(|| Failure::unavailable("No issued job"))?;
    let checked = prepared.check(&control)?;
    let mut observed = checked.inputs();
    // An external editor changes the real original source after the checker
    // completed. The original issued inputs cannot be replaced by this read.
    std::fs::write(&source, BROKEN)?;
    observed.source_sha256 = hash(&std::fs::read(&source)?);
    let storage = root.path().join("artifacts");
    let staged = checked.stage(&storage)?;
    let admitted = running.complete(
        &mut current,
        serde_json::to_vec(&observed).map_err(Failure::unavailable)?,
        b"candidate artifact".to_vec(),
        method_identity,
        staged,
    );
    assert!(admitted.is_none());
    assert!(!current.is_active() && !current.eligible(1, b"candidate artifact".to_vec()));
    assert_eq!(std::fs::read_dir(storage)?.count(), 0);
    Ok(())
}
