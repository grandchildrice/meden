//! Bounded model-checking harnesses, run with `cargo kani -p meden-core` in CI.
//!
//! Each harness states a property over *all* inputs within a small bound.
//! Kani (CBMC) explores them exhaustively, so a passing harness is a proof of
//! the property for those bounds, including absence of panics and arithmetic
//! overflow.
//!
//! Three rules keep these cheap enough for a hosted runner:
//!
//! * **No heap.** The harnesses run the rule on stack data only ([`GrantRef`],
//!   [`ReqRef`], char arrays). Scopes come from a fixed menu of concrete
//!   strings chosen by a symbolic index, so no string is symbolic.
//! * **No panicking operation.** Every array access goes through `get`, so
//!   slice bounds checks do not enter the model, and assertions are
//!   `assert!` rather than `assert_eq!`, which keeps `Debug` formatting out.
//! * **No symbolic-length loops.** Arrays have a fixed length and symbolic
//!   contents; a symbolic length is applied afterwards with `get(..n)`.
//!
//! The Lean model carries the unbounded argument; these harnesses pin the
//! Rust code to it, and the property tests in `policy.rs` and
//! `attestation.rs` pin the owned APIs to the borrowed ones used here.

use crate::attestation::{tier_of_ref, Result, Tier};
use crate::capability::Capability;
use crate::glob::glob_match_chars;
use crate::policy::{allowed_ref, covers_ref, violates_ref, GrantRef, ReqRef};

/// Scopes a requirement may carry (concrete).
const REQ_SCOPES: [&str; 2] = ["a", "b"];
/// Scopes a grant may carry (concrete, with wildcards).
const GRANT_SCOPES: [&str; 3] = ["a", "*", "?"];
/// Characters a symbolic pattern or text is built from.
const ALPHABET: [char; 2] = ['a', 'b'];

fn any_cap() -> Capability {
    let i: usize = kani::any();
    kani::assume(i < Capability::ALL.len());
    // The assume makes the fallback unreachable; `get` keeps the panic
    // branch out of the model.
    *Capability::ALL.get(i).unwrap_or(&Capability::UiControl)
}

fn any_req() -> ReqRef<'static> {
    let scoped: bool = kani::any();
    let i: usize = kani::any();
    kani::assume(i < REQ_SCOPES.len());
    let scope = REQ_SCOPES.get(i).copied().unwrap_or("a");
    ReqRef {
        capability: any_cap(),
        scope: scoped.then_some(scope),
    }
}

fn any_grant() -> GrantRef<'static> {
    let scoped: bool = kani::any();
    let i: usize = kani::any();
    kani::assume(i < GRANT_SCOPES.len());
    let scope = GRANT_SCOPES.get(i).copied().unwrap_or("a");
    GrantRef {
        capability: any_cap(),
        scope: scoped.then_some(scope),
    }
}

fn any_char() -> char {
    let i: usize = kani::any();
    kani::assume(i < ALPHABET.len());
    ALPHABET.get(i).copied().unwrap_or('a')
}

/// Slice equality without `memcmp`.
///
/// `&[char] == &[char]` lowers to a byte-wise `memcmp`, whose builtin loop
/// would need an unwind bound of four times the length; comparing element by
/// element keeps the bound proportional to the number of characters.
fn chars_eq(a: &[char], b: &[char]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x == y)
}

/// `glob_match_chars` never panics, and a wildcard-free pattern matches
/// exactly itself.
#[kani::proof]
#[kani::unwind(6)]
fn glob_literal_is_identity() {
    let p = [any_char(), any_char(), any_char()];
    let t = [any_char(), any_char(), any_char()];
    let n: usize = kani::any();
    kani::assume(n <= p.len());
    let m: usize = kani::any();
    kani::assume(m <= t.len());
    let pattern = p.get(..n).unwrap_or(&[]);
    let text = t.get(..m).unwrap_or(&[]);
    assert!(glob_match_chars(pattern, text) == chars_eq(pattern, text));
}

/// A lone `*` matches every text, wildcards in the text included.
#[kani::proof]
#[kani::unwind(5)]
fn star_matches_everything() {
    let t = [any_char(), any_char(), any_char()];
    let m: usize = kani::any();
    kani::assume(m <= t.len());
    assert!(glob_match_chars(&['*'], t.get(..m).unwrap_or(&[])));
}

/// Deny always wins: a requirement on a denied capability is violated,
/// whatever the grants are.
#[kani::proof]
#[kani::unwind(4)]
fn deny_wins() {
    let req = any_req();
    let grants = [any_grant(), any_grant()];
    let denies = [any_cap()];
    if denies.contains(&req.capability) {
        assert!(violates_ref(&denies, &grants, req));
    }
}

/// `allowed` is exactly "every requirement granted and none denied".
#[kani::proof]
#[kani::unwind(4)]
fn allowed_iff_granted_and_not_denied() {
    let reqs = [any_req(), any_req()];
    let grants = [any_grant()];
    let denies = [any_cap()];
    let expected = reqs
        .iter()
        .all(|r| !denies.contains(&r.capability) && grants.iter().any(|g| covers_ref(*g, *r)));
    assert!(allowed_ref(&denies, &grants, &reqs) == expected);
}

/// Adding a grant never turns an allowance into a denial.
#[kani::proof]
#[kani::unwind(4)]
fn grants_are_monotone() {
    let reqs = [any_req()];
    let g1 = any_grant();
    let g2 = any_grant();
    let before = allowed_ref(&[], &[g1], &reqs);
    let after = allowed_ref(&[], &[g1, g2], &reqs);
    assert!(!before || after);
}

/// A scoped grant never covers an unscoped requirement; an unscoped grant
/// covers every requirement on its capability.
#[kani::proof]
#[kani::unwind(4)]
fn scope_rules() {
    let cap = any_cap();
    let g = any_grant();
    let unscoped_req = ReqRef {
        capability: cap,
        scope: None,
    };
    if g.capability == cap && g.scope.is_some() {
        assert!(!covers_ref(g, unscoped_req));
    }
    let req = any_req();
    if req.capability == cap {
        let unscoped_grant = GrantRef {
            capability: cap,
            scope: None,
        };
        assert!(covers_ref(unscoped_grant, req));
    }
}

/// The tier rule: only `proved` can verify; a Lean-only TCB verifies without
/// approval; a foreign TCB verifies only with an approved backend.
///
/// The bound covers the builtin `memcmp` loop: the rule compares backend and
/// TCB names, and the longest of them (`verus+z3`) is eight bytes.
#[kani::proof]
#[kani::unwind(12)]
fn tier_rule() {
    let ri: u8 = kani::any();
    kani::assume(ri < 4);
    let result = match ri {
        0 => Result::Proved,
        1 => Result::Tested,
        2 => Result::Scanned,
        _ => Result::Failed,
    };
    let foreign: bool = kani::any();
    let tcb: [&str; 1] = if foreign { ["z3"] } else { ["lean4"] };
    let approved: bool = kani::any();
    let backend = "verus+z3";
    let approved_backends: &[&str] = if approved { &[backend] } else { &[] };

    let t = tier_of_ref(result, &tcb, backend, approved_backends);
    if result != Result::Proved {
        assert!(t == Tier::NotVerified);
    } else if !foreign {
        assert!(t == Tier::Verified);
    } else if approved {
        assert!(t == Tier::VerifiedWithTcb);
    } else {
        assert!(t == Tier::NotVerified);
    }
}
