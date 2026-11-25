//! Lean adapters compose their decoded data and independent application roles.
pub mod cache;
pub mod collect;
mod compiler;
pub(super) mod definition;
pub mod layout;
pub mod pipeline;
pub mod repair;
pub mod saved;
pub mod search;
pub mod semantic;
pub(crate) mod snapshot;
mod view;
pub use compiler::Lean;
pub use snapshot::Snapshot;
