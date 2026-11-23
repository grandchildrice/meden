//! Deterministic meaning-rendering port. Each implementation owns its decoded
//! formal representation; no universal prover AST or executable payload is used.
use anyhow::Result;
use meden_runtime::process::Control;

/// A completed presentation observation, distinct from a semantic match/proof.
pub struct Reading {
    /// Bounded realized sentence for one exact input expression.
    pub sentence: String,
    /// Selected renderer/resource identity for presentation provenance.
    pub renderer: String,
}

/// Role contract with a validated backend-owned input type.
pub trait Renderer {
    /// Decoded formal expression accepted by this implementation.
    type Expression;
    /// Render one expression in its prepared isolated worker.
    fn render(&self, control: &Control, expression: Self::Expression) -> Result<Reading>;
}
