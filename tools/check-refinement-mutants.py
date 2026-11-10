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
        "source": "crates/meden-runtime/src/pages.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "proof": "PageNetworkRefinement",
        "prerequisites": ["PageNetwork"],
        "test": "pages::tests::excludes_special_use_and_transition_addresses",
        "mutations": [
            (
                "page_loopback_admitted",
                "0 | 10 | 127 | 224..=255 => false,",
                "0 | 10 | 224..=255 => false,",
            ),
            (
                "page_shared_space_boundary_admitted",
                "100 => !matches!(b, 64..=127),",
                "100 => !matches!(b, 65..=127),",
            ),
            (
                "page_ipv6_global_prefix_ignored",
                "if a & 0xe000 != 0x2000 {",
                "if a & 0xe000 != 0x2000 && false {",
            ),
            (
                "page_ipv6_transition_admitted",
                "0x2002 => false,",
                "0x2002 => true,",
            ),
        ],
    },
    {
        "proof": "ToolOutcomeRefinement",
        "prerequisites": [
            "ConversationRefinement",
            "CheckFailureRefinement",
            "ToolOutcome",
        ],
        "source": "crates/meden-runtime/src/file.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "test": (
            "file::tests::process_facts_precede_every_worker_response_"
            "and_only_complete_frames_succeed"
        ),
        "mutations": [
            (
                "file_result_ignores_truncation",
                "if truncated\n        ||",
                "if (truncated && false)\n        ||",
            ),
            (
                "file_result_accepts_nonzero_exit",
                "if word != 0 {\n"
                "                    return "
                "Err(ToolError::unavailable(diagnostic));",
                "if word != 0 && false {\n"
                "                    return "
                "Err(ToolError::unavailable(diagnostic));",
            ),
            (
                "file_result_accepts_worker_cancellation",
                "Err(Some(Err(_)) | None) => Err(ToolError::unavailable(diagnostic)),",
                "Err(Some(Err(_)) | None) => { let _ = diagnostic; "
                "Err(ToolError::cancelled()) },",
            ),
        ],
    },
    {
        "proof": "ToolOutcomeRefinement",
        "prerequisites": [
            "ConversationRefinement",
            "CheckFailureRefinement",
            "ToolOutcome",
        ],
        "source": "crates/meden-runtime/src/file.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "test": "file::tests::verification_repair_depends_on_error_kind_and_never_diagnostic_words",
        "mutations": [
            (
                "file_refusal_repaired_as_source",
                "Ok(Err(_)) | Err(Some(Ok(_))) => FailureKind::Unavailable,",
                "Ok(Err(_)) => FailureKind::Unverified,\n"
                "        Err(Some(Ok(_))) => FailureKind::Unavailable,",
            )
        ],
    },
    {
        "proof": "ToolOutcomeRefinement",
        "prerequisites": [
            "ConversationRefinement",
            "CheckFailureRefinement",
            "ToolOutcome",
        ],
        "source": "crates/meden-agent/src/conversation.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "test": (
            "conversation::tests::typed_tool_errors_preserve_refusal_"
            "cancellation_and_operational_stops"
        ),
        "mutations": [
            (
                "tool_refusal_changed_to_error",
                "Ok(Err(message)) => ExecutionError::Refused(message),",
                "Ok(Err(message)) => ExecutionError::Tool(message),",
            ),
            (
                "tool_cancellation_changed_to_incomplete",
                "Err(None) => ExecutionError::Stopped(Status::Cancelled),",
                "Err(None) => ExecutionError::Stopped(Status::Incomplete),",
            ),
        ],
    },
    {
        "proof": "ToolOutcomeRefinement",
        "prerequisites": [
            "ConversationRefinement",
            "CheckFailureRefinement",
            "ToolOutcome",
        ],
        "source": "crates/meden-agent/src/tool.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "test": (
            "tool::tests::failure_tags_and_diagnostics_survive_the_wire_"
            "without_reclassification"
        ),
        "mutations": [
            (
                "tool_refusal_constructor_changes_class",
                "pub fn refused(message: String) -> Self {\n"
                "        Self(Ok(Err(message)))",
                "pub fn refused(message: String) -> Self {\n"
                "        Self(Ok(Ok(message)))",
            ),
            (
                "tool_unavailable_constructor_changes_class",
                "pub fn unavailable(message: String) -> Self {\n"
                "        Self(Err(Some(Ok(message))))",
                "pub fn unavailable(message: String) -> Self {\n"
                "        Self(Ok(Ok(message)))",
            ),
        ],
    },
    {
        "source": "crates/meden-core/src/invocation.rs",
        "package": "meden-core",
        "model": "Policy",
        "proof": "InvocationContextRefinement",
        "prerequisites": ["PolicyRefinement", "InvocationRefinement"],
        "test": (
            "invocation::tests::the_admitted_call_retains_its_original_immutable_policy_context"
        ),
        "mutations": [
            (
                "bound_call_discards_requirements",
                "Ok((Call { request, rules }, requirements))",
                "{ let _ = requirements; Ok((Call { request, rules }, Vec::new())) }",
            ),
            (
                "bound_call_starts_already_announced",
                "Attempt::new(\n                Ok((Call { request, rules }, requirements)),\n"
                "                rules,\n                policy_refusal,\n            )",
                "{ let mut issued = Attempt::new(Ok((Call { request, rules }, requirements)), "
                "rules, policy_refusal); issued.announced = true; issued }",
            ),
            (
                "bound_call_changes_preparation_refusal",
                "Err(refusal) => Attempt::new(Err(refusal), rules, policy_refusal),",
                "Err(refusal) => { let _ = refusal; "
                "Attempt { checked: Some(Err(policy_refusal)), announced: false } },",
            ),
            (
                "bound_call_refuses_permitted_requests",
                "Attempt::new(\n                Ok((Call { request, rules }, requirements)),\n"
                "                rules,\n                policy_refusal,\n            )",
                "{ let _ = (request, requirements); "
                "Attempt { checked: Some(Err(policy_refusal)), announced: false } }",
            ),
        ],
    },
    {
        "source": "crates/meden-core/src/file.rs",
        "package": "meden-core",
        "model": "Policy",
        "proof": "FileRefinement",
        "prerequisites": [
            "PolicyRefinement", "InvocationRefinement", "InvocationContextRefinement"
        ],
        "test": "file::tests::resolved_resources_need_both_original_and_canonical_authority",
        "mutations": [
            (
                "resolved_ignores_original_authority",
                "allowed(&declared, rules) && allowed(&resolved, rules)",
                "(allowed(&declared, rules) || true) && allowed(&resolved, rules)",
            ),
            (
                "resolved_ignores_canonical_authority",
                "allowed(&declared, rules) && allowed(&resolved, rules)",
                "allowed(&declared, rules) && (allowed(&resolved, rules) || true)",
            ),
            (
                "resolved_accepts_either_path",
                "allowed(&declared, rules) && allowed(&resolved, rules)",
                "allowed(&declared, rules) || allowed(&resolved, rules)",
            ),
            (
                "resolved_checks_wrong_operation",
                "Request::new(request.operation, self.canonical.clone(), ())",
                "Request::new(Operation::Read, self.canonical.clone(), ())",
            ),
            (
                "resolved_replaces_canonical_with_alias",
                "pub fn bind<R>(self, canonical: Scope, resource: R) -> Resolved<T, R> {",
                "pub fn bind<R>(self, canonical: Scope, resource: R) -> Resolved<T, R> {\n"
                "        let _ = canonical; let canonical = self.path.clone();",
            ),
            (
                "resolved_returns_another_operation",
                "Some((request, self.canonical, self.resource))",
                "{ let mut changed = request; changed.operation = Operation::Read; "
                "Some((changed, self.canonical, self.resource)) }",
            ),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "InspectionRefinement",
        "prerequisites": ["ContractsTrace"],
        "test": (
            "evidence::tests::inspections_bind_all_context_fields_and_return_only_the_frozen_view"
        ),
        "mutations": [
            ("inspection_ignores_session", "self.session == current.session",
             "(self.session == current.session || true)"),
            ("inspection_ignores_generation", "self.sequence == current.sequence",
             "(self.sequence == current.sequence || true)"),
            ("inspection_ignores_acceptance", "self.ready == current.is_accepted()",
             "(self.ready == current.is_accepted() || true)"),
            ("inspection_ignores_accepted_view", "same_view(&self.accepted, &current.accepted())",
             "(same_view(&self.accepted, &current.accepted()) || true)"),
            ("inspection_ignores_displayed_view",
             "same_view(&self.displayed, &current.displayed())",
             "(same_view(&self.displayed, &current.displayed()) || true)"),
            ("inspection_captures_wrong_display", "displayed: self.displayed(),",
             "displayed: self.accepted(),"),
            (
                "inspection_hides_accepted_view",
                "pub fn accepted(&self) -> Option<&ContractView> {\n"
                "        match &self.accepted {\n            Some(view) => Some(view),",
                "pub fn accepted(&self) -> Option<&ContractView> {\n"
                "        match &self.accepted {\n"
                "            Some(view) => { let _ = view; None },",
            ),
            ("inspection_consume_bypasses_context", "if self.matches(current) {",
             "if self.matches(current) || true {"),
        ],
    },
    {
        "source": "crates/meden-core/src/file.rs",
        "package": "meden-core",
        "model": "Policy",
        "proof": "FileRefinement",
        "prerequisites": [
            "PolicyRefinement", "InvocationRefinement", "InvocationContextRefinement"
        ],
        "test": "file::tests::exact_operation_path_and_payload_determine_all_file_effects",
        "mutations": [
            (
                "edit_omits_read_authority",
                "Operation::Read | Operation::List | Operation::Edit | Operation::Snapshot",
                "Operation::Read | Operation::List | Operation::Snapshot",
            ),
            (
                "edit_omits_write_authority",
                "matches!(self.operation, Operation::Write | Operation::Edit)",
                "matches!(self.operation, Operation::Write)",
            ),
            (
                "file_read_declared_as_write",
                "capability: Capability::FsRead,",
                "capability: Capability::FsWrite,",
            ),
            (
                "file_read_scope_discarded",
                "capability: Capability::FsRead,\n                scope: Some(self.path.clone()),",
                "capability: Capability::FsRead,\n                scope: None,",
            ),
            (
                "prepared_file_path_substituted",
                "(self, requirements)",
                "{ let mut changed = self; changed.path = Scope::from(Vec::new()); "
                "(changed, requirements) }",
            ),
            (
                "worker_file_operation_substituted",
                "(self.operation, self.path, self.payload)",
                "(Operation::Read, self.path, self.payload)",
            ),
        ],
    },
    {
        "kernel": "policy",
        "model": "Policy",
        "proof": "PolicyRefinement",
        "prerequisites": ["GlobRefinement"],
        "test": (
            "policy::tests::every_requirement_keeps_its_refusal_"
            "and_a_later_allowance_cannot_reset_denial"
        ),
        "mutations": [
            (
                "mismatched_capability_granted",
                "if g.capability != r.capability {",
                "if g.capability != r.capability && false {",
            ),
            (
                "scoped_grant_authorizes_unscoped",
                "(Some(_), None) => false,",
                "(Some(_), None) => true,",
            ),
            (
                "explicit_policy_deny_ignored",
                "if denies.contains(&req.capability) {",
                "if denies.contains(&req.capability) && false {",
            ),
            (
                "later_allowance_resets_denial",
                "if refusal.is_some() {\n            allowed = false;\n        }",
                "allowed = refusal.is_none();",
            ),
            (
                "first_requirement_omitted",
                "let mut refusals = Vec::new();\n    let mut index = 0;",
                "let mut refusals = Vec::new();\n    let mut index = 1;",
            ),
            (
                "policy_refusals_swapped",
                "if denies.contains(&req.capability) {\n"
                "        Some(Refusal::Denied)\n"
                "    } else if !grants_ref(grants, req) {\n"
                "        Some(Refusal::NotGranted)\n    }",
                "if denies.contains(&req.capability) {\n"
                "        Some(Refusal::NotGranted)\n"
                "    } else if !grants_ref(grants, req) {\n"
                "        Some(Refusal::Denied)\n    }",
            ),
        ],
    },
    {
        "source": "crates/meden-core/src/glob.rs",
        "package": "meden-core",
        "model": "Policy",
        "proof": "GlobRefinement",
        "test": "glob::tests::all_small_unicode_scopes_match_the_recursive_language",
        "mutations": [
            ("star_requires_both_suffixes", "old || new_right", "old && new_right"),
            ("question_mark_loses_wildcard", "pc == '?'", "pc == '!'"),
            ("literal_reads_new_row", "old_right = old;", "old_right = value;"),
            (
                "empty_pattern_accepts_nonempty",
                "let mut row = vec![false; text.len()];",
                "let mut row = vec![true; text.len()];",
            ),
        ],
    },
    {
        "source": "crates/meden-core/src/invocation.rs",
        "package": "meden-core",
        "model": "Policy",
        "proof": "InvocationRefinement",
        "prerequisites": ["PolicyRefinement", "ConversationRefinement"],
        "test": (
            "invocation::tests::"
            "policy_notification_exact_payload_stop_and_consumption_share_one_controller"
        ),
        "mutations": [
            (
                "policy_admission_bypassed",
                "if allowed(&requirements, rules) {",
                "if allowed(&requirements, rules) || true {",
            ),
            (
                "preparation_refusal_replaced",
                "Err(refusal) => Err(refusal),",
                "Err(refusal) => { let _ = refusal; Err(policy_refusal) },",
            ),
            (
                "invocation_without_notification",
                "if !self.announced {",
                "if !self.announced && false {",
            ),
            (
                "repeated_notification_retains_invocation",
                "self.checked = None;",
                "let _ = &self.checked;",
            ),
            (
                "invocation_ignores_cancellation",
                "if cancelled {",
                "if cancelled && false {",
            ),
            (
                "invocation_ignores_host_stop",
                "} else if let Some(status) = host_stop {",
                "} else if let Some(status) = { let _ = host_stop; None } {",
            ),
            (
                "invocation_reclassifies_refusal",
                "Err(Error::refused(refusal))",
                "{ let _ = refusal; Err(Error::invalid()) }",
            ),
        ],
    },
    {
        "source": "crates/meden-agent/src/conversation.rs",
        "package": "meden-agent",
        "model": "Conversation",
        "proof": "InvocationRefinement",
        "prerequisites": ["PolicyRefinement", "ConversationRefinement"],
        "test": (
            "conversation::tests::dispatch_error_mapping_preserves_refusal_stop_and_cancellation"
        ),
        "mutations": [
            (
                "dispatch_refusal_becomes_tool_error",
                "Err(Some(refusal)) => ExecutionError::Refused(refusal.reason)",
                "Err(Some(refusal)) => ExecutionError::Tool(refusal.reason)",
            ),
            (
                "dispatch_cancellation_becomes_incomplete",
                "Ok(None) => ExecutionError::Stopped(Status::Cancelled)",
                "Ok(None) => ExecutionError::Stopped(Status::Incomplete)",
            ),
            (
                "dispatch_host_status_lost",
                "Ok(Some(status)) => ExecutionError::Stopped(status)",
                "Ok(Some(status)) => "
                "{ let _ = status; ExecutionError::Stopped(Status::Incomplete) }",
            ),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "DeliveryRefinement",
        "prerequisites": [
            "EvidenceRefinement",
            "CheckJobsRefinement",
            "CompletionRefinement",
            "ContractsRefinement",
        ],
        "test": "evidence::tests::delivery_cannot_bypass_contract_or_current_evidence",
        "mutations": [
            (
                "delivery_skips_current_evidence",
                "event == crate::export_control::ExportEvent::Approve && !self.permitted(current)",
                "event == crate::export_control::ExportEvent::Approve "
                "&& !self.permitted(current) && false",
            ),
            (
                "delivery_ignores_frozen_context",
                "Some(view) => view == *frozen,",
                "Some(view) => view == *frozen || true,",
            ),
            (
                "raw_export_bypasses_project_contract",
                "None => current.displayed().is_none(),",
                "None => current.displayed().is_none() || true,",
            ),
            (
                "delivery_accepts_incomplete_evidence",
                "current.eligible(frozen.revision, frozen.subject.inputs.clone())",
                "(current.eligible(frozen.revision, frozen.subject.inputs.clone()) || true)",
            ),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "ContractsRefinement",
        "prerequisites": ["EvidenceRefinement", "CheckJobsRefinement", "CompletionRefinement"],
        "test": (
            "evidence::tests::identical_reproposal_after_rejection_requires_a_fresh_host_review"
        ),
        "mutations": [
            (
                "reused_contract_revision",
                "self.sequence = next;\n        self.jobs.invalidate",
                "let _ = next;\n        self.jobs.invalidate",
            ),
            (
                "stale_contract_review",
                "Some(draft) => draft.revision == revision && draft.gate.review(origin),",
                "Some(draft) => { let _ = revision; draft.gate.review(origin) },",
            ),
            (
                "stale_contract_acceptance",
                "if draft.revision != revision || !draft.gate.accept(origin) {",
                "if (draft.revision != revision && false) || !draft.gate.accept(origin) {",
            ),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "ContractsRefinement",
        "prerequisites": ["EvidenceRefinement", "CheckJobsRefinement", "CompletionRefinement"],
        "test": (
            "evidence::tests::"
            "rejecting_a_replacement_preserves_accepted_evidence_but_never_its_old_job"
        ),
        "mutations": [
            (
                "pending_contract_eligible",
                "if self.draft.is_some() || self.jobs.is_active() {",
                "if self.jobs.is_active() {",
            ),
            (
                "rejected_contract_discards_accepted",
                "self.draft = None;\n        true\n    }\n\n    /// Observe changed inputs",
                "self.draft = None;\n        self.accepted = None;\n"
                "        true\n    }\n\n    /// Observe changed inputs",
            ),
        ],
    },
    {
        "kernel": "evidence",
        "model": "Evidence",
        "proof": "ContractsRefinement",
        "prerequisites": ["EvidenceRefinement", "CheckJobsRefinement", "CompletionRefinement"],
        "test": "evidence::tests::editing_then_restoring_inputs_does_not_restore_proof_coverage",
        "mutations": [
            (
                "edited_contract_evidence_revived",
                "accepted.gate.completed = 0;",
                "let _ = accepted.gate.completed;",
            ),
        ],
    },
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
        "kernel": "processor",
        "model": "Processor",
        "proof": "ProcessorRefinement",
        "test": "processor::tests::approval_scope_frozen_parameters_and_terminal_revocation",
        "mutations": [
            (
                "model_processor_approval",
                "origin != ProcessorOrigin::Host ||",
                "origin == ProcessorOrigin::Host ||",
            ),
            (
                "processor_grant_renewed",
                "self.phase != ProcessorPhase::Pending",
                "(self.phase != ProcessorPhase::Pending && false)",
            ),
            (
                "processor_deadline_wraps",
                "now.checked_add(APPROVAL_LIFETIME_MILLIS)",
                "Some(now.wrapping_add(APPROVAL_LIFETIME_MILLIS))",
            ),
            (
                "pending_processor_dispatched",
                "if !self.is_authorized(session, now) {",
                "if !self.is_authorized(session, now) && self.phase != ProcessorPhase::Pending {",
            ),
            ("foreign_processor_session", "expected == session", "(expected == session || true)"),
            ("processor_deadline_inclusive", "now < self.expires", "now <= self.expires"),
            (
                "pending_processor_revocation_ignored",
                "self.phase = ProcessorPhase::Revoked;\n        true",
                "if self.phase == ProcessorPhase::Pending { return false; }\n"
                "        self.phase = ProcessorPhase::Revoked;\n        true",
            ),
            (
                "expired_processor_reopens",
                "self.phase = ProcessorPhase::Revoked;\n        }\n        if !self.is_authorized",
                "let _ = self.phase;\n        }\n        if !self.is_authorized",
            ),
            (
                "processor_endpoint_substituted",
                "endpoint: self.endpoint.clone(),",
                "endpoint: self.provider.clone(),",
            ),
            (
                "processor_credential_lost",
                "Some(key) => Some(key.clone()),",
                "Some(key) => { let _ = key; None },",
            ),
            (
                "local_processor_paths_substituted",
                "Self { engine, weights }",
                "Self { engine: weights, weights: engine }",
            ),
            (
                "wrong_local_resource_accepted",
                "self.parameters == *actual",
                "self.parameters == *actual || true",
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
        "model": "Evidence",
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
        "model": "Evidence",
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
        "source": "crates/meden-cli/src/tui/approval.rs",
        "package": "meden-cli",
        "model": "Approval",
        "proof": "ApprovalRefinement",
        "test": "tui::approval::tests::review_area_must_have_content_cells",
        "mutations": [
            (
                "hidden_review_approval",
                "if width <= 2 || height <= 2 {",
                "if (width <= 2 || height <= 2) && false {",
            ),
        ],
    },
    {
        "source": "crates/meden-cli/src/tui/approval.rs",
        "package": "meden-cli",
        "model": "Approval",
        "proof": "ApprovalRefinement",
        "test": "tui::approval::tests::complete_review_heading_must_fit",
        "mutations": [
            (
                "clipped_review_approval",
                "if width == 3 || rows == 0 || rows > height.saturating_sub(2) {",
                "if (width == 3 || rows == 0 || rows > height.saturating_sub(2)) && false {",
            ),
        ],
    },
    {
        "source": "crates/meden-runtime/src/verification/contract.rs",
        "package": "meden-runtime",
        "model": "Repair",
        "proof": "DecimalRefinement",
        "test": "verification::contract::tests::decimal_parser_is_exact_and_canonical",
        "mutations": [
            (
                "decimal_leading_zero",
                "return if bytes.len() == 1 { Some(0) } else { None };",
                "return if bytes.len() == 1 || true { Some(0) } else { None };",
            ),
            (
                "decimal_extra_digit",
                "if byte < b'0' || byte > b'9' {",
                "if byte < b'0' || byte > b':' {",
            ),
            (
                "decimal_overflow_boundary",
                "if value > (u64::MAX - digit) / 10 {",
                "if value >= (u64::MAX - digit) / 10 {",
            ),
            (
                "decimal_place_value",
                "value = value * 10 + digit;",
                "value = value * 9 + digit;",
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
            (
                "unaccepted_check",
                "if accepted && changed {\n                    if let Some(fresh) = inspection {",
                "if (accepted || true) && changed {\n"
                "                    if let Some(fresh) = inspection {",
            ),
            (
                "unchanged_check",
                "if accepted && changed {\n                    if let Some(fresh) = inspection {",
                "if accepted && (changed || true) {\n"
                "                    if let Some(fresh) = inspection {",
            ),
            (
                "budget_reuses_old_rejection",
                "!fresh && matches!(failure, Some(FailureKind::Unverified))",
                "(!fresh || true) && matches!(failure, Some(FailureKind::Unverified))",
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
            "proposal_repair_requires_a_failed_proposal_and_shares_the_turn_budget"
        ),
        "mutations": [
            (
                "unobserved_proposal_repaired",
                "!accepted && proposal_failed && inspection.is_some() {\n"
                "                    self.repair()",
                "!accepted && (proposal_failed || true) && inspection.is_some() {\n"
                "                    self.repair()",
            ),
            (
                "accepted_proposal_repaired",
                "!accepted && proposal_failed && inspection.is_some() {\n"
                "                    self.repair()",
                "(!accepted || true) && proposal_failed && inspection.is_some() {\n"
                "                    self.repair()",
            ),
            (
                "failed_conversation_restarts_proposal",
                "!accepted && proposal_failed && inspection.is_some() {\n"
                "                    self.repair()",
                "!accepted && proposal_failed && true {\n                    self.repair()",
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
        "test": "host_stop_before_model_request_prevents_context_disclosure",
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
                "let Some(next) = self.sequence.checked_add(1) else {\n"
                "            return None;",
                "let Some(next) = self.sequence.checked_add(0) else {\n"
                "            return None;",
            ),
            (
                "wrapped_check_job_id",
                "let Some(next) = self.sequence.checked_add(1) else {\n"
                "            return None;",
                "let Some(next) = Some(self.sequence.wrapping_add(1)) else {\n"
                "            return None;",
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
                    "no if-then-else conditions to split",
                    "maximum number of heartbeats",
                    "maximum recursion depth",
                    "stack overflow",
                    "Unknown constant",
                    "Unknown identifier",
                    "unknown module prefix",
                    "out of memory",
                    "memory allocation",
                    "failed to create thread",
                )
                if any(error.casefold() in output.casefold() for error in operational_errors):
                    raise RuntimeError(
                        f"{name}: operational proof failure does not count\n{output}"
                    )
                proof_errors = (
                    "unsolved goals",
                    "made no progress",
                    "omega could not prove the goal:",
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
