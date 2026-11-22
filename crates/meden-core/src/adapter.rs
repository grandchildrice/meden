//! Backend-neutral observation interface. Adapters produce untrusted artifacts;
//! this interface grants no verification, filesystem or delivery authority.

/// Explicitly named, immutable input to a host-selected adapter.
pub struct Input {
    /// Media type or format identifier interpreted by this adapter.
    pub format: String,
    /// Original encoded input; the core never interprets backend syntax.
    pub bytes: Vec<u8>,
}

/// Prepared observation, separate from the evidence admission kernel.
pub struct Artifact {
    /// Producer identity, including its implementation/resource version.
    pub producer: String,
    /// Output format identifier.
    pub format: String,
    /// Complete encoded observation, never a proof authority bit.
    pub bytes: Vec<u8>,
}

/// Compiler, meaning renderer or search adapter selected by trusted host code.
/// The host supplies its context (isolation, cancellation and frozen resources).
pub trait Adapter<Context> {
    /// Backend-specific preparation or execution failure.
    type Error;
    /// Consume one explicit request and return the complete observation.
    fn run(&self, context: &Context, input: Input) -> Result<Artifact, Self::Error>;
}
