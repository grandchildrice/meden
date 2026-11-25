//! Lean adapters compose their decoded data and independent application roles.
pub mod cache;
mod compiler;
pub(super) mod definition;
pub mod pipeline;
pub mod repair;
pub mod search;
pub mod semantic;
mod snapshot;
mod view;
pub use compiler::Lean;
pub use snapshot::Snapshot;
