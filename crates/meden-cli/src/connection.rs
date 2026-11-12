//! Host-selected connection arguments. Explicit URLs do not inherit credentials
//! from discovery, subscription logins or unrelated environment configuration.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use meden_llm::{Protocol, Provider};

#[derive(Args)]
pub(crate) struct ModelArgs {
    /// Existing HTTP connection ID (including codex/claude logins), or "auto".
    #[arg(long, conflicts_with = "base_url")]
    provider: Option<String>,
    /// Model identifier for the selected connection.
    #[arg(long)]
    model: Option<String>,
    /// OpenAI-compatible API base URL; selects HTTP without --provider.
    #[arg(long, value_name = "URL", requires = "model")]
    base_url: Option<String>,
    /// API key for --base-url. Kept in memory; not saved in the project/history.
    #[arg(
        long,
        value_name = "KEY",
        requires = "base_url",
        conflicts_with = "api_key_env"
    )]
    api_key: Option<String>,
    /// Read the API key from this existing environment variable.
    #[arg(long, value_name = "NAME", requires = "base_url")]
    api_key_env: Option<String>,
    /// Authorize this exact inference URL to receive this session's context
    /// for up to eight hours. Required for run; optional in the TUI.
    #[arg(long, value_name = "URL")]
    pub(crate) approve_processor: Option<String>,
    /// Prepared llama-server executable. Defaults to MEDEN_LOCAL_ENGINE.
    #[arg(long, conflicts_with_all = ["provider", "model", "base_url", "approve_processor"])]
    local_engine: Option<PathBuf>,
    /// Prepared GGUF weights or first split. Defaults to MEDEN_LOCAL_MODEL.
    #[arg(long, conflicts_with_all = ["provider", "model", "base_url", "approve_processor"])]
    local_model: Option<PathBuf>,
}

impl ModelArgs {
    pub(crate) fn local_config(&self) -> Result<Option<meden_llm::LocalConfig>> {
        if self.provider.is_some() || self.base_url.is_some() {
            return Ok(None);
        }
        anyhow::ensure!(
            self.model.is_none() && self.approve_processor.is_none(),
            "HTTP model/approval requires --base-url or --provider"
        );
        let engine = self
            .local_engine
            .clone()
            .or_else(|| std::env::var_os("MEDEN_LOCAL_ENGINE").map(PathBuf::from));
        let model = self
            .local_model
            .clone()
            .or_else(|| std::env::var_os("MEDEN_LOCAL_MODEL").map(PathBuf::from));
        Ok(Some(meden_llm::LocalConfig {
            engine: engine.context("configure --local-engine and --local-model; for HTTP use --base-url URL --model MODEL or --provider NAME")?,
            model: model.context("local inference requires prepared GGUF weights via --local-model or MEDEN_LOCAL_MODEL")?,
        }))
    }

    pub(crate) fn connections(&self) -> Result<Vec<Provider>> {
        if let Some(base_url) = &self.base_url {
            let model = self
                .model
                .as_ref()
                .filter(|model| !model.trim().is_empty())
                .context("--base-url requires a nonempty --model")?;
            let api_key = if let Some(name) = &self.api_key_env {
                Some(
                    std::env::var(name)
                        .ok()
                        .filter(|key| !key.trim().is_empty())
                        .with_context(|| {
                            format!("API key environment variable {name:?} is unset or empty")
                        })?,
                )
            } else {
                anyhow::ensure!(
                    self.api_key
                        .as_ref()
                        .is_none_or(|key| !key.trim().is_empty()),
                    "--api-key must be nonempty"
                );
                self.api_key.clone()
            };
            return Ok(vec![Provider {
                id: "configured".into(),
                label: "Configured server".into(),
                model: model.clone(),
                base_url: base_url.clone(),
                protocol: Protocol::ChatCompletions,
                api_key,
                note: "from command-line connection settings".into(),
            }]);
        }
        let found: Vec<_> = meden_llm::detect()
            .into_iter()
            .map(|mut provider| {
                if let Some(model) = &self.model {
                    provider.model.clone_from(model);
                }
                provider
            })
            .collect();
        match self.provider.as_deref() {
            Some(wanted) if wanted != "auto" => {
                let provider = found.into_iter().find(|provider| provider.id == wanted)
                    .with_context(|| format!("no provider called {wanted:?}; try `meden providers` (subscriptions: codex login / hermes auth, claude auth login / claude setup-token)"))?;
                Ok(vec![provider])
            }
            _ => Ok(found),
        }
    }

    pub(crate) fn connection(&self) -> Result<Provider> {
        self.connections()?
            .into_iter()
            .next()
            .context("no model found; try `meden providers`")
    }
}
