//! Persistent, offline llama.cpp inference with fixed prepared weights.

use std::path::PathBuf;
use std::sync::Arc;

use meden_agent::conversation::{Delta, ModelError, Reply};
use meden_core::processor::{LocalCall, LocalParameters};
use meden_runtime::process::Control;
use serde_json::Value;

/// Prepared host configuration; never deserialize from model/tool input.
#[derive(Clone, Debug)]
pub struct LocalConfig {
    /// Installed llama-server executable, supporting Unix socket listeners.
    pub engine: PathBuf,
    /// Prepared GGUF file, or first named split with all siblings installed.
    /// No downloads or cloud fallback occur.
    pub model: PathBuf,
}

pub(crate) struct Local {
    parameters: LocalParameters,
    #[cfg(target_os = "linux")]
    state: std::sync::Mutex<State>,
    #[cfg(target_os = "linux")]
    control: Arc<Control>,
    #[cfg(target_os = "linux")]
    root: tempfile::TempDir,
}

#[cfg(target_os = "linux")]
struct State {
    command: std::process::Command,
    server: Option<meden_runtime::process::Background>,
    started: bool,
}

impl Local {
    pub(crate) fn parameters(&self) -> &LocalParameters {
        &self.parameters
    }
    pub(crate) fn stop(&self) {
        #[cfg(target_os = "linux")]
        self.control.cancel();
    }

    pub(crate) fn new(
        config: &LocalConfig,
        worker: &std::path::Path,
        control: Arc<Control>,
    ) -> Result<Self, ModelError> {
        #[cfg(target_os = "linux")]
        {
            let root = tempfile::Builder::new()
                .prefix("meden-inference-")
                .tempdir()
                .map_err(failed)?;
            let command = meden_runtime::sandbox::Sandbox::inference_command(
                root.path(),
                worker,
                &config.engine,
                &config.model,
            )
            .map_err(failed)?;
            Ok(Self {
                parameters: LocalParameters::new(
                    config.engine.as_os_str().as_encoded_bytes().to_vec(),
                    config.model.as_os_str().as_encoded_bytes().to_vec(),
                ),
                state: std::sync::Mutex::new(State {
                    command,
                    server: None,
                    started: false,
                }),
                control,
                root,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (config, worker, control);
            Err(failed("isolated local inference requires Linux"))
        }
    }
}

impl Local {
    pub(crate) fn reply(
        &self,
        authorization: LocalCall,
        messages: &[Value],
        tools: &[Value],
        notify: &dyn Fn(Delta<'_>),
    ) -> Result<Reply, ModelError> {
        if !authorization.consume_for(&self.parameters) {
            return Err(failed("local processor binding mismatch"));
        }
        self.exchange(messages, tools, notify)
    }

    fn exchange(
        &self,
        messages: &[Value],
        tools: &[Value],
        notify: &dyn Fn(Delta<'_>),
    ) -> Result<Reply, ModelError> {
        #[cfg(target_os = "linux")]
        {
            use std::sync::atomic::Ordering;
            use std::time::{Duration, Instant};
            let mut state = self
                .state
                .lock()
                .map_err(|_| failed("local inference state unavailable"))?;
            let result = (|| {
                if self.control.flag().load(Ordering::SeqCst) {
                    return Err(failed("local inference cancelled"));
                }
                if !state.started {
                    state.started = true;
                    state.server = Some(
                        meden_runtime::process::Background::spawn(
                            &mut state.command,
                            Arc::clone(&self.control),
                        )
                        .map_err(failed)?,
                    );
                    let started = Instant::now();
                    loop {
                        if self.control.flag().load(Ordering::SeqCst)
                            || started.elapsed() >= Duration::from_secs(120)
                            || !state
                                .server
                                .as_mut()
                                .is_some_and(|server| server.is_running().unwrap_or(false))
                        {
                            return Err(failed(
                                "isolated local engine failed to start; no fallback",
                            ));
                        }
                        if crate::unix::agent(
                            &self.root.path().join("llama.sock"),
                            Duration::from_secs(1),
                        )
                        .get("http://meden.local/health")
                        .call()
                        .is_ok_and(|response| response.status().is_success())
                        {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(25));
                    }
                }
                if self.control.flag().load(Ordering::SeqCst)
                    || !state
                        .server
                        .as_mut()
                        .is_some_and(|server| server.is_running().unwrap_or(false))
                {
                    return Err(failed("isolated local engine stopped; restart the session"));
                }
                let body = crate::openai::request_body("meden-local", messages, tools);
                let bytes = serde_json::to_vec(&body).map_err(failed)?;
                if bytes.len() > 8_388_608 {
                    return Err(failed("local inference request exceeds 8 MiB"));
                }
                let agent = crate::unix::agent(
                    &self.root.path().join("llama.sock"),
                    Duration::from_secs(300),
                );
                let mut response = agent
                    .post("http://meden.local/v1/chat/completions")
                    .header("content-type", "application/json")
                    .send(bytes.as_slice())
                    .map_err(|error| match error {
                        ureq::Error::Timeout(_) => failed(
                            "local inference exchange timed out after 300 seconds; no fallback",
                        ),
                        _ => failed("local inference exchange failed; no fallback"),
                    })?;
                if !response.status().is_success() {
                    return Err(failed("local inference returned an error"));
                }
                crate::stream::read_response(
                    &mut response,
                    &crate::Protocol::ChatCompletions,
                    notify,
                    1_048_576,
                )
            })();
            if result.is_err() {
                state.server = None;
            }
            result
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (messages, tools, notify);
            Err(failed("isolated local inference requires Linux"))
        }
    }
}

fn failed(error: impl std::fmt::Display) -> ModelError {
    ModelError::Unavailable(error.to_string())
}
