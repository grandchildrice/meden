//! File-local build diagnostics, independent of the chat transcript.
use crate::{
    adapters::Observer,
    gui::{lock, View},
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, serde::Serialize)]
pub(in crate::gui) struct Preparation {
    pub state: &'static str,
    pub cache_hit: bool,
    pub error: Option<String>,
    phase: String,
    elapsed_ms: u64,
    phase_elapsed_ms: u64,
    limit_seconds: u64,
    log: String,
    log_truncated: bool,
    #[serde(skip)]
    started: Instant,
    #[serde(skip)]
    phase_started: Instant,
    #[serde(skip)]
    prefix: String,
}

impl Preparation {
    pub fn new() -> Self {
        Self {
            state: "preparing",
            cache_hit: false,
            error: None,
            phase: "Resolving Lean modules".into(),
            elapsed_ms: 0,
            phase_elapsed_ms: 0,
            limit_seconds: 0,
            log: String::new(),
            log_truncated: false,
            started: Instant::now(),
            phase_started: Instant::now(),
            prefix: String::new(),
        }
    }
    pub fn finish(&mut self, error: Option<String>, cache_hit: bool) {
        self.state = if error.is_some() { "failed" } else { "ready" };
        self.error = error;
        self.cache_hit = cache_hit;
        self.elapsed_ms = millis(self.started);
        self.phase_elapsed_ms = millis(self.phase_started);
    }
}

fn millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
fn tail(text: &str) -> (String, bool) {
    let mut start = text.len().saturating_sub(65_536);
    while !text.is_char_boundary(start) {
        start = start.saturating_add(1);
    }
    (text[start..].into(), start > 0)
}

pub(super) fn observer(view: &Arc<Mutex<View>>, paths: &[String]) -> Observer {
    let view = Arc::clone(view);
    let paths = paths.to_vec();
    Arc::new(move |phase, limit, output| {
        let mut view = lock(&view);
        for path in &paths {
            let Some(preparation) = view.source_preparation.get_mut(path) else {
                continue;
            };
            if preparation.state != "preparing" {
                continue;
            }
            if output.is_none() {
                preparation.phase = phase.into();
                preparation.phase_started = Instant::now();
                preparation.limit_seconds = limit.as_secs();
                preparation.prefix = format!("{}\n[{phase}]\n", preparation.log);
            }
            let text = output.map_or_else(String::new, |p| {
                crate::text::visible(&format!("{}{}", p.stdout, p.stderr))
            });
            let (log, truncated) = tail(&format!("{}{text}", preparation.prefix));
            preparation.log = log;
            preparation.log_truncated |= truncated || output.is_some_and(|p| p.truncated);
            preparation.elapsed_ms = millis(preparation.started);
            preparation.phase_elapsed_ms = millis(preparation.phase_started);
        }
    })
}
