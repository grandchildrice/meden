//! Core of the Meden gate: capability vocabulary, scope matching, policies,
//! decisions and attestation tiers.
//!
//! This crate is the executable counterpart of the Lean specification in
//! `lean/MedenSpec`. The two are kept in step by the conformance tests in
//! `crates/meden-cli/tests/conformance.rs`, which feed the same random cases
//! to this crate and to the compiled Lean model and compare the answers.
//! Bounded properties of this crate are also model-checked with Kani (see
//! `src/kani_harness.rs`).
//!
//! The crate has no `unsafe` code, never panics on any input (all slicing
//! and arithmetic is checked by Clippy lints), and performs no I/O.

#![forbid(unsafe_code)]

pub mod attestation;
pub mod capability;
pub mod digest;
pub mod glob;
pub mod policy;

#[cfg(kani)]
mod kani_harness;

pub use attestation::{
    Artifact, Attestation, AttestationError, Proof, Property, Result, Tier, Verifier,
};
pub use capability::{Capability, Requirement};
pub use policy::{
    allowed, allowed_ref, check, covers_ref, violates, violates_ref, Decision, Grant, GrantRef,
    Policy, ReqRef,
};
