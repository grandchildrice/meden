//! Host-only second-model selection. Each processor keeps its own immutable grant.
use anyhow::Result;
use meden_llm::Gateway;
use meden_runtime::workspace::SessionId;
use std::sync::Arc;

pub(crate) use meden_app::model::Binding;
pub(crate) fn binding(gateway: &Gateway) -> Binding {
    meden_app::model::binding(gateway)
}
pub(crate) fn prepare(
    session: &SessionId,
    primary: &Gateway,
    model: Option<&str>,
) -> Result<Option<Arc<Gateway>>> {
    let Some(model) = model else { return Ok(None) };
    anyhow::ensure!(
        !primary.is_local(),
        "Separate judge currently requires an explicitly reviewed HTTP connection"
    );
    anyhow::ensure!(
        !model.trim().is_empty() && model.len() <= 1000 && model != primary.provider().model,
        "Judge model must be nonempty and distinct from the primary model"
    );
    let mut provider = primary.provider().clone();
    model.clone_into(&mut provider.model);
    Ok(Some(Arc::new(Gateway::prepare(session, provider)?)))
}
pub(crate) fn authorized(judge: Option<&Gateway>, session: &SessionId) -> bool {
    judge.is_none_or(|judge| judge.is_authorized(session))
}
