//! Real offline finite verification through the public Meden worker executable.
use meden_runtime::{
    process::Control,
    verification::{
        artifact::Class,
        finite::{prepare, Contract},
        Failure, FailureKind,
    },
};
use std::sync::{atomic::AtomicBool, Arc};
fn control() -> Control {
    Control::new(Arc::new(AtomicBool::new(false)))
}
const CONTRACT: &str = include_str!("../../../verification/examples/finite-counter.contract.json");
const MODEL: &str = include_str!("../../../verification/examples/finite-counter.model.json");
const BROKEN: &str =
    include_str!("../../../verification/examples/finite-counter-broken.model.json");
#[test]
fn real_offline_z3_checks_the_original_finite_transition_and_refutes_the_mutation(
) -> Result<(), Failure> {
    let c = Contract::parse(CONTRACT)?;
    let control = control();
    let worker = std::path::PathBuf::from(env!("CARGO_BIN_EXE_meden"));
    let checked = prepare(MODEL, &c, &worker, &control)?.check(&control)?;
    assert_eq!(checked.report().class, Class::Bounded);
    let storage = tempfile::tempdir()?;
    let artifact = checked.stage(storage.path())?.commit();
    artifact.validate()?;
    assert_eq!(
        prepare(BROKEN, &c, &worker, &control)?
            .check(&control)
            .err()
            .map(|e| e.kind),
        Some(FailureKind::Unverified)
    );
    let prepared = prepare(MODEL, &c, &worker, &control)?;
    control.cancel();
    assert_eq!(
        prepared.check(&control).err().map(|e| e.kind),
        Some(FailureKind::Inconclusive)
    );
    assert_eq!(
        prepare(MODEL, &c, &worker, &control).err().map(|e| e.kind),
        Some(FailureKind::Inconclusive)
    );
    Ok(())
}
