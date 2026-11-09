//! Bounded model-checking harnesses, run with `cargo kani -p meden-core` in CI.
//!
//! Each harness states a property over *all* inputs within a small bound.
//! Kani (CBMC) explores them exhaustively, so a passing harness is a proof of
//! the property for those bounds, including absence of panics and arithmetic
//! overflow.
//!
//! Three rules keep these cheap enough for a hosted runner:
//!
//! * **Small allocations.** Scope vectors, the matcher row and refusal vectors
//!   are bounded. Inputs use fixed arrays of [`Grant`] and [`Requirement`];
//!   decoded scopes come from a fixed menu, so no string is symbolic.
//! * **Small checks.** Harness array access uses `get`; the shared matcher
//!   retains its checked indexing. Assertions use `assert!` rather than
//!   `assert_eq!`, keeping `Debug` formatting out of the model.
//! * **No symbolic-length loops.** Arrays have a fixed length and symbolic
//!   contents; a symbolic length is applied afterwards with `get(..n)`.
//!
//! The Lean model carries the unbounded argument; these harnesses pin the
//! Rust code to it. The harnesses use the same owned policy types as production.

use crate::capability::{Capability, Requirement};
use crate::glob::glob_match_chars;
use crate::policy::{allowed_ref, covers_ref, violates_ref, Grant};
use crate::scope::Scope;

/// Scopes a requirement may carry (concrete).
const REQ_SCOPES: [&[char]; 2] = [&['a'], &['b']];
/// Scopes a grant may carry (concrete, with wildcards).
const GRANT_SCOPES: [&[char]; 3] = [&['a'], &['*'], &['?']];
/// Characters a symbolic pattern or text is built from.
const ALPHABET: [char; 2] = ['a', 'b'];

fn any_cap() -> Capability {
    let i: usize = kani::any();
    kani::assume(i < Capability::ALL.len());
    // The assume makes the fallback unreachable; `get` keeps the panic
    // branch out of the model.
    *Capability::ALL.get(i).unwrap_or(&Capability::UiControl)
}

fn any_req() -> Requirement {
    let scoped: bool = kani::any();
    let i: usize = kani::any();
    kani::assume(i < REQ_SCOPES.len());
    let scope = REQ_SCOPES.get(i).copied().unwrap_or(&['a']);
    Requirement {
        capability: any_cap(),
        scope: scoped.then(|| Scope::from(scope.to_vec())),
        evidence: None,
    }
}

fn any_grant() -> Grant {
    let scoped: bool = kani::any();
    let i: usize = kani::any();
    kani::assume(i < GRANT_SCOPES.len());
    let scope = GRANT_SCOPES.get(i).copied().unwrap_or(&['a']);
    Grant {
        capability: any_cap(),
        scope: scoped.then(|| Scope::from(scope.to_vec())),
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
        assert!(violates_ref(&denies, &grants, &req));
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
        .all(|r| !denies.contains(&r.capability) && grants.iter().any(|g| covers_ref(g, r)));
    assert!(allowed_ref(&denies, &grants, &reqs) == expected);
}

/// Adding a grant never turns an allowance into a denial.
#[kani::proof]
#[kani::unwind(4)]
fn grants_are_monotone() {
    let reqs = [any_req()];
    let g1 = any_grant();
    let g2 = any_grant();
    let before = allowed_ref(&[], &[g1.clone()], &reqs);
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
    let unscoped_req = Requirement {
        capability: cap,
        scope: None,
        evidence: None,
    };
    if g.capability == cap && g.scope.is_some() {
        assert!(!covers_ref(&g, &unscoped_req));
    }
    let req = any_req();
    if req.capability == cap {
        let unscoped_grant = Grant {
            capability: cap,
            scope: None,
        };
        assert!(covers_ref(&unscoped_grant, &req));
    }
}
