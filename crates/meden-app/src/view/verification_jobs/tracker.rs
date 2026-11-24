//! Host clocks and actual result attribution for the observation publisher.
//! This adapter never constructs or consumes kernel authority.
use crate::project::{Reference, Summary};
use crate::{
    verification::Contract,
    view::verification_jobs::{Completion, Observation, Publisher},
};
use anyhow::Result;
use meden_runtime::{
    process::Control,
    verification::{artifact::Inputs, Failure, FailureKind},
};
use std::{sync::atomic::Ordering, time::Instant};

pub(crate) struct Progress<'a> {
    publisher: &'a Publisher,
    observation: Option<Observation>,
    started: Instant,
    control: &'a Control,
}
impl<'a> Progress<'a> {
    pub(crate) fn new(
        publisher: &'a Publisher,
        reference: &Reference,
        contract: &Contract,
        control: &'a Control,
    ) -> Self {
        Self {
            publisher,
            observation: publisher.begin(reference, contract),
            started: Instant::now(),
            control,
        }
    }
    pub(crate) fn proposal(
        publisher: &'a Publisher,
        session: &crate::identity::SessionId,
        contract: &Contract,
        control: &'a Control,
    ) -> Self {
        Self {
            publisher,
            observation: publisher.begin_proposal(session, contract),
            started: Instant::now(),
            control,
        }
    }
    pub(crate) fn recording_proposal(&self) {
        self.publisher
            .recording_proposal(self.observation.as_ref(), self.elapsed());
    }
    pub(crate) fn recorded(
        publisher: &'a Publisher,
        session: &[u8],
        id: &str,
        control: &'a Control,
        origin: super::RecordedOrigin,
    ) -> Self {
        Self {
            publisher,
            observation: publisher.begin_recorded(session, id, origin),
            started: Instant::now(),
            control,
        }
    }
    pub(crate) fn work(&self, work: &crate::records::Work) {
        self.publisher
            .recorded_work(self.observation.as_ref(), self.elapsed(), work);
    }
    pub(crate) fn engine(&self, engine: &str) {
        self.publisher
            .recorded_engine(self.observation.as_ref(), self.elapsed(), engine);
    }
    pub(crate) fn inputs(&self, source: &str) {
        self.publisher
            .recorded_source(self.observation.as_ref(), self.elapsed(), source);
    }
    pub(crate) fn step(&self, next: super::recorded::Step) {
        self.publisher
            .recorded_step(self.observation.as_ref(), self.elapsed(), next);
    }
    pub(crate) fn finish_recorded(self, result: &Result<crate::records::store::Observation>) {
        self.complete(result, Completion::Recorded, FailureKind::Unavailable);
    }
    fn elapsed(&self) -> u32 {
        u32::try_from(self.started.elapsed().as_millis()).unwrap_or(u32::MAX)
    }
    pub(crate) fn source(&self, source: &str) {
        self.publisher.source(
            self.observation.as_ref(),
            self.elapsed(),
            crate::project::hash(source.as_bytes()),
        );
    }
    pub(crate) fn checking(&self, inputs: &Inputs) {
        self.publisher
            .checking(self.observation.as_ref(), self.elapsed(), inputs);
    }
    pub(crate) fn refreshing(&self) {
        self.publisher
            .refreshing(self.observation.as_ref(), self.elapsed());
    }
    pub(crate) fn retaining(&self) {
        self.publisher
            .retaining(self.observation.as_ref(), self.elapsed());
    }
    pub(crate) fn finish(self, result: &Result<Summary>) {
        self.complete(result, Completion::Admitted, FailureKind::Inconclusive);
    }
    pub(crate) fn finish_proposal(self, result: &Result<Summary>) {
        self.complete(result, Completion::Proposed, FailureKind::Inconclusive);
    }
    fn complete<'r, T>(
        mut self,
        result: &'r Result<T>,
        success: impl FnOnce(&'r T) -> Completion<'r>,
        unavailable: FailureKind,
    ) {
        let message;
        let outcome = match result {
            Ok(summary) => success(summary),
            Err(error) => {
                message = format!("{error:#}");
                if self.control.flag().load(Ordering::SeqCst) {
                    Completion::Cancelled(&message)
                } else {
                    Completion::Failed(
                        error
                            .downcast_ref::<Failure>()
                            .map_or(unavailable, |failure| failure.kind),
                        &message,
                    )
                }
            }
        };
        self.publisher
            .finish(self.observation.take(), self.elapsed(), outcome);
    }
}
impl Drop for Progress<'_> {
    fn drop(&mut self) {
        if self.observation.is_some() {
            self.publisher.failed_observation(
                self.observation.as_ref(),
                "Verification observation ended before a terminal result",
            );
        }
    }
}
