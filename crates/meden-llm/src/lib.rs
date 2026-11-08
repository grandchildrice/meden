//! Host-approved HTTP model connections.
#![forbid(unsafe_code)]
mod gateway;
pub mod hermes;
mod openai;
pub mod provider;
pub use gateway::{Gateway, APPROVAL_LIFETIME};
pub use provider::{detect, Provider};
/// Explicit model endpoint base URL.
pub const ENV_BASE_URL: &str = "MEDEN_LLM_BASE_URL";
/// Explicit model identifier.
pub const ENV_MODEL: &str = "MEDEN_LLM_MODEL";
/// Optional credential for the explicit endpoint.
pub const ENV_API_KEY: &str = "MEDEN_LLM_API_KEY";
