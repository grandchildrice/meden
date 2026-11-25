//! Native finite-model registration. Typed role implementations remain outside
//! the shared viewer, workflow and authority kernel.
mod cache;
mod compiled;
pub(super) mod navigation;
mod positions;
mod render;
mod search;
mod snapshot;
use anyhow::Result;
pub use snapshot::Snapshot;
use std::path::PathBuf;
pub struct Backend {
    worker: PathBuf,
}
impl Backend {
    /// The executable comes from trusted host composition, never a source file.
    pub fn new(worker: PathBuf) -> Self {
        Self { worker }
    }
}
impl crate::analysis::prepared::Backend for Backend {
    type Output = Snapshot;
    fn read(
        &self,
        context: &crate::adapters::Context<'_>,
        paths: &[String],
    ) -> Result<(Snapshot, bool)> {
        cache::read(context, paths, &self.worker)
    }
    fn details(&self, output: &Snapshot) -> Vec<String> {
        vec![output.compiled.contract.scope()]
    }
}

fn digest(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    format!("{:x}", sha2::Sha256::digest(bytes))
}
