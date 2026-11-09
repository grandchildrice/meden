//! Conformance of the Rust implementation with the Lean model.
//!
//! Generates policy/glob cases and processor traces with a fixed-seed PRNG, runs
//! the compiled Lean oracle (`lean/.lake/build/bin/meden-spec`, or the path
//! in `MEDEN_SPEC_BIN`) on them, and compares with `meden-core`.
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

use meden_core::glob::glob_match;
use meden_core::processor::{
    Binding, Dispatch, HttpParameters, LocalParameters, ProcessorOrigin as Origin,
    ProcessorPhase as Phase, State, APPROVAL_LIFETIME_MILLIS,
};
use meden_core::{check, Capability, Grant, Policy, Requirement};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
struct Input {
    policy_cases: Vec<PolicyCase>,
    glob_cases: Vec<GlobCase>,
    processor_cases: Vec<ProcessorCase>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ProcessorBinding {
    is_local: bool,
    engine: Vec<u8>,
    weights: Vec<u8>,
    provider: Vec<u8>,
    endpoint: Vec<u8>,
    model: Vec<u8>,
    credential: Option<Vec<u8>>,
}
impl ProcessorBinding {
    fn native(&self) -> Binding {
        if self.is_local {
            Binding::Local(LocalParameters::new(
                self.engine.clone(),
                self.weights.clone(),
            ))
        } else {
            Binding::Http(HttpParameters::new(
                self.provider.clone(),
                self.endpoint.clone(),
                self.model.clone(),
                self.credential.clone(),
            ))
        }
    }
    fn observe(binding: &Binding) -> Self {
        let mut out = Self {
            is_local: false,
            engine: vec![],
            weights: vec![],
            provider: vec![],
            endpoint: vec![],
            model: vec![],
            credential: None,
        };
        match binding {
            Binding::Local(p) => {
                out.is_local = true;
                out.engine = p.engine().to_vec();
                out.weights = p.weights().to_vec();
            }
            Binding::Http(p) => {
                out.provider = p.provider().to_vec();
                out.endpoint = p.endpoint().to_vec();
                out.model = p.model().to_vec();
                out.credential = p.credential().map(<[u8]>::to_vec);
            }
        }
        out
    }
}
#[derive(Serialize)]
struct ProcessorAction {
    kind: &'static str,
    host: bool,
    now: u64,
    session: Vec<u8>,
}
#[derive(Serialize)]
struct ProcessorCase {
    session: Vec<u8>,
    binding: ProcessorBinding,
    actions: Vec<ProcessorAction>,
}
#[derive(Debug, PartialEq, Eq, Deserialize)]
struct ProcessorObservation {
    accepted: bool,
    effect: Option<ProcessorBinding>,
}
#[derive(Debug, PartialEq, Eq, Deserialize)]
struct ProcessorResult {
    steps: Vec<ProcessorObservation>,
    binding: ProcessorBinding,
    phase: String,
    expires: u64,
}
fn run_processor(case: &ProcessorCase) -> ProcessorResult {
    let mut state = State::new(case.session.clone(), case.binding.native());
    let steps = case
        .actions
        .iter()
        .map(|a| {
            let origin = if a.host { Origin::Host } else { Origin::Model };
            match a.kind {
                "approve" => ProcessorObservation {
                    accepted: state.approve(origin, a.now),
                    effect: None,
                },
                "revoke" => ProcessorObservation {
                    accepted: state.revoke(origin),
                    effect: None,
                },
                _ => {
                    let effect = state.dispatch(&a.session, a.now).map(|d| {
                        let binding = match d {
                            Dispatch::Local(c) => Binding::Local(c.into_parameters()),
                            Dispatch::Http(c) => Binding::Http(c.into_parameters()),
                        };
                        ProcessorBinding::observe(&binding)
                    });
                    ProcessorObservation {
                        accepted: effect.is_some(),
                        effect,
                    }
                }
            }
        })
        .collect();
    ProcessorResult {
        steps,
        binding: ProcessorBinding::observe(state.binding()),
        phase: match state.phase() {
            Phase::Pending => "pending",
            Phase::Granted => "granted",
            Phase::Revoked => "revoked",
        }
        .into(),
        expires: state.expires(),
    }
}
#[derive(Deserialize)]
struct Output {
    policy: Vec<bool>,
    glob: Vec<bool>,
    processor: Vec<ProcessorResult>,
}

fn oracle_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("MEDEN_SPEC_BIN") {
        return Some(PathBuf::from(p));
    }
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../lean/.lake/build/bin/meden-spec");
    root.is_file().then_some(root)
}

#[test]
fn rust_agrees_with_lean_oracle() {
    let Some(bin) = oracle_path() else {
        eprintln!("conformance: Lean oracle not built; skipping (set MEDEN_SPEC_BIN or run `lake build` in lean/)");
        return;
    };
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let scope_alpha = ['a', 'b', '/', '.'];
    let glob_alpha = ['a', 'b', '/', '.', '*', '?'];
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
    let processor_cases = (0..4000)
        .map(|i| {
            let binary = format!("日本語🙂\0-{}", rng.next()).into_bytes();
            let binding = if i % 2 == 0 {
                Binding::Local(LocalParameters::new(binary.clone(), vec![0, 255, 1]))
            } else {
                Binding::Http(HttpParameters::new(
                    binary.clone(),
                    vec![0, 255, 2],
                    binary,
                    match i % 3 {
                        0 => None,
                        1 => Some(vec![]),
                        _ => Some(vec![0, 255, 3]),
                    },
                ))
            };
            let times = [
                0,
                1,
                APPROVAL_LIFETIME_MILLIS - 1,
                APPROVAL_LIFETIME_MILLIS,
                APPROVAL_LIFETIME_MILLIS + 1,
                u64::MAX - APPROVAL_LIFETIME_MILLIS,
                u64::MAX - APPROVAL_LIFETIME_MILLIS + 1,
                u64::MAX,
            ];
            let mut actions = vec![];
            // Half the traces begin with trusted approval, ensuring successful
            // dispatches and later expiry/revocation are exercised as well.
            if i % 4 < 2 {
                actions.push(ProcessorAction {
                    kind: "approve",
                    host: true,
                    now: 0,
                    session: vec![],
                });
            }
            for _ in 0..16 {
                actions.push(ProcessorAction {
                    kind: match rng.below(5) {
                        0 => "approve",
                        1 => "revoke",
                        _ => "dispatch",
                    },
                    host: rng.chance(1, 2),
                    now: if rng.chance(3, 4) {
                        times[rng.below(times.len())]
                    } else {
                        rng.next()
                    },
                    session: if rng.chance(3, 4) {
                        b"session-a".to_vec()
                    } else {
                        b"foreign".to_vec()
                    },
                });
            }
            ProcessorCase {
                session: b"session-a".to_vec(),
                binding: ProcessorBinding::observe(&binding),
                actions,
            }
        })
        .collect();
    let input = Input {
        policy_cases,
        glob_cases,
        processor_cases,
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
    assert_eq!(output.processor.len(), input.processor_cases.len());
    for (i, (case, expected)) in input
        .processor_cases
        .iter()
        .zip(output.processor)
        .enumerate()
    {
        assert_eq!(
            run_processor(case),
            expected,
            "processor trace {i}: {}",
            json!(case)
        );
    }
}
