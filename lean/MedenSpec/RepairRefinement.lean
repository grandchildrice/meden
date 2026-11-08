import MedenSpec.Generated.Repair

/-! Check scheduling over actual conversation outcomes. Evidence acceptance,
the I/O loop, source freshness, flag attribution and atomics are separate obligations. -/
namespace MedenSpec.Repair

inductive Turn where
  | done | failed | budget | cancelled | review
  deriving DecidableEq

/-- Accepted changed work needs a check after a normal or budget stop.
Review, failure and cancellation never authorize one. -/
abbrev required (turn : Turn) (accepted changed cancelled : Bool) : Prop :=
  accepted = true ∧ changed = true ∧ cancelled = false ∧
    (turn = .done ∨ turn = .budget)

end MedenSpec.Repair

namespace MedenSpec.RepairRefinement
open Aeneas Aeneas.Std Result
open MedenRepairImpl.meden_agent.conversation

def turn : Status → Repair.Turn
  | .Done _ => .done | .Failed _ => .failed | .Incomplete => .budget
  | .Cancelled => .cancelled | .NeedsReview => .review

theorem needs_verification_refines (s : Status) (accepted changed cancelled : Bool) :
    MedenRepairImpl.session.needs_verification s accepted changed cancelled =
      ok (decide (Repair.required (turn s) accepted changed cancelled)) := by
  cases s <;> cases accepted <;> cases changed <;> cases cancelled <;> rfl

theorem budget_is_checked :
    MedenRepairImpl.session.needs_verification .Incomplete true true false = ok true := by
  rfl

theorem checking_requires_authority (s : Status) (accepted changed cancelled : Bool)
    (h : MedenRepairImpl.session.needs_verification s accepted changed cancelled = ok true) :
    accepted = true ∧ changed = true ∧ cancelled = false := by
  cases s <;> cases accepted <;> cases changed <;> cases cancelled <;>
    simp_all [MedenRepairImpl.session.needs_verification]

theorem terminal_does_not_check (text : String) (accepted changed cancelled : Bool) :
    MedenRepairImpl.session.needs_verification (.Failed text) accepted changed cancelled = ok false ∧
    MedenRepairImpl.session.needs_verification .Cancelled accepted changed cancelled = ok false ∧
    MedenRepairImpl.session.needs_verification .NeedsReview accepted changed cancelled = ok false := by
  cases accepted <;> cases changed <;> cases cancelled <;>
    simp [MedenRepairImpl.session.needs_verification]

end MedenSpec.RepairRefinement
