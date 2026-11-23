//! Host clock/output attribution for application-owned preparation observations.
use crate::{
    adapters::Observer,
    gui::{lock, View},
};
use meden_app::view::jobs::Observation;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

pub(super) fn millis(started: Instant) -> u32 {
    u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX)
}
pub(super) fn observer(
    view: &Arc<Mutex<View>>,
    originals: &[Observation],
    started: Instant,
) -> Observer {
    let view = Arc::clone(view);
    let originals = originals.to_vec();
    Arc::new(move |phase, limit, output| {
        // Host-only normalization, once per callback rather than once per file.
        let output = output.map(|p| {
            (
                crate::text::visible(&format!("{}{}", p.stdout, p.stderr)),
                p.truncated,
            )
        });
        let mut view = lock(&view);
        for original in &originals {
            if let Err(error) = view.source_preparation.update(
                original,
                millis(started),
                phase,
                u32::try_from(limit.as_secs()).unwrap_or(u32::MAX),
                output
                    .as_ref()
                    .map(|(text, clipped)| (text.as_str(), *clipped)),
            ) {
                view.source_preparation.finish(
                    original,
                    millis(started),
                    Err(meden_app::view::jobs::Failure {
                        kind: meden_app::view::jobs::FailureKind::Observation,
                        message: format!("Preparation progress unavailable: {error:#}"),
                    }),
                );
            }
        }
    })
}

pub(super) fn failure(
    control: &meden_runtime::process::Control,
    message: String,
) -> meden_app::view::jobs::Failure {
    use meden_app::view::jobs::{Failure, FailureKind};
    Failure {
        kind: if control.flag().load(std::sync::atomic::Ordering::SeqCst) {
            FailureKind::Cancelled
        } else {
            FailureKind::Operation
        },
        message,
    }
}
