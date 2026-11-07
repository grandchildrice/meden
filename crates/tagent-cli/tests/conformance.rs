//! Conformance of the Rust implementation with the Lean model.
//!
//! Generates random policy, glob and tier cases with a fixed-seed PRNG, runs
//! the compiled Lean oracle (`lean/.lake/build/bin/tagent-spec`, or the path
//! in `TAGENT_SPEC_BIN`) on them, and compares with `tagent-core`.
//!
//! The test is skipped (with a notice) when the oracle is not built, so
//! `cargo test` stays usable without a Lean toolchain; CI always builds it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::struct_field_names,
    clippy::too_many_lines,
    clippy::panic
)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tagent_core::attestation::{tier_of, Result as VResult, Tier};
use tagent_core::glob::glob_match;
use tagent_core::{check, Capability, Grant, Policy, Requirement};

/// Deterministic xorshift64* generator so failures reproduce.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
    fn string(&mut self, alphabet: &[char], max_len: usize) -> String {
        let len = self.below(max_len + 1);
        (0..len)
            .map(|_| alphabet[self.below(alphabet.len())])
            .collect()
    }
    fn cap(&mut self) -> Capability {
        Capability::ALL[self.below(Capability::ALL.len())]
    }
}

/// Policy sent to the oracle as explicit objects (the Rust `Policy`
/// serializes grants as labels, which the oracle does not parse).
#[derive(Serialize)]
struct PolicyCase {
    policy: serde_json::Value,
    requirements: Vec<Requirement>,
    #[serde(skip)]
    rust_policy: Policy,
}

fn policy_json(p: &Policy) -> serde_json::Value {
    json!({
        "name": p.name,
        "grants": p.grants.iter().map(|g| json!({"capability": g.capability, "scope": g.scope})).collect::<Vec<_>>(),
        "denies": p.denies,
    })
}

#[derive(Serialize)]
struct GlobCase {
    pattern: String,
    text: String,
}

#[derive(Serialize)]
struct TierCase {
    result: VResult,
    tcb: Vec<String>,
    backend: String,
    approved: Vec<String>,
}

#[derive(Serialize)]
struct Input {
    policy_cases: Vec<PolicyCase>,
    glob_cases: Vec<GlobCase>,
    tier_cases: Vec<TierCase>,
}

#[derive(Deserialize)]
struct Output {
    policy: Vec<bool>,
    glob: Vec<bool>,
    tier: Vec<String>,
}

fn oracle_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TAGENT_SPEC_BIN") {
        return Some(PathBuf::from(p));
    }
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../lean/.lake/build/bin/tagent-spec");
    root.is_file().then_some(root)
}

fn tier_name(t: Tier) -> &'static str {
    match t {
        Tier::Verified => "verified",
        Tier::VerifiedWithTcb => "verified-with-tcb",
        Tier::NotVerified => "not-verified",
    }
}

#[test]
fn rust_agrees_with_lean_oracle() {
    let Some(bin) = oracle_path() else {
        eprintln!("conformance: Lean oracle not built; skipping (set TAGENT_SPEC_BIN or run `lake build` in lean/)");
        return;
    };
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let scope_alpha = ['a', 'b', '/', '.'];
    let glob_alpha = ['a', 'b', '/', '.', '*', '?'];
    let tcb_pool = [
        "lean4", "lake", "aeneas", "charon", "rustc", "z3", "verus", "pytest",
    ];
    let backends = ["lean4", "lean4+aeneas", "verus+z3", "pytest"];
    let results = [
        VResult::Proved,
        VResult::Tested,
        VResult::Scanned,
        VResult::Failed,
    ];

    let policy_cases: Vec<PolicyCase> = (0..400)
        .map(|_| {
            let grants = (0..rng.below(4))
                .map(|_| {
                    let scope = rng.chance(1, 3).then(|| rng.string(&glob_alpha, 5));
                    Grant::new(rng.cap(), scope.as_deref())
                })
                .collect();
            let denies = (0..rng.below(3)).map(|_| rng.cap()).collect();
            let requirements = (0..rng.below(4))
                .map(|_| {
                    let scope = rng.chance(2, 3).then(|| rng.string(&scope_alpha, 5));
                    Requirement::new(rng.cap(), scope.as_deref())
                })
                .collect();
            let rust_policy = Policy {
                name: "p".into(),
                grants,
                denies,
            };
            PolicyCase {
                policy: policy_json(&rust_policy),
                requirements,
                rust_policy,
            }
        })
        .collect();
    let glob_cases: Vec<GlobCase> = (0..600)
        .map(|_| GlobCase {
            pattern: rng.string(&glob_alpha, 7),
            text: rng.string(&scope_alpha, 7),
        })
        .collect();
    let tier_cases: Vec<TierCase> = (0..200)
        .map(|_| {
            let tcb = (0..rng.below(4))
                .map(|_| tcb_pool[rng.below(tcb_pool.len())].to_owned())
                .collect();
            let backend = backends[rng.below(backends.len())].to_owned();
            let approved = (0..rng.below(3))
                .map(|_| backends[rng.below(backends.len())].to_owned())
                .collect();
            TierCase {
                result: results[rng.below(4)],
                tcb,
                backend,
                approved,
            }
        })
        .collect();

    let input = Input {
        policy_cases,
        glob_cases,
        tier_cases,
    };
    let mut child = Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("spawning {}: {e}", bin.display()));
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&input).unwrap().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "oracle failed: {}", out.status);
    let output: Output = serde_json::from_slice(&out.stdout).expect("oracle output");

    assert_eq!(output.policy.len(), input.policy_cases.len());
    for (i, c) in input.policy_cases.iter().enumerate() {
        let rust = check(&c.requirements, &c.rust_policy).allowed;
        assert_eq!(rust, output.policy[i], "policy case {i}: {}", json!(c));
    }
    assert_eq!(output.glob.len(), input.glob_cases.len());
    for (i, c) in input.glob_cases.iter().enumerate() {
        assert_eq!(
            glob_match(&c.pattern, &c.text),
            output.glob[i],
            "glob case {i}: {:?} vs {:?}",
            c.pattern,
            c.text
        );
    }
    assert_eq!(output.tier.len(), input.tier_cases.len());
    for (i, c) in input.tier_cases.iter().enumerate() {
        let rust = tier_name(tier_of(c.result, &c.tcb, &c.backend, &c.approved));
        assert_eq!(rust, output.tier[i], "tier case {i}: {}", json!(c));
    }
}
