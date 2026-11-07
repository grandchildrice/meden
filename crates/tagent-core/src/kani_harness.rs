//! Bounded model-checking harnesses, run with `cargo kani -p tagent-core` in CI.
//!
//! Each harness states a property over *all* inputs within a small bound.
//! Kani (CBMC) explores them exhaustively, so a passing harness is a proof of
//! the property for those bounds, including absence of panics and arithmetic
//! overflow. Scopes are drawn from a fixed menu of concrete strings by a
//! symbolic index: the choice is exhaustive, the strings are not symbolic,
//! which keeps the SAT problems small. The Lean model carries the unbounded
//! argument; these harnesses pin the Rust code to it.

use crate::attestation::{tier_of, Result, Tier};
use crate::capability::{Capability, Requirement};
use crate::glob::glob_match_chars;
use crate::policy::{allowed, violates, Grant, Policy};

/// Scopes a requirement may carry (concrete). Kept to one character: the
/// glob itself is covered by its own harnesses and by the Lean model; here
/// the scope only has to be able to match or not match a grant.
const REQ_SCOPES: [&str; 2] = ["a", "b"];
/// Scopes a grant may carry (concrete, with wildcards).
const GRANT_SCOPES: [&str; 3] = ["a", "*", "?"];

fn any_cap() -> Capability {
    let i: usize = kani::any();
    kani::assume(i < Capability::ALL.len());
    Capability::ALL[i]
}

fn any_req() -> Requirement {
    let scoped: bool = kani::any();
    let i: usize = kani::any();
    kani::assume(i < REQ_SCOPES.len());
    Requirement::new(any_cap(), scoped.then_some(REQ_SCOPES[i]))
}

fn any_grant() -> Grant {
    let scoped: bool = kani::any();
    let i: usize = kani::any();
    kani::assume(i < GRANT_SCOPES.len());
    Grant::new(any_cap(), scoped.then_some(GRANT_SCOPES[i]))
}

/// `glob_match` never panics and a wildcard-free pattern matches only itself.
#[kani::proof]
#[kani::unwind(5)]
fn glob_literal_is_identity() {
    const ALPHABET: [char; 2] = ['a', 'b'];
    let n: usize = kani::any();
    kani::assume(n <= 3);
    let m: usize = kani::any();
    kani::assume(m <= 3);
    let mut p = [' '; 3];
    let mut t = [' '; 3];
    for i in 0..n {
        let c: usize = kani::any();
        kani::assume(c < ALPHABET.len());
        p[i] = ALPHABET[c];
    }
    for i in 0..m {
        let c: usize = kani::any();
        kani::assume(c < ALPHABET.len());
        t[i] = ALPHABET[c];
    }
    assert_eq!(glob_match_chars(&p[..n], &t[..m]), p[..n] == t[..m]);
}

/// A lone `*` matches every text (and never panics on wildcards in the text).
#[kani::proof]
#[kani::unwind(6)]
fn star_matches_everything() {
    const ALPHABET: [char; 4] = ['a', 'b', '*', '?'];
    let m: usize = kani::any();
    kani::assume(m <= 4);
    let mut t = [' '; 4];
    for i in 0..m {
        let c: usize = kani::any();
        kani::assume(c < ALPHABET.len());
        t[i] = ALPHABET[c];
    }
    assert!(glob_match_chars(&['*'], &t[..m]));
}

/// Deny always wins: a requirement on a denied capability is violated,
/// whatever the grants are.
#[kani::proof]
#[kani::unwind(6)]
fn deny_wins() {
    let req = any_req();
    let policy = Policy {
        name: String::new(),
        grants: vec![any_grant()],
        denies: vec![any_cap()],
    };
    if policy.denies(req.capability) {
        assert!(violates(&policy, &req));
    }
}

/// `allowed` is exactly "every requirement granted and none denied".
#[kani::proof]
#[kani::unwind(6)]
fn allowed_iff_granted_and_not_denied() {
    let reqs = [any_req(), any_req()];
    let policy = Policy {
        name: String::new(),
        grants: vec![any_grant()],
        denies: vec![any_cap()],
    };
    let expected = reqs
        .iter()
        .all(|r| !policy.denies(r.capability) && policy.grants(r));
    assert_eq!(allowed(&reqs, &policy), expected);
}

/// Adding a grant never turns an allowance into a denial.
#[kani::proof]
#[kani::unwind(6)]
fn grants_are_monotone() {
    let reqs = [any_req()];
    let g1 = any_grant();
    let g2 = any_grant();
    let smaller = Policy {
        name: String::new(),
        grants: vec![g1.clone()],
        denies: vec![],
    };
    let larger = Policy {
        name: String::new(),
        grants: vec![g1, g2],
        denies: vec![],
    };
    let before = allowed(&reqs, &smaller);
    let after = allowed(&reqs, &larger);
    assert!(!before || after);
}

/// A scoped grant never covers an unscoped requirement; an unscoped grant
/// covers every requirement on its capability.
#[kani::proof]
#[kani::unwind(6)]
fn scope_rules() {
    let cap = any_cap();
    let g = any_grant();
    let unscoped_req = Requirement::new(cap, None);
    if g.capability == cap && g.scope.is_some() {
        assert!(!g.covers(&unscoped_req));
    }
    let req = any_req();
    if req.capability == cap {
        assert!(Grant::new(cap, None).covers(&req));
    }
}

/// The tier rule: only `proved` can verify; a Lean-only TCB verifies without
/// approval; a foreign TCB verifies only with an approved backend.
#[kani::proof]
#[kani::unwind(12)]
fn tier_rule() {
    const RESULTS: [Result; 4] = [
        Result::Proved,
        Result::Tested,
        Result::Scanned,
        Result::Failed,
    ];
    let ri: usize = kani::any();
    kani::assume(ri < RESULTS.len());
    let result = RESULTS[ri];
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
