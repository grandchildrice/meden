//! Bounded model-checking harnesses for the flow rules.
//!
//! These follow the same discipline as `tagent-core`'s (see CONTRIBUTING.md):
//! no heap, no panicking operation, no symbolic-length loops. That is why the
//! rules in [`crate::gate`] come in a flag form: a label's provenance and
//! readers are heap data, but *whether* a label is trusted and *whether* it
//! permits a reader are booleans, and the rule depends on nothing else. The
//! property tests in `gate.rs` tie the label form to the flag form, and
//! `lean/TAgentSpec/Flow.lean` carries the unbounded argument.

use crate::gate::{disclosure_ok_flags, integrity_ok_flags};

/// **The anti-injection property**, for every pair of arguments: if any
/// argument steers the call and is not trusted, the call is refused.
#[kani::proof]
#[kani::unwind(4)]
fn untrusted_control_is_refused() {
    let args: [(bool, bool); 2] = [(kani::any(), kani::any()), (kani::any(), kani::any())];
    let ok = integrity_ok_flags(&args);
    let some_untrusted_steers = args.iter().any(|(control, trusted)| *control && !*trusted);
    assert!(ok != some_untrusted_steers);
}

/// Trusted data may steer, and untrusted data that does not steer is fine.
#[kani::proof]
#[kani::unwind(4)]
fn data_that_does_not_steer_is_allowed() {
    let trusted_a: bool = kani::any();
    let trusted_b: bool = kani::any();
    // Neither argument steers, so neither one's trust matters.
    assert!(integrity_ok_flags(&[
        (false, trusted_a),
        (false, trusted_b)
    ]));
}

/// **The anti-exfiltration property**: if any argument does not permit the
/// reader, the disclosure is refused.
#[kani::proof]
#[kani::unwind(4)]
fn unreadable_is_not_disclosed() {
    let permits: [bool; 3] = [kani::any(), kani::any(), kani::any()];
    let ok = disclosure_ok_flags(&permits);
    let some_refuses = permits.iter().any(|p| !*p);
    assert!(ok != some_refuses);
}

/// Adding an argument never turns a refusal into an acceptance.
#[kani::proof]
#[kani::unwind(4)]
fn rules_are_monotone_in_arguments() {
    let a: (bool, bool) = (kani::any(), kani::any());
    let b: (bool, bool) = (kani::any(), kani::any());
    assert!(!integrity_ok_flags(&[a, b]) || integrity_ok_flags(&[a]));

    let p: bool = kani::any();
    let q: bool = kani::any();
    assert!(!disclosure_ok_flags(&[p, q]) || disclosure_ok_flags(&[p]));
}
