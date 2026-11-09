//! Local project checking. Candidate generation has no checker authority.

mod bundle;
mod contract;
mod failure;
/// Pure Rust transition checker for host-reviewed mathematical contracts.
pub mod kernel;
mod source;
/// Shared host-turn controller used by interactive and automated sessions.
pub mod turn;

pub use bundle::Bundle;
pub use contract::Contract;
pub use failure::{failed_worker, verification_stops, Failure, FailureKind};
pub use source::validate_source;
