//! Host-owned connection choices. Each selection prepares a new immutable,
//! initially unapproved gateway; a prior gateway is revoked, never mutated.
use super::lock;
use anyhow::{Context, Result};
use meden_llm::{Gateway, Provider};
use meden_runtime::workspace::SessionId;
use std::sync::{Arc, Mutex};

use meden_app::view::session::Choice;
pub(super) struct Connections {
    providers: Vec<Provider>,
    pub gateway: Option<Arc<Gateway>>,
    pub judge: Option<Arc<Gateway>>,
    pub revision: u64,
    judge_model: Option<String>,
}

impl Connections {
    pub fn new(args: &crate::connection::ModelArgs, session: &SessionId) -> Result<Self> {
        let selected = if args.interactive() {
            None
        } else {
            Some(args.connection()?)
        };
        let mut providers = meden_llm::detect();
        if let Some(selected) = &selected {
            providers.retain(|provider| provider.id != selected.id);
            providers.insert(0, selected.clone());
        }
        let mut connections = Self {
            providers,
            gateway: None,
            judge: None,
            revision: 0,
            judge_model: args.judge_model.clone(),
        };
        if let Some(provider) = selected {
            connections.select(session, &provider.id, &provider.model)?;
        }
        Ok(connections)
    }

    pub fn choices(&self) -> Vec<Choice> {
        self.providers
            .iter()
            .map(|provider| Choice {
                id: provider.id.clone(),
                label: provider.label.clone(),
                model: provider.model.clone(),
                note: provider.note.clone(),
            })
            .collect()
    }

    pub fn refresh(&mut self) {
        let mut providers = meden_llm::detect();
        if let Some(gateway) = &self.gateway {
            if !providers
                .iter()
                .any(|provider| provider.id == gateway.provider().id)
            {
                providers.insert(0, gateway.provider().clone());
            }
        }
        self.providers = providers;
    }

    pub fn models(&self, id: &str) -> Result<Vec<String>> {
        let provider = self
            .providers
            .iter()
            .find(|provider| provider.id == id)
            .context("Choose an existing host connection")?;
        meden_llm::provider::models(provider)
            .context("Model list unavailable. The configured model is still selectable; enter another model ID if needed.")
    }

    pub fn select(&mut self, session: &SessionId, id: &str, model: &str) -> Result<()> {
        anyhow::ensure!(
            !model.trim().is_empty() && model.len() <= 1024 && !model.chars().any(char::is_control),
            "Supply a valid model ID"
        );
        let mut provider = self
            .providers
            .iter()
            .find(|provider| provider.id == id)
            .cloned()
            .context("Choose an existing host connection")?;
        model.clone_into(&mut provider.model);
        let gateway = Arc::new(Gateway::prepare(session, provider)?);
        let judge = crate::judge::prepare(session, &gateway, self.judge_model.as_deref())?;
        let revision = self
            .revision
            .checked_add(1)
            .context("Connection selection exhausted")?;
        if let Some(previous) = &self.gateway {
            previous.revoke_from_host()?;
        }
        if let Some(previous) = &self.judge {
            previous.revoke_from_host()?;
        }
        self.gateway = Some(gateway);
        self.judge = judge;
        self.revision = revision;
        Ok(())
    }

    pub fn display(&self, view: &mut super::View) {
        view.connections = self.choices();
        view.connection_revision = std::num::NonZeroU64::new(self.revision).map(Into::into);
        view.provider = self
            .gateway
            .as_ref()
            .map(|gateway| gateway.provider().id.clone());
        view.endpoint = self
            .gateway
            .as_ref()
            .map(|gateway| gateway.endpoint().to_owned())
            .unwrap_or_default();
        view.model = self
            .gateway
            .as_ref()
            .map(|gateway| gateway.provider().model.clone())
            .unwrap_or_default();
        view.judge = self.judge.as_deref().map(crate::judge::binding);
        view.authorized = false;
        view.approval_available = self.gateway.is_some();
    }
}

pub(super) fn approve(
    connections: &Mutex<Connections>,
    session: &SessionId,
    revision: u64,
    endpoint: &str,
    model: &str,
    judge: Option<&crate::judge::Binding>,
) -> Result<()> {
    let connections = lock(connections);
    let gateway = connections
        .gateway
        .as_ref()
        .context("Choose a provider and model first")?;
    anyhow::ensure!(
        revision == connections.revision
            && endpoint == gateway.endpoint()
            && model == gateway.provider().model
            && judge
                == connections
                    .judge
                    .as_deref()
                    .map(crate::judge::binding)
                    .as_ref(),
        "processor review target changed"
    );
    anyhow::ensure!(
        !gateway.is_authorized(session),
        "Processor already approved"
    );
    gateway.approve_from_host()?;
    if let Some(judge) = &connections.judge {
        judge.approve_from_host()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_counter_is_exact_at_u64_maximum_and_overflow_cannot_switch_the_gateway() -> Result<()>
    {
        let provider = Provider {
            id: "host".into(),
            label: "Host".into(),
            model: "original".into(),
            base_url: "https://example.invalid/v1".into(),
            protocol: meden_llm::Protocol::ChatCompletions,
            api_key: None,
            note: "counter boundary fixture; no inference".into(),
        };
        // Seed only the host counter at its boundary; selection and approval
        // still use the real original prepared Gateway and its terminal grants.
        let connections = Mutex::new(Connections {
            providers: vec![provider],
            gateway: None,
            judge: None,
            revision: u64::MAX - 1,
            judge_model: None,
        });
        lock(&connections).select(
            &SessionId::from_host_bytes(b"workspace"),
            "host",
            "original",
        )?;
        let gateway = lock(&connections)
            .gateway
            .clone()
            .context("prepared original gateway")?;
        assert!(!gateway.is_authorized(&SessionId::from_host_bytes(b"workspace")));
        for (counter, allowed) in [(u64::MAX - 1, false), (u64::MAX, true)] {
            let command: super::super::Command = serde_json::from_value(serde_json::json!({
                "type":"approve_processor", "connection_revision":counter.to_string(),
                "endpoint":gateway.endpoint(), "model":gateway.provider().model,
                "judge":null
            }))?;
            let super::super::Command::ApproveProcessor {
                connection_revision,
                endpoint,
                model,
                judge,
            } = command
            else {
                anyhow::bail!("original command tag changed");
            };
            assert_eq!(connection_revision.value(), counter);
            assert_eq!(
                approve(
                    &connections,
                    &SessionId::from_host_bytes(b"workspace"),
                    connection_revision.value(),
                    &endpoint,
                    &model,
                    judge.as_ref()
                )
                .is_ok(),
                allowed
            );
            assert_eq!(
                gateway.is_authorized(&SessionId::from_host_bytes(b"workspace")),
                allowed
            );
        }
        assert!(lock(&connections)
            .select(
                &SessionId::from_host_bytes(b"workspace"),
                "host",
                "replacement"
            )
            .is_err());
        let retained = lock(&connections)
            .gateway
            .clone()
            .context("retained original gateway")?;
        assert!(Arc::ptr_eq(&gateway, &retained));
        assert_eq!(retained.provider().model, "original");
        assert!(retained.is_authorized(&SessionId::from_host_bytes(b"workspace")));
        Ok(())
    }

    #[test]
    fn changing_models_revokes_the_frozen_previous_gateway_and_requires_exact_fresh_review(
    ) -> Result<()> {
        let provider = Provider {
            id: "host".into(),
            label: "Host".into(),
            model: "first".into(),
            base_url: "https://example.invalid/v1".into(),
            protocol: meden_llm::Protocol::ChatCompletions,
            api_key: Some("HOST_SECRET".into()),
            note: "fixture".into(),
        };
        let connections = Mutex::new(Connections {
            providers: vec![provider],
            gateway: None,
            judge: None,
            revision: 0,
            judge_model: None,
        });
        lock(&connections).select(&SessionId::from_host_bytes(b"workspace"), "host", "first")?;
        approve(
            &connections,
            &SessionId::from_host_bytes(b"workspace"),
            1,
            "https://example.invalid/v1/chat/completions",
            "first",
            None,
        )?;
        let previous = lock(&connections)
            .gateway
            .clone()
            .context("first gateway")?;
        assert!(previous.is_authorized(&SessionId::from_host_bytes(b"workspace")));
        lock(&connections).select(&SessionId::from_host_bytes(b"workspace"), "host", "second")?;
        assert_eq!(previous.provider().model, "first");
        assert!(!previous.is_authorized(&SessionId::from_host_bytes(b"workspace")));
        assert!(previous.approve_from_host().is_err());
        assert!(approve(
            &connections,
            &SessionId::from_host_bytes(b"workspace"),
            1,
            "https://example.invalid/v1/chat/completions",
            "second",
            None
        )
        .is_err());
        assert!(approve(
            &connections,
            &SessionId::from_host_bytes(b"workspace"),
            2,
            "https://example.invalid/v1/chat/completions",
            "first",
            None
        )
        .is_err());
        approve(
            &connections,
            &SessionId::from_host_bytes(b"workspace"),
            2,
            "https://example.invalid/v1/chat/completions",
            "second",
            None,
        )?;
        assert!(!previous.is_authorized(&SessionId::from_host_bytes(b"workspace")));
        assert!(!serde_json::to_string(&lock(&connections).choices())?.contains("HOST_SECRET"));
        Ok(())
    }
}
