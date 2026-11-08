//! Local project checking. Candidate generation has no checker authority.

mod bundle;
mod failure;
mod ledger;
mod source;

pub use bundle::Bundle;
pub use failure::{failed_worker, verification_stops, Failure, FailureKind};
pub use ledger::{ledger_checker_id, verify_ledger, Artifact, Checked, Report, LEDGER_CONTRACT};
pub use source::validate_ledger_source;
