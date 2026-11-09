import MedenSpec.Contracts

/-! Accepted mathematical meaning is immutable between host acceptances.
Input/artifact bindings, proof coverage and job activity may change separately. -/
namespace MedenSpec.Contracts
set_option maxHeartbeats 1000000
open Evidence

private def scope (g : Evidence.Gate) : List Nat × List Nat × List Obligation :=
  (g.subject.session, g.subject.contract, g.obligations)

private def frozen (v : Versioned) : Nat × List Nat × List Nat × List Obligation :=
  (v.revision, scope v.gate)

def meaning (s : State) : Option (Nat × List Nat × List Nat × List Obligation) :=
  s.accepted.map frozen

def hostAcceptance : Action → Bool
  | .accept .host _ _ => true
  | _ => false

private theorem reviewOne_frozen (v : Versioned) (o : Origin) (revision : Nat) :
    frozen (reviewOne v o revision).2 = frozen v := by
  unfold reviewOne Evidence.review
  split_ifs <;> rfl

private theorem refresh_scope (g : Evidence.Gate) (o : Origin) (current : Subject) :
    scope (Evidence.refresh g o current).2 = scope g := by
  unfold Evidence.refresh
  split_ifs <;> simp_all [scope]

private theorem record_scope (g : Evidence.Gate) (o : Origin) (current : Subject) (r : Receipt) :
    scope (Evidence.record g o current r).2 = scope g := by
  unfold Evidence.record
  split_ifs <;> rfl

private theorem completion_scope (j : CheckJobs.State) (g : Evidence.Gate) (o : Origin)
    (t : CheckJobs.Ticket) (current : Subject) (r : Completion.Output) :
    scope (Completion.complete j g o t current r).2.2 = scope g := by
  unfold Completion.complete
  dsimp only
  split_ifs <;> simp only [record_scope, refresh_scope]

private theorem review_meaning (s : State) (o : Origin) (session : List Nat) (revision : Nat) :
    meaning (review s o session revision).2 = meaning s := by
  by_cases hs : s.session = session
  · cases hd : s.draft with
    | some d =>
      cases reviewOne d o revision; simp [review, hs, hd, meaning]
    | none =>
      cases ha : s.accepted with
      | none => simp [review, hs, hd, ha, meaning]
      | some a =>
        have hm := reviewOne_frozen a o revision
        cases h : reviewOne a o revision with
        | mk allowed next => simp [review, hs, hd, ha, h, meaning, ← hm]
  · simp [review, hs]

private theorem refresh_meaning (s : State) (o : Origin) (revision : Nat) (inputs : List Nat) :
    meaning (refresh s o revision inputs).2 = meaning s := by
  cases ha : s.accepted with
  | none => simp [refresh, ha]
  | some a =>
    have hm := refresh_scope a.gate o (withInputs a.gate inputs)
    cases h : Evidence.refresh a.gate o (withInputs a.gate inputs) with
    | mk allowed next =>
      simp only [h] at hm
      by_cases hr : a.revision = revision <;>
        simp [refresh, ha, hr, h, meaning, frozen, hm]

private theorem start_meaning (s : State) (o : Origin) (revision : Nat) (inputs : List Nat) :
    meaning (start s o revision inputs).2 = meaning s := by
  by_cases hs : o ≠ .controller ∨ s.draft.isSome = true
  · simp [start, hs]
  · cases ha : s.accepted with
    | none => simp [start, hs, ha]
    | some a =>
      have hm := refresh_scope a.gate o (withInputs a.gate inputs)
      cases h : Evidence.refresh a.gate o (withInputs a.gate inputs) with
      | mk allowed next =>
        simp only [h] at hm
        cases hj : CheckJobs.start (CheckJobs.invalidate s.jobs .controller).2 o next (withInputs a.gate inputs) with
        | mk issued jobs =>
          by_cases hr : a.revision = revision <;> cases allowed <;>
            simp [start, hs, ha, hr, h, hj, meaning, frozen, hm]

private theorem completion_meaning (s : State) (o : Origin) (revision : Nat) (t : CheckJobs.Ticket)
    (inputs : List Nat) (r : Completion.Output) :
    meaning (complete s o revision t inputs r).2 = meaning s := by
  cases hd : s.draft with
  | some d => simp [complete, hd]
  | none =>
    cases ha : s.accepted with
    | none => simp [complete, hd, ha]
    | some a =>
      have hm := completion_scope s.jobs a.gate o t (withInputs a.gate inputs) r
      cases h : Completion.complete s.jobs a.gate o t (withInputs a.gate inputs) r with
      | mk allowed rest =>
        rcases rest with ⟨jobs, gate⟩
        simp only [h] at hm
        by_cases hr : a.revision = revision <;>
          simp [complete, hd, ha, hr, h, meaning, frozen, hm]

theorem step_preserves_meaning (s : State) (a : Action) (h : hostAcceptance a = false) :
    meaning (step s a) = meaning s := by
  cases a with
  | propose o c obs => simp [step, meaning, propose_preserves_accepted]
  | review o session revision => exact review_meaning s o session revision
  | accept o session revision =>
    cases o <;> cases hd : s.draft <;>
      simp [hostAcceptance] at h
    all_goals simp [step, accept, hd, Evidence.accept]
  | reject o session revision => simp [step, meaning, reject_preserves_accepted]
  | refresh o revision inputs => exact refresh_meaning s o revision inputs
  | start o revision inputs => exact start_meaning s o revision inputs
  | invalidate o => simp [step, invalidate, meaning]
  | edit o =>
    by_cases ho : o = .controller <;> cases ha : s.accepted <;>
      simp [step, edit, meaning, ha, ho, frozen, scope]
  | abort o t => simp [step, abort, meaning]
  | complete o revision t inputs r => exact completion_meaning s o revision t inputs r

theorem run_preserves_meaning (s : State) (actions : List Action)
    (h : ∀ a ∈ actions, hostAcceptance a = false) : meaning (run s actions) = meaning s := by
  induction actions generalizing s with
  | nil => rfl
  | cons a rest ih =>
    have ha := h a (by simp)
    have ht : ∀ a ∈ rest, hostAcceptance a = false := by
      intro a hm
      exact h a (by simp [hm])
    exact (ih (step s a) ht).trans (step_preserves_meaning s a ha)

end MedenSpec.Contracts
