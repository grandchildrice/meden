//! Local project checking. Candidate generation has no checker authority.

mod bundle;
mod failure;
mod ledger;
mod source;
/// Shared host-turn controller used by interactive and automated sessions.
pub mod turn;

pub use bundle::Bundle;
pub use failure::{failed_worker, verification_stops, Failure, FailureKind};
pub use ledger::{
    ledger_checker_id, prepare_ledger, Artifact, Checked, Inputs, PreparedLedger, Report,
    LEDGER_CONTRACT,
};
pub use source::validate_ledger_source;
