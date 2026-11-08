import MedenSpec
import MedenProof

/-!
Axiom audit. CI runs `lake env lean Audit.lean` and fails if the output
mentions `sorryAx` or any axiom outside the standard three
(`propext`, `Classical.choice`, `Quot.sound`).
-/

open MedenSpec

#print axioms Capability.ofString_toString
#print axioms Capability.toString_injective
#print axioms globMatch_star
#print axioms globMatch_literal
#print axioms Policy.allowed_iff
#print axioms Policy.deny_wins
#print axioms Policy.grants_monotone
#print axioms Policy.denies_antitone
#print axioms Transfer.accepted_iff
#print axioms Transfer.exact_request
#print axioms Transfer.accepted_spends
#print axioms Transfer.no_replay
#print axioms Transfer.rejection_preserves
#print axioms Transfer.no_changed_recipient
#print axioms Transfer.no_changed_content
#print axioms TransferRefinement.allowed_refines
#print axioms TransferRefinement.reserve_refines
#print axioms TransferRefinement.extracted_no_replay
#print axioms ExportControl.dispatch_iff
#print axioms ExportControl.at_most_one_dispatch
#print axioms ExportControlRefinement.step_refines
#print axioms ExportControlRefinement.trace_refines
#print axioms ExportControlRefinement.extracted_at_most_one
#print axioms MedenSpec.Evidence.record_iff
#print axioms MedenSpec.Evidence.model_cannot_record
#print axioms MedenSpec.Evidence.inactive_cannot_record
#print axioms MedenSpec.Evidence.stale_cannot_record
#print axioms MedenSpec.Evidence.no_replay
#print axioms MedenSpec.Evidence.no_kind_substitution
#print axioms MedenSpec.Evidence.changed_inputs_invalidate
#print axioms MedenSpec.Evidence.eligible_iff
#print axioms MedenSpec.Evidence.trace_covered
#print axioms MedenSpec.Evidence.eligible_requires_all_evidence
#print axioms MedenSpec.EvidenceRefinement.propose_refines
#print axioms MedenSpec.EvidenceRefinement.review_refines
#print axioms MedenSpec.EvidenceRefinement.accept_refines
#print axioms MedenSpec.EvidenceRefinement.is_accepted_refines
#print axioms MedenSpec.EvidenceRefinement.eligible_refines
#print axioms MedenSpec.EvidenceRefinement.refresh_refines
#print axioms MedenSpec.EvidenceRefinement.record_refines
#print axioms MedenSpec.EvidenceTrace.trace_refines
#print axioms MedenSpec.EvidenceTrace.extracted_requires_all_evidence
#print axioms MedenSpec.Dispatch.effect_iff
#print axioms MedenSpec.Dispatch.frozen_preserved
#print axioms MedenSpec.Dispatch.trace_frozen
#print axioms MedenSpec.Dispatch.at_most_one_effect
#print axioms MedenSpec.DispatchRefinement.new_refines
#print axioms MedenSpec.DispatchRefinement.request_refines
#print axioms MedenSpec.DispatchRefinement.phase_refines
#print axioms MedenSpec.DispatchRefinement.advance_refines
#print axioms MedenSpec.DispatchRefinement.trace_refines
#print axioms MedenSpec.DispatchRefinement.extracted_trace_safe
#print axioms MedenSpec.ApprovalRefinement.approves_refines
#print axioms MedenSpec.ApprovalRefinement.paste_cannot_approve
#print axioms MedenSpec.ApprovalRefinement.modified_key_cannot_approve
#print axioms MedenSpec.ApprovalRefinement.nonpress_cannot_approve
#print axioms MedenSpec.RepairRefinement.begin_refines
#print axioms MedenSpec.RepairRefinement.advance_refines
#print axioms MedenSpec.RepairRefinement.inspection_requires_acceptance
#print axioms MedenSpec.RepairRefinement.budget_gets_fresh_check
#print axioms MedenSpec.RepairRefinement.inspection_error
#print axioms MedenSpec.RepairTrace.trace_refines
#print axioms MedenSpec.RepairTrace.extracted_conversation_budget
#print axioms MedenSpec.RepairTrace.extracted_stopped
#print axioms MedenSpec.CheckFailureRefinement.repairable_refines
#print axioms MedenSpec.CheckFailureRefinement.verification_stops_refines
#print axioms MedenSpec.CheckFailureRefinement.operational_stops
#print axioms MedenSpec.CheckFailureRefinement.failed_worker_refines
#print axioms MedenSpec.CheckFailureRefinement.truncated_is_inconclusive
#print axioms MedenSpec.CheckJobsRefinement.new_refines
#print axioms MedenSpec.CheckJobsRefinement.start_refines
#print axioms MedenSpec.CheckJobsRefinement.invalidate_refines
#print axioms MedenSpec.CheckJobsRefinement.abort_refines
#print axioms MedenSpec.CheckJobsRefinement.finish_refines
#print axioms MedenSpec.CheckJobsRefinement.is_active_refines
#print axioms MedenSpec.CheckJobsTrace.trace_refines
#print axioms MedenSpec.CheckJobsTrace.extracted_no_late_result
#print axioms MedenSpec.CheckJobsTrace.extracted_result_was_issued
#print axioms MedenSpec.CompletionRefinement.complete_refines
#print axioms MedenSpec.CompletionRefinement.success_requires_current_job
#print axioms MedenSpec.CompletionRefinement.model_cannot_complete
#print axioms MedenSpec.CheckJobsTrace.extracted_completion_was_issued
