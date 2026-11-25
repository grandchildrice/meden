//! Plain text retains its original line anchor and has no compiler dependencies.
use crate::{
    analysis::{repair, sources::Snapshot},
    semantic::Input,
};
use anyhow::Result;
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Context;
impl repair::Context for Context {
    fn check(&self, _: &crate::adapters::Context<'_>) -> Result<()> {
        Ok(())
    }
    fn paths(&self) -> Option<&[String]> {
        None
    }
    fn relocatable(&self) -> bool {
        false
    }
}
impl repair::Observation for Snapshot {
    type Repair = Context;
    fn repair_context(&self) -> Result<Context> {
        Ok(Context)
    }
    fn refreshed_input(&self, original: &Input) -> Result<Input> {
        let mut input = original.clone();
        input.source_revision.clone_from(&self.revision);
        input.graph_revision = None;
        Ok(input)
    }
}
