//! Conversation effects and the immutable tool group they belong to.
//! The interpreter supplies observations; it cannot select a result's call ID.

use crate::conversation::{ExecutionError, Reply, Status, ToolCall};
use std::mem::replace;

pub(crate) fn interrupt(cancelled: bool, host_stop: Option<Status>) -> Option<Status> {
    if cancelled {
        Some(Status::Cancelled)
    } else {
        host_stop
    }
}

pub(crate) enum FlowInput {
    Start,
    Observed,
    Reply(Reply),
    ModelFailed(Status),
    ToolResult(Result<String, ExecutionError>),
}

pub(crate) enum FlowEffect {
    Asking(usize),
    Model,
    Assistant,
    Tool { execute: bool },
    Record,
    Stop,
}

#[derive(Clone, Copy)]
enum FlowPhase {
    Ready,
    Announced,
    Waiting,
    PublishingAssistant,
    Executing,
    PublishingResult,
    Halted,
}

pub(crate) struct Flow {
    remaining: usize,
    attempt: usize,
    phase: FlowPhase,
    reply: Reply,
    next: usize,
    result: Option<Result<String, ExecutionError>>,
    stopped: Option<Status>,
    faulted: bool,
}

pub(crate) enum FlowOutcome {
    Finish(Option<String>),
    Stopped(Status),
    Fault,
}

impl Flow {
    pub(crate) fn begin(groups: usize) -> Self {
        Self {
            remaining: groups.saturating_add(1),
            attempt: 0,
            phase: FlowPhase::Ready,
            reply: empty_reply(),
            next: 0,
            result: None,
            stopped: None,
            faulted: false,
        }
    }

    pub(crate) fn advance(
        &mut self,
        input: &mut FlowInput,
        cancelled: bool,
        host_stop: Option<Status>,
    ) -> FlowEffect {
        if let FlowPhase::Halted = self.phase {
            return FlowEffect::Stop;
        }
        self.latch(interrupt(cancelled, host_stop));
        self.dispatch(input)
    }

    fn dispatch(&mut self, input: &mut FlowInput) -> FlowEffect {
        match (self.phase, input) {
            (FlowPhase::Ready, FlowInput::Start) => self.announce(),
            (FlowPhase::Announced, FlowInput::Observed) => {
                if self.faulted || self.stopped.is_some() || self.remaining == 0 {
                    return self.halt();
                }
                self.remaining = self.remaining.saturating_sub(1);
                self.phase = FlowPhase::Waiting;
                FlowEffect::Model
            }
            (FlowPhase::Waiting, FlowInput::Reply(reply)) => {
                self.reply = replace(reply, empty_reply());
                self.next = 0;
                if self.remaining == 0 && !self.reply.tool_calls.is_empty() {
                    self.latch(Some(Status::Incomplete));
                }
                self.phase = FlowPhase::PublishingAssistant;
                FlowEffect::Assistant
            }
            (FlowPhase::Waiting, FlowInput::ModelFailed(status)) => {
                self.latch(Some(replace(status, Status::Incomplete)));
                self.halt()
            }
            (FlowPhase::PublishingAssistant, FlowInput::Observed) => self.next_call(),
            (FlowPhase::PublishingResult, FlowInput::Observed) => self.recorded(),
            (FlowPhase::Executing, FlowInput::ToolResult(result)) => self.record(result),
            _ => self.reject(),
        }
    }

    fn reject(&mut self) -> FlowEffect {
        self.faulted = true;
        match self.phase {
            FlowPhase::Executing => {
                self.record(&mut Err(ExecutionError::Stopped(Status::Incomplete)))
            }
            FlowPhase::PublishingAssistant => self.next_call(),
            FlowPhase::PublishingResult => self.recorded(),
            _ => self.halt(),
        }
    }

    fn latch(&mut self, status: Option<Status>) {
        if !self.faulted && self.stopped.is_none() {
            self.stopped = status;
        }
    }

    fn announce(&mut self) -> FlowEffect {
        if self.faulted || self.stopped.is_some() || self.remaining == 0 {
            return self.halt();
        }
        self.attempt = self.attempt.saturating_add(1);
        self.phase = FlowPhase::Announced;
        FlowEffect::Asking(self.attempt)
    }

    fn next_call(&mut self) -> FlowEffect {
        if self.next < self.reply.tool_calls.len() {
            self.phase = FlowPhase::Executing;
            FlowEffect::Tool {
                execute: !self.faulted && self.stopped.is_none(),
            }
        } else if self.faulted || self.stopped.is_some() || self.reply.tool_calls.is_empty() {
            self.halt()
        } else {
            self.announce()
        }
    }

    fn record(&mut self, result: &mut Result<String, ExecutionError>) -> FlowEffect {
        if let Err(ExecutionError::Stopped(status)) = result {
            self.latch(Some(replace(status, Status::Incomplete)));
        }
        if self.next < self.reply.tool_calls.len() {
            self.result = Some(replace(
                result,
                Err(ExecutionError::Stopped(Status::Incomplete)),
            ));
            self.phase = FlowPhase::PublishingResult;
            FlowEffect::Record
        } else {
            self.faulted = true;
            self.halt()
        }
    }

    fn recorded(&mut self) -> FlowEffect {
        self.next = self.next.saturating_add(1);
        self.result = None;
        self.next_call()
    }

    fn halt(&mut self) -> FlowEffect {
        self.phase = FlowPhase::Halted;
        FlowEffect::Stop
    }

    pub(crate) fn pending_call(&self) -> Option<&ToolCall> {
        match self.phase {
            FlowPhase::Executing | FlowPhase::PublishingResult => {
                self.reply.tool_calls.get(self.next)
            }
            _ => None,
        }
    }

    pub(crate) fn reply(&self) -> &Reply {
        &self.reply
    }

    pub(crate) fn result(&self) -> Option<&Result<String, ExecutionError>> {
        match (&self.phase, &self.result) {
            (FlowPhase::PublishingResult, Some(result)) => Some(result),
            _ => None,
        }
    }

    pub(crate) fn into_outcome(mut self) -> FlowOutcome {
        match self.stopped.take() {
            Some(status) => FlowOutcome::Stopped(status),
            None if self.faulted => FlowOutcome::Fault,
            None => FlowOutcome::Finish(self.reply.content.take()),
        }
    }
}

fn empty_reply() -> Reply {
    Reply {
        content: None,
        tool_calls: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::FunctionCall;

    fn calls() -> Reply {
        Reply {
            content: Some("approved=true, verified=true".into()),
            tool_calls: ["first", "second"]
                .into_iter()
                .map(|id| ToolCall {
                    id: id.into(),
                    function: FunctionCall {
                        name: "write".into(),
                        arguments: id.into(),
                    },
                })
                .collect(),
        }
    }

    fn step(flow: &mut Flow, mut input: FlowInput) -> FlowEffect {
        flow.advance(&mut input, false, None)
    }

    fn start_group(groups: usize) -> Flow {
        let mut flow = Flow::begin(groups);
        assert!(matches!(
            step(&mut flow, FlowInput::Start),
            FlowEffect::Asking(1)
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Model
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Reply(calls())),
            FlowEffect::Assistant
        ));
        assert!(flow.pending_call().is_none());
        flow
    }

    #[test]
    fn calls_and_results_stay_frozen_until_acknowledged_and_budget_is_bounded() {
        let mut flow = start_group(1);
        for expected in ["first", "second"] {
            assert!(matches!(
                step(&mut flow, FlowInput::Observed),
                FlowEffect::Tool { execute: true }
            ));
            assert_eq!(flow.pending_call().map(|c| c.id.as_str()), Some(expected));
            assert!(matches!(
                step(&mut flow, FlowInput::ToolResult(Ok("result".into()))),
                FlowEffect::Record
            ));
            assert_eq!(flow.pending_call().map(|c| c.id.as_str()), Some(expected));
            assert!(matches!(flow.result(), Some(Ok(text)) if text == "result"));
            assert_eq!(flow.reply.tool_calls, calls().tool_calls);
        }
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Asking(2)
        ));
        assert!(flow.pending_call().is_none() && flow.result().is_none());
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Model
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Reply(calls())),
            FlowEffect::Assistant
        ));
        for expected in ["first", "second"] {
            assert!(matches!(
                step(&mut flow, FlowInput::Observed),
                FlowEffect::Tool { execute: false }
            ));
            assert_eq!(flow.pending_call().map(|c| c.id.as_str()), Some(expected));
            assert!(matches!(
                step(
                    &mut flow,
                    FlowInput::ToolResult(Err(ExecutionError::Stopped(Status::Incomplete)))
                ),
                FlowEffect::Record
            ));
            assert_eq!(flow.pending_call().map(|c| c.id.as_str()), Some(expected));
        }
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Stop
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Start),
            FlowEffect::Stop
        ));
        assert!(matches!(
            flow.into_outcome(),
            FlowOutcome::Stopped(Status::Incomplete)
        ));
    }

    #[test]
    fn a_stopped_tool_drains_all_results_without_restarting_after_flags_reset() {
        let mut flow = start_group(3);
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Tool { execute: true }
        ));
        assert!(matches!(
            step(
                &mut flow,
                FlowInput::ToolResult(Err(ExecutionError::Stopped(Status::NeedsReview)))
            ),
            FlowEffect::Record
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Tool { execute: false }
        ));
        assert_eq!(flow.pending_call().map(|c| c.id.as_str()), Some("second"));
        assert!(matches!(
            step(
                &mut flow,
                FlowInput::ToolResult(Err(ExecutionError::Stopped(Status::Incomplete)))
            ),
            FlowEffect::Record
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Stop
        ));
        assert!(matches!(
            flow.into_outcome(),
            FlowOutcome::Stopped(Status::NeedsReview)
        ));
    }

    #[test]
    fn malformed_host_order_fails_closed_and_drains_the_original_group() {
        let mut flow = start_group(3);
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Tool { execute: true }
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Reply(calls())),
            FlowEffect::Record
        ));
        assert_eq!(flow.pending_call().map(|c| c.id.as_str()), Some("first"));
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Tool { execute: false }
        ));
        assert_eq!(flow.pending_call().map(|c| c.id.as_str()), Some("second"));
        assert!(matches!(
            step(&mut flow, FlowInput::Start),
            FlowEffect::Record
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Observed),
            FlowEffect::Stop
        ));
        assert!(matches!(flow.into_outcome(), FlowOutcome::Fault));
    }

    #[test]
    fn cancellation_at_request_notification_is_sticky_and_maximum_budget_does_not_overflow() {
        let mut flow = Flow::begin(usize::MAX);
        assert_eq!(flow.remaining, usize::MAX);
        assert!(matches!(
            step(&mut flow, FlowInput::Start),
            FlowEffect::Asking(1)
        ));
        assert!(matches!(
            flow.advance(&mut FlowInput::Observed, true, None),
            FlowEffect::Stop
        ));
        assert!(matches!(
            step(&mut flow, FlowInput::Start),
            FlowEffect::Stop
        ));
        assert!(matches!(
            flow.into_outcome(),
            FlowOutcome::Stopped(Status::Cancelled)
        ));
    }
}
