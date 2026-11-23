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
