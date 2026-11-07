import MedenSpec

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
#print axioms tier_not_proved
#print axioms tier_lean_only
#print axioms tier_foreign_unapproved
#print axioms tier_foreign_approved
#print axioms verified_implies_lean_only
#print axioms tested_never_verifies
#print axioms Readers.meet_allows
#print axioms Readers.anyone_meet
#print axioms Label.join_trusted
#print axioms Label.join_untrusted_left
#print axioms Label.join_untrusted_right
#print axioms Label.join_allows
#print axioms Label.mem_join_left
#print axioms Label.mem_join_right
#print axioms untrusted_control_refused
#print axioms unreadable_not_disclosed
#print axioms integrity_cons
