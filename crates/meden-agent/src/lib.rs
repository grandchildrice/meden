//! Native conversation engine and tool interfaces.
//! Capability checks constrain declared tool effects; they do not establish
//! the outbound confidentiality requirement in docs/design.md.
#![forbid(unsafe_code)]
mod chat_json;
pub mod conversation;
pub mod tool;
pub use conversation::{ChatModel, ModelError};
pub use tool::Registry;
