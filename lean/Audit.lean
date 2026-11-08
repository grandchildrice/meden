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
