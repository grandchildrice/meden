//! Plain observed text uses the same assessment service without an elaborator.
use crate::{analysis, identity::snapshots::GraphRevision, semantic};
use anyhow::Result;
#[derive(serde::Serialize)]
pub enum Context {}
impl semantic::Observation for analysis::sources::Snapshot {
    type Context = Context;
    fn graph_revision(&self) -> Option<&GraphRevision> {
        None
    }
    fn context(&self, _: &semantic::Target) -> Result<Option<Context>> {
        Ok(None)
    }
}
impl analysis::Observation for analysis::sources::Snapshot {
    fn check(&self, _: &crate::adapters::Context<'_>) -> Result<analysis::Freshness> {
        Ok(analysis::Freshness::Current)
    }
}
