//! Host clocks and actual result attribution for the observation publisher.
//! This adapter never constructs or consumes kernel authority.
use super::{Reference, Summary};
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

pub(super) struct Progress<'a> {
    publisher: &'a Publisher,
    observation: Option<Observation>,
    started: Instant,
    control: &'a Control,
}
impl<'a> Progress<'a> {
    pub(super) fn new(
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
    pub(super) fn proposal(
        publisher: &'a Publisher,
        session: &str,
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
    pub(super) fn recording_proposal(&self) {
        self.publisher
            .recording_proposal(self.observation.as_ref(), self.elapsed());
    }
    fn elapsed(&self) -> u32 {
        u32::try_from(self.started.elapsed().as_millis()).unwrap_or(u32::MAX)
    }
    pub(super) fn source(&self, source: &str) {
        self.publisher.source(
            self.observation.as_ref(),
            self.elapsed(),
            super::hash(source.as_bytes()),
        );
    }
    pub(super) fn checking(&self, inputs: &Inputs) {
        self.publisher
            .checking(self.observation.as_ref(), self.elapsed(), inputs);
    }
    pub(super) fn refreshing(&self) {
        self.publisher
            .refreshing(self.observation.as_ref(), self.elapsed());
    }
    pub(super) fn retaining(&self) {
        self.publisher
            .retaining(self.observation.as_ref(), self.elapsed());
    }
    pub(super) fn finish(self, result: &Result<Summary>) {
        self.complete(result, Completion::Admitted);
    }
    pub(super) fn finish_proposal(self, result: &Result<Summary>) {
        self.complete(result, Completion::Proposed);
    }
    fn complete<'r>(
        mut self,
        result: &'r Result<Summary>,
        success: impl FnOnce(&'r Summary) -> Completion<'r>,
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
                            .map_or(FailureKind::Inconclusive, |failure| failure.kind),
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
