//! Shared use cases and typed integration ports, independent of presentation
//! and model transports. Authority remains in the consuming core controllers.
#![forbid(unsafe_code)]

// Existing schema and service fields are being moved out of the binary, not
// declared a stable SDK. New ports document their own invariants explicitly.
#[allow(missing_docs)]
pub mod adapters;
#[allow(missing_docs)]
pub mod analysis;
#[allow(missing_docs)]
pub mod atlas;
pub mod compiler;
#[allow(missing_docs)]
pub mod explanation;
pub mod meaning;
pub mod model;
#[allow(missing_docs)]
pub mod project;
#[allow(missing_docs)]
pub mod semantic;
#[allow(missing_docs)]
pub mod shell;
#[allow(missing_docs)]
pub mod storage;
pub mod verification;
#[allow(missing_docs)]
pub mod verifier;
/// Backend-neutral materialized source/graph observations for presentation.
pub mod view;
#[allow(missing_docs)]
pub mod workspace;

/// Notices for the bundled source adaptations, retained with the implementations.
pub const SOURCE_NOTICES: (&str, &str, &str) = (
    include_str!("atlas/vendor/LICENSE"),
    include_str!("atlas/vendor/MedenPlausible/LICENSE"),
    include_str!("semantic/CLAIMCHECK-LICENSE"),
);
