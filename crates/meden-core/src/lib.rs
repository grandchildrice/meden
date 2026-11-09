//! Core of the Meden gate: capability vocabulary, scope matching, policies,
//! and decisions.
//!
//! This crate is the executable counterpart of the Lean specification in
//! `lean/MedenSpec`. The two are kept in step by the conformance tests in
//! `crates/meden-cli/tests/conformance.rs`, which feed the same random cases
//! to this crate and to the compiled Lean model and compare the answers.
//! Bounded properties of this crate are also model-checked with Kani (see
//! `src/kani_harness.rs`).
//!
//! The crate forbids `unsafe` code and performs no I/O. The processor,
//! export-review and evidence kernels have extraction-based refinement proofs; policy
//! and glob conformance/model checks have their own narrower scope. Lints
//! are development checks, not a mathematical proof of whole-crate behavior.

#![forbid(unsafe_code)]

pub mod capability;
pub mod evidence;
pub mod export_control;
pub mod glob;
pub mod policy;
pub mod processor;

#[cfg(kani)]
mod kani_harness;

pub use capability::{Capability, Requirement};
pub use policy::{
    allowed, allowed_ref, check, covers_ref, violates, violates_ref, Decision, Grant, GrantRef,
    Policy, ReqRef,
};
