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
