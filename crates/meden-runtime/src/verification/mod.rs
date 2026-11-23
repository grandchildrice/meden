//! Local project checking. Candidate generation has no checker authority.

/// Backend-neutral checked files, reports and artifact persistence.
pub mod artifact;
mod bundle;
mod contract;
mod failure;
/// Data-only finite bit-vector checking through an offline Z3 adapter.
pub mod finite;
/// Pure Rust transition checker for host-reviewed mathematical contracts.
pub mod kernel;
mod source;
/// Shared host-turn controller used by interactive and automated sessions.
pub mod turn;

pub use bundle::Bundle;
pub use contract::Contract;
pub use failure::{failed_worker, verification_stops, Failure, FailureKind};
pub use source::validate_source;
