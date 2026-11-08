"""Require real Rust faults to fail unchanged refinement proofs.

The transition models must compile before a proof failure counts. All owned
source/model bytes are restored, including on interruption or failed checks.
"""

import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GROUPS = [
    {
        "source": "crates/meden-cli/src/text.rs",
        "package": "meden-cli",
        "model": "Text",
        "proof": "TextRefinement",
        "test": "text::tests::every_unicode_scalar_keeps_the_original_display_policy",
        "mutations": [
            ("terminal_escape_retained", "0x1b => 0x241b", "0x1b => 0x1b"),
            ("display_newline_lost", "0x0a => 0x0a", "0x0a => 0xfffd"),
            ("directional_override_retained", "0x202a..=0x202e", "0x202a..=0x202d"),
            ("ordinary_display_text_lost", "_ => code,", "_ => 0xfffd,"),
        ],
    },
    {
        "kernel": "transfer",
        "model": "Control",
        "proof": "TransferRefinement",
        "test": (
            "transfer::tests::exact_session_request_deadline_and_active_permission_are_required"
        ),
        "mutations": [
            ("request_mismatch", "permit.request == *request", "permit.request != *request"),
            ("expired_permit", "now < permit.expires", "now <= permit.expires"),
            ("spent_permit", "&& !permit.spent", "&& true"),
            (
                "foreign_transfer_session",
                "request_session == session",
                "request_session == session || true",
            ),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "EvidenceRefinement",
        "test": "evidence::tests::exact_current_evidence_is_required_for_each_obligation",
        "mutations": [
            (
                "untrusted_evidence",
                "!receipt.active\n            || origin != EvidenceOrigin::Checker",
                "!receipt.active\n            || (origin != EvidenceOrigin::Checker && false)",
            ),
            ("cancelled_evidence", "!receipt.active", "(!receipt.active && false)"),
            ("stale_evidence", "receipt.subject != *current", "false"),
            (
                "kind_substitution",
                "*expected != receipt.obligation",
                "*expected != receipt.obligation && expected.kind == receipt.obligation.kind",
            ),
            (
                "reused_result",
                "receipt.position != self.completed",
                "(receipt.position != self.completed && false)",
            ),
            ("stale_progress", "self.completed = 0", "let _ = self.completed"),
            (
                "stale_delivery",
                "&& self.subject == *current",
                "&& (self.subject == *current || true)",
            ),
            ("unreviewed_contract", "self.phase != EvidencePhase::Reviewed", "false"),
        ],
    },
    {
        "kernel": "export_control",
        "model": "Control",
        "proof": "ExportControlRefinement",
        "test": "export_control::tests::dispatch_requires_review_host_origin_and_a_fresh_attempt",
        "mutations": [
            (
                "unreviewed_approval",
                "(Phase::Reviewed, Origin::Host, ExportEvent::Approve)",
                "(Phase::Pending | Phase::Reviewed, Origin::Host, ExportEvent::Approve)",
            ),
            (
                "model_approval",
                "(Phase::Reviewed, Origin::Host, ExportEvent::Approve)",
                "(Phase::Reviewed, Origin::Model, ExportEvent::Approve)",
            ),
            (
                "review_reopens_terminal",
                "(_, Origin::Host, ExportEvent::Review) => phase",
                "(_, Origin::Host, ExportEvent::Review) => Phase::Reviewed",
            ),
            (
                "approval_does_not_close",
                "phase: Phase::Attempted,",
                "phase: Phase::Reviewed,",
            ),
        ],
    },
    {
        "kernel": "export_control",
        "model": "Control",
        "proof": "DispatchRefinement",
        "prerequisites": ["ExportControlRefinement"],
        "test": "export_control::tests::effects_are_frozen_reviewed_session_bound_and_single_use",
        "mutations": [
            (
                "forged_effect_origin",
                "let decision = step(self.phase, origin, event);",
                "let decision = step(self.phase, { let _ = origin; Origin::Host }, event);",
            ),
            (
                "substituted_effect_payload",
                "&self.body",
                "{ let _ = &self.body; &[] }",
            ),
            (
                "uncommitted_effect_attempt",
                "self.phase = decision.phase;",
                "let _ = decision.phase;",
            ),
            (
                "missing_authorized_effect",
                "Some(Dispatch {\n                request: &self.request,\n            })",
                "None",
            ),
            (
                "foreign_export_session",
                "request_session == session",
                "(request_session == session || true)",
            ),
            ("expired_export_approval", "now < expires", "now <= expires"),
        ],
    },
    {
        "source": "crates/meden-cli/src/tui/approval.rs",
        "package": "meden-cli",
        "model": "Approval",
        "proof": "ApprovalRefinement",
        "test": "tui::approval::tests::only_unmodified_key_presses_can_approve",
        "mutations": [
            (
                "repeat_approval",
                "key.kind == KeyEventKind::Press",
                "(key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat)",
            ),
            ("modified_approval", "key.modifiers.is_empty()", "(key.modifiers.is_empty() || true)"),
            (
                "enter_approval",
                "key.code == KeyCode::Char('y')",
                "(key.code == KeyCode::Char('y') || key.code == KeyCode::Enter)",
            ),
            ("paste_approval", "} else {\n        false\n", "} else {\n        true\n"),
        ],
    },
    {
        "source": "crates/meden-cli/src/tui/approval.rs",
        "package": "meden-cli",
        "model": "Approval",
        "proof": "ApprovalRefinement",
        "test": "tui::approval::tests::approval_is_bound_to_the_current_review_target",
        "mutations": [
            (
                "approval_outside_review",
                "Screen::ReviewingExport => Some(Choice::Export)",
                "Screen::ReviewingExport | Screen::Instructing => Some(Choice::Export)",
            ),
            (
                "wrong_review_target",
                "Screen::ReviewingExport => Some(Choice::Export)",
                "Screen::ReviewingExport => { let _ = Choice::Export; Some(Choice::Processor) }",
            ),
            (
                "untrusted_review_input",
                "if !approves(event) {",
                "if !approves(event) && false {",
            ),
        ],
    },
    {
        "source": "crates/meden-runtime/src/verification/turn.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "proof": "RepairRefinement",
        "test": (
            "verification::turn::tests::"
            "checks_require_authority_and_budget_stops_still_obtain_fresh_evidence"
        ),
        "mutations": [
            ("unaccepted_check", "accepted && changed", "(accepted || true) && changed"),
            ("unchanged_check", "accepted && changed", "accepted && (changed || true)"),
            (
                "budget_reuses_old_rejection",
                "!fresh && matches!(failure, Some(FailureKind::Unverified))",
                "(!fresh || true) && matches!(failure, Some(FailureKind::Unverified))",
            ),
        ],
    },
    {
        "source": "crates/meden-agent/src/conversation.rs",
        "package": "meden-runtime",
        "model": "Conversation",
        "proof": "ConversationRefinement",
        "test": (
            "verification::turn::tests::"
            "checks_require_authority_and_budget_stops_still_obtain_fresh_evidence"
        ),
        "mutations": [
            ("budget_skips_check", "Self::Incomplete => Some(true)", "Self::Incomplete => None"),
            (
                "failure_restarts_check",
                "Self::Failed(_) | Self::Cancelled | Self::NeedsReview => None",
                "Self::Failed(_) => Some(false), Self::Cancelled | Self::NeedsReview => None",
            ),
        ],
    },
    {
        "source": "crates/meden-agent/src/flow.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "proof": "ConversationRefinement",
        "test": (
            "conversation::tests::"
            "host_stop_observed_during_decision_prevents_dispatch_and_stays_latched"
        ),
        "mutations": [
            (
                "ignored_host_stop",
                "        host_stop\n    }",
                "        let _ = host_stop; None\n    }",
            ),
        ],
    },
    {
        "source": "crates/meden-agent/src/flow.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "proof": "ConversationRefinement",
        "test": (
            "flow::tests::"
            "cancellation_at_request_notification_is_sticky_and_maximum_budget_does_not_overflow"
        ),
        "mutations": [
            ("cancelled_request_started", "if cancelled {", "if cancelled && false {"),
        ],
    },
    {
        "source": "crates/meden-agent/src/flow.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "proof": "ConversationRefinement",
        "test": (
            "flow::tests::a_stopped_tool_drains_all_results_without_restarting_after_flags_reset"
        ),
        "mutations": [
            (
                "stopped_tool_dispatched",
                "execute: !self.faulted && self.stopped.is_none()",
                "execute: !self.faulted && (self.stopped.is_none() || true)",
            ),
            (
                "lost_tool_stop",
                "if let Err(ExecutionError::Stopped(status)) = result {\n"
                "            self.latch(Some(replace(status, Status::Incomplete)));",
                "if let Err(ExecutionError::Stopped(status)) = result {\n"
                "            let _ = replace(status, Status::Incomplete);",
            ),
        ],
    },
    {
        "source": "crates/meden-agent/src/flow.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "proof": "ConversationRefinement",
        "test": (
            "flow::tests::calls_and_results_stay_frozen_until_acknowledged_and_budget_is_bounded"
        ),
        "mutations": [
            (
                "unbounded_inference",
                "self.remaining.saturating_sub(1)",
                "self.remaining.saturating_sub(0)",
            ),
            ("duplicate_call_record", "self.next.saturating_add(1)", "self.next.saturating_add(0)"),
        ],
    },
    {
        "source": "crates/meden-agent/src/flow.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "proof": "ConversationObservations",
        "prerequisites": ["ConversationRefinement"],
        "test": (
            "flow::tests::calls_and_results_stay_frozen_until_acknowledged_and_budget_is_bounded"
        ),
        "mutations": [
            (
                "mismatched_call_slot",
                "self.reply.tool_calls.get(self.next)",
                "self.reply.tool_calls.get(0)",
            ),
        ],
    },
    {
        "source": "crates/meden-runtime/src/verification/turn.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "proof": "RepairRefinement",
        "test": (
            "verification::turn::tests::"
            "terminal_stops_are_sticky_and_only_four_model_rounds_are_permitted"
        ),
        "mutations": [
            ("cancelled_turn_restarted", "if cancelled {", "if cancelled && false {"),
            (
                "stopped_turn_reopened",
                "return Effect::Stop(reason);",
                "let _ = Effect::Stop(reason); self.stage = Stage::Ready;",
            ),
            (
                "unbounded_model_rounds",
                "self.remaining.checked_sub(1)",
                "self.remaining.checked_sub(0)",
            ),
        ],
    },
    {
        "source": "crates/meden-runtime/src/verification/turn.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "proof": "RepairRefinement",
        "test": (
            "verification::turn::tests::unattributed_check_failure_never_starts_automatic_repair"
        ),
        "mutations": [
            (
                "unattributed_check_repaired",
                "self.stop(Terminal::Failed)\n                }\n            },\n            _ =>",
                "self.repair()\n                }\n            },\n            _ =>",
            ),
        ],
    },
    {
        "source": "crates/meden-runtime/src/verification/turn.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "proof": "RepairRefinement",
        "test": "verification::turn::tests::failed_source_probes_cannot_reuse_an_old_rejection",
        "mutations": [
            (
                "unavailable_probe_repaired",
                "                Err(FailureKind::Unavailable | FailureKind::Inconclusive) => {\n"
                "                    self.stop(Terminal::Failed)",
                "                Err(FailureKind::Unavailable | FailureKind::Inconclusive) => {\n"
                "                    self.repair()",
            ),
            (
                "rejected_probe_reuses_old_diagnostic",
                "Ok(false) | Err(FailureKind::Unverified) => {",
                "Err(FailureKind::Unverified) => self.repair(),\n                Ok(false) => {",
            ),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "CompletionRefinement",
        "prerequisites": ["EvidenceRefinement", "CheckJobsRefinement"],
        "test": "evidence::tests::admission_requires_an_issued_job_and_the_exact_accepted_result",
        "mutations": [
            (
                "inactive_completion",
                "let active = self.finish(origin, gate, ticket, current);",
                "let active = { let _ = self.finish(origin, gate, ticket, current); true };",
            ),
            (
                "failed_completion",
                "passed: result.passed,",
                "passed: true,",
            ),
            (
                "substituted_completion_artifact",
                "subject: result.subject,",
                "subject: current.clone(),",
            ),
            (
                "refused_completion_reported_success",
                "if !gate.refresh(EvidenceOrigin::Controller, result.subject.clone()) {\n"
                "            return false;",
                "if !gate.refresh(EvidenceOrigin::Controller, result.subject.clone()) {\n"
                "            return true;",
            ),
        ],
    },
    {
        "source": "crates/meden-runtime/src/verification/failure.rs",
        "package": "meden-cli",
        "model": "Repair",
        "proof": "CheckFailureRefinement",
        "test": "session::tests::operational_verification_failure_stops_automatic_repair",
        "mutations": [
            (
                "operational_failure_repaired",
                "matches!(failure, Some(kind) if !kind.repairable())",
                "matches!(failure, Some(kind) if kind.repairable())",
            ),
            (
                "missing_operational_stop",
                "matches!(failure, Some(kind) if !kind.repairable())",
                "{ let _ = failure; false }",
            ),
        ],
    },
    {
        "source": "crates/meden-core/src/evidence.rs",
        "package": "meden-core",
        "model": "Evidence",
        "proof": "CheckJobsRefinement",
        "prerequisites": ["EvidenceRefinement"],
        "test": "evidence::tests::issued_jobs_are_session_bound_single_use_and_never_resurrected",
        "mutations": [
            (
                "unaccepted_check_job",
                "if origin != EvidenceOrigin::Controller\n            || !gate.is_accepted()",
                "if origin != EvidenceOrigin::Controller\n"
                "            || (!gate.is_accepted() && false)",
            ),
            (
                "untrusted_check_job",
                "if origin != EvidenceOrigin::Controller\n            || !gate.is_accepted()",
                "if (origin != EvidenceOrigin::Controller && false)\n"
                "            || !gate.is_accepted()",
            ),
            (
                "foreign_session_check_job",
                "self.session != subject.session",
                "(self.session != subject.session && false)",
            ),
            (
                "reused_check_job_id",
                "self.sequence.checked_add(1)",
                "self.sequence.checked_add(0)",
            ),
            (
                "wrapped_check_job_id",
                "self.sequence.checked_add(1)",
                "Some(self.sequence.wrapping_add(1))",
            ),
            (
                "cancelled_check_job_retained",
                "self.active = None;\n        true\n    }\n\n    /// A failed old job",
                "let _ = &self.active;\n        true\n    }\n\n    /// A failed old job",
            ),
            (
                "check_job_input_substitution",
                "ticket.subject != *current",
                "(ticket.subject != *current && false)",
            ),
            (
                "completed_check_job_reused",
                "self.active = None;\n        true\n    }\n\n    /// A status observation",
                "let _ = &self.active;\n        true\n    }\n\n    /// A status observation",
            ),
            (
                "old_check_job_aborts_new",
                "if !self.matches(ticket) {",
                "if !self.matches(ticket) && false {",
            ),
        ],
    },
    {
        "source": "crates/meden-runtime/src/verification/failure.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "proof": "CheckFailureRefinement",
        "test": "verification::failure::tests::worker_limits_and_signals_are_not_repairable",
        "mutations": [
            ("truncated_check_repaired", "if truncated {", "if truncated && false {"),
            (
                "signalled_check_repaired",
                "status.into_raw() & 127 == 0",
                "{ let _ = status.into_raw(); true }",
            ),
            (
                "limited_check_repaired",
                "Termination::TimedOut | Termination::Cancelled | Termination::OutputLimit => {\n"
                "                FailureKind::Inconclusive",
                "Termination::TimedOut | Termination::Cancelled | Termination::OutputLimit => {\n"
                "                FailureKind::Unverified",
            ),
        ],
    },
]


def source_path(group):
    if "source" in group:
        return ROOT / group["source"]
    return ROOT / f"crates/meden-core/src/{group['kernel']}.rs"


def run(args, cwd=ROOT):
    # Faults must reach assertions/proofs under the same warning policy as CI.
    return subprocess.run(
        args,
        cwd=cwd,
        env={**os.environ, "RUSTFLAGS": "-D warnings"},
        capture_output=True,
        text=True,
        check=False,
    )


def require_success(args, cwd=ROOT):
    result = run(args, cwd)
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)


def main():
    originals = {}
    for group in GROUPS:
        for path in [
            source_path(group),
            ROOT / f"lean/MedenSpec/Generated/{group['model']}.lean",
        ]:
            originals[path] = path.read_bytes()
    logs = ROOT / "target/verification/mutants"
    logs.mkdir(parents=True, exist_ok=True)
    require_success(["lake", "build", "MedenProof"], ROOT / "lean")
    try:
        for group in GROUPS:
            kernel = source_path(group)
            for name, old, new in group["mutations"]:
                source = originals[kernel].decode()
                if source.count(old) != 1:
                    raise RuntimeError(f"{name}: expected exactly one mutation target")
                kernel.write_text(source.replace(old, new))
                test = run(
                    [
                        "cargo",
                        "test",
                        "-p",
                        group.get("package", "meden-core"),
                        group["test"],
                        "--",
                        "--exact",
                    ]
                )
                test_output = test.stdout + test.stderr
                (logs / f"{name}-rust.txt").write_text(test_output)
                if test.returncode == 0 or "test result: FAILED" not in test_output:
                    raise RuntimeError(f"{name}: expected an observed Rust failure\n{test_output}")
                require_success(
                    ["bash", "tools/extract-transfer.sh", "--write", group["model"].lower()]
                )
                require_success(
                    ["lake", "build", f"MedenSpec.Generated.{group['model']}"], ROOT / "lean"
                )
                for prerequisite in group.get("prerequisites", []):
                    require_success(["lake", "build", f"MedenSpec.{prerequisite}"], ROOT / "lean")
                result = run(
                    ["lake", "env", "lean", f"MedenSpec/{group['proof']}.lean"], ROOT / "lean"
                )
                output = result.stdout + result.stderr
                (logs / f"{name}.txt").write_text(output)
                operational_errors = (
                    "maximum number of heartbeats",
                    "maximum recursion depth",
                    "stack overflow",
                    "Unknown constant",
                    "Unknown identifier",
                    "unknown module prefix",
                )
                if any(error.casefold() in output.casefold() for error in operational_errors):
                    raise RuntimeError(
                        f"{name}: operational proof failure does not count\n{output}"
                    )
                proof_errors = (
                    "unsolved goals",
                    "made no progress",
                    "error: Tactic ",
                    "error: Type mismatch",
                    "error: Application type mismatch",
                )
                if result.returncode == 0 or not any(error in output for error in proof_errors):
                    raise RuntimeError(f"{name}: expected a correctness proof failure\n{output}")
                print(f"{name}: refinement rejected the Rust fault", flush=True)
                for path, original in originals.items():
                    path.write_bytes(original)
    finally:
        for path, original in originals.items():
            path.write_bytes(original)
        require_success(["lake", "build", "MedenProof"], ROOT / "lean")


if __name__ == "__main__":
    main()
