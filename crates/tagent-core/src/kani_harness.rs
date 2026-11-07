//! Bounded model-checking harnesses, run with `cargo kani` in CI.
//!
//! Each harness states a property over *all* inputs within a small bound.
//! Kani (CBMC) explores them exhaustively, so a passing harness is a proof of
//! the property for those bounds, including absence of panics and arithmetic
//! overflow. The bounds are deliberately tiny: the point is the shape of the
//! argument, not coverage, which the Lean model and the conformance tests
//! provide.

use crate::attestation::{tier_of, Result, Tier};
use crate::capability::{Capability, Requirement};
use crate::glob::glob_match_chars;
use crate::policy::{check, Grant, Policy};

const ALPHABET: [char; 4] = ['a', 'b', '*', '?'];

fn any_cap() -> Capability {
    let i: usize = kani::any();
    kani::assume(i < Capability::ALL.len());
    Capability::ALL[i]
}

fn any_scope<const N: usize>(wild: bool) -> Option<String> {
    if kani::any() {
        return None;
    }
    let len: usize = kani::any();
    kani::assume(len <= N);
    let mut s = String::with_capacity(N);
    for _ in 0..len {
        let i: usize = kani::any();
        kani::assume(i < if wild { 4 } else { 2 });
        s.push(ALPHABET[i]);
    }
    Some(s)
}

/// `glob_match` never panics and a wildcard-free pattern matches only itself.
#[kani::proof]
#[kani::unwind(6)]
fn glob_literal_is_identity() {
    let n: usize = kani::any();
    kani::assume(n <= 3);
    let m: usize = kani::any();
    kani::assume(m <= 3);
    let mut p = [' '; 3];
    let mut t = [' '; 3];
    for i in 0..n {
        let c: usize = kani::any();
        kani::assume(c < 2);
        p[i] = ALPHABET[c];
    }
    for i in 0..m {
        let c: usize = kani::any();
        kani::assume(c < 2);
        t[i] = ALPHABET[c];
    }
    let matched = glob_match_chars(&p[..n], &t[..m]);
    assert_eq!(matched, p[..n] == t[..m]);
}

/// A lone `*` matches everything; `*` never panics on any text.
#[kani::proof]
#[kani::unwind(6)]
fn star_matches_everything() {
    let m: usize = kani::any();
    kani::assume(m <= 4);
    let mut t = [' '; 4];
    for i in 0..m {
        let c: usize = kani::any();
        kani::assume(c < 4);
        t[i] = ALPHABET[c];
    }
    assert!(glob_match_chars(&['*'], &t[..m]));
}

/// Deny always wins: a requirement whose capability is denied is never allowed.
#[kani::proof]
#[kani::unwind(8)]
fn deny_wins() {
    let req = Requirement::new(any_cap(), any_scope::<2>(false).as_deref());
    let grants = vec![
        Grant::new(any_cap(), any_scope::<2>(true).as_deref()),
        Grant::new(any_cap(), any_scope::<2>(true).as_deref()),
    ];
    let denies = vec![any_cap()];
    let policy = Policy {
        name: String::new(),
        grants,
        denies,
    };
    let d = check(std::slice::from_ref(&req), &policy);
    if policy.denies(req.capability) {
        assert!(!d.allowed);
    }
    assert_eq!(d.allowed, d.reasons.is_empty());
}

/// Allowed iff every requirement is granted and none denied (two requirements).
#[kani::proof]
#[kani::unwind(8)]
fn allowed_iff_granted_and_not_denied() {
    let reqs = [
        Requirement::new(any_cap(), any_scope::<2>(false).as_deref()),
        Requirement::new(any_cap(), any_scope::<2>(false).as_deref()),
    ];
    let policy = Policy {
        name: String::new(),
        grants: vec![Grant::new(any_cap(), any_scope::<2>(true).as_deref())],
        denies: vec![any_cap()],
    };
    let d = check(&reqs, &policy);
    let expected = reqs
        .iter()
        .all(|r| !policy.denies(r.capability) && policy.grants(r));
    assert_eq!(d.allowed, expected);
}

/// The tier rule: only `proved` can verify; a Lean-only TCB verifies without
/// approval; a foreign TCB verifies only with an approved backend.
#[kani::proof]
#[kani::unwind(4)]
fn tier_rule() {
    let results = [
        Result::Proved,
        Result::Tested,
        Result::Scanned,
        Result::Failed,
    ];
    let ri: usize = kani::any();
    kani::assume(ri < 4);
    let result = results[ri];
    let foreign: bool = kani::any();
    let tcb = if foreign {
        vec!["z3".to_owned()]
    } else {
        vec!["lean4".to_owned()]
    };
    let approved: bool = kani::any();
    let backend = "verus+z3";
    let approved_backends = if approved {
        vec![backend.to_owned()]
    } else {
        vec![]
    };
    let t = tier_of(result, &tcb, backend, &approved_backends);
    if result != Result::Proved {
        assert_eq!(t, Tier::NotVerified);
    } else if !foreign {
        assert_eq!(t, Tier::Verified);
    } else if approved {
        assert_eq!(t, Tier::VerifiedWithTcb);
    } else {
        assert_eq!(t, Tier::NotVerified);
    }
}
