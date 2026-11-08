import MedenSpec.CheckJobs

/-! One controller transition joining issued jobs to exact evidence. Outputs
come from a validated checker adapter; activity is derived by the controller. -/
namespace MedenSpec.Completion
open Evidence

structure Output where
  subject : Subject
  position : Nat
  obligation : Obligation
  passed : Bool
  deriving DecidableEq

def receipt (r : Output) : Receipt :=
  ⟨r.subject, r.position, r.obligation, r.passed, true⟩

def complete (j : CheckJobs.State) (g : Gate) (o : Origin)
    (t : CheckJobs.Ticket) (current : Subject) (r : Output) :
    Bool × CheckJobs.State × Gate :=
  let finished := CheckJobs.finish j o g t current
  if finished.1 then
    let refreshed := Evidence.refresh g .controller r.subject
    if refreshed.1 then
      let recorded := Evidence.record refreshed.2 o r.subject (receipt r)
      (recorded.1, finished.2, recorded.2)
    else (false, finished.2, refreshed.2)
  else (false, finished.2, g)

theorem success_requires_job (j : CheckJobs.State) (g : Gate) (o : Origin)
    (t : CheckJobs.Ticket) (current : Subject) (r : Output)
    (h : (complete j g o t current r).1 = true) :
    (CheckJobs.finish j o g t current).1 = true := by
  by_cases hf : (CheckJobs.finish j o g t current).1 = true
  · exact hf
  · simp [complete, hf] at h

theorem success_corresponds (j : CheckJobs.State) (g : Gate) (o : Origin)
    (t : CheckJobs.Ticket) (current : Subject) (r : Output)
    (h : (complete j g o t current r).1 = true) :
    o = .checker ∧ j.active = some t ∧ t.subject = current ∧
    r.passed = true ∧ (complete j g o t current r).2.1.active = none ∧
    (complete j g o t current r).2.2.subject = r.subject := by
  have hf := success_requires_job j g o t current r h
  have hm : o = .checker ∧ CheckJobs.permitted j g current ∧
      j.active = some t ∧ t.subject = current := by
    by_cases hm : o = .checker ∧ CheckJobs.permitted j g current ∧
        j.active = some t ∧ t.subject = current
    · exact hm
    · simp [CheckJobs.finish, hm] at hf
  by_cases hr : (Evidence.refresh g .controller r.subject).1 = true
  · simp only [complete, hf, hr, ite_true] at h ⊢
    have hc := (Evidence.record_iff _ _ _ _).mp h
    refine ⟨hm.1, hm.2.2.1, hm.2.2.2, hc.2.2.2.2.2.1, ?_, ?_⟩
    · simp [CheckJobs.finish, hm]
    · rw [Evidence.record, ite_eq_left hc]
      exact hc.2.2.1
  · simp [complete, hf, hr] at h

end MedenSpec.Completion
