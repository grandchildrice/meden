import MedenSpec.Conversation

namespace MedenSpec.Conversation
set_option maxHeartbeats 4000000

theorem step_budget (maximum : Nat) (s : State) (i : Observation) :
    (step maximum s i).2.remaining + models (step maximum s i).1 ≤ s.remaining := by
  rcases s with ⟨remaining, attempt, phase, reply, next, result, stopped, faulted⟩
  rcases i with ⟨input, cancelled, hostStop⟩
  cases phase <;> cases input <;>
    simp only [step, advance, dispatch]
  all_goals repeat' (first
    | simp_all [latch, halt, announce, nextCall, recorded, record, reject, blocked, models]
    | split)
  all_goals omega

theorem trace_budget (maximum : Nat) (s : State) (inputs : List Observation) :
    (run maximum s inputs).1.remaining + (run maximum s inputs).2 ≤ s.remaining := by
  induction inputs generalizing s with
  | nil => simp [run]
  | cons i rest ih =>
    have hs := step_budget maximum s i
    have hr := ih (step maximum s i).2
    dsimp only [run]
    omega

theorem trace_halted (maximum : Nat) (s : State) (inputs : List Observation)
    (hs : s.phase = .halted) : run maximum s inputs = (s, 0) := by
  induction inputs with
  | nil => rfl
  | cons i rest ih => simp [run, step, advance, hs, models, ih]

theorem stop_persists (maximum : Nat) (s : State) (i : Observation) (reason : Status)
    (hs : s.stopped = some reason) : (step maximum s i).2.stopped = some reason := by
  rcases s with ⟨remaining, attempt, phase, reply, next, result, stopped, faulted⟩
  rcases i with ⟨input, cancelled, hostStop⟩
  cases phase <;> cases input <;>
    simp only [step, advance, dispatch]
  all_goals repeat' (first
    | simp_all [latch, halt, announce, nextCall, recorded, record, reject, blocked]
    | split)

theorem blocked_has_no_effects (maximum : Nat) (s : State) (i : Observation)
    (hs : blocked s = true) :
    (step maximum s i).1 ≠ .model ∧ (step maximum s i).1 ≠ .tool true := by
  rcases s with ⟨remaining, attempt, phase, reply, next, result, stopped, faulted⟩
  rcases i with ⟨input, cancelled, hostStop⟩
  cases phase <;> cases input <;>
    simp only [step, advance, dispatch]
  all_goals repeat' (first
    | simp_all [latch, halt, announce, nextCall, recorded, record, reject, blocked]
    | split)

def WellFormed (maximum : Nat) (s : State) : Prop :=
  s.reply.calls.length ≤ maximum ∧ s.next ≤ s.reply.calls.length ∧
    match s.phase with
    | .ready => s.reply.calls = [] ∧ s.result = none
    | .announced | .waiting | .halted => s.next = s.reply.calls.length ∧ s.result = none
    | .publishingAssistant => s.next = 0 ∧ s.result = none
    | .executing => s.next < s.reply.calls.length ∧ s.result = none
    | .publishingResult => s.next < s.reply.calls.length ∧ s.result.isSome = true

def BoundedInput (maximum : Nat) : Input → Prop
  | .reply value => value.calls.length ≤ maximum
  | _ => True

theorem initial_wellFormed (maximum groups : Nat) : WellFormed maximum (initial maximum groups) := by
  simp [WellFormed, initial]

private theorem latch_wellFormed (maximum : Nat) (s : State) (stop : Option Status)
    (hs : WellFormed maximum s) : WellFormed maximum (latch s stop) := by
  unfold latch
  split <;> simpa [WellFormed] using hs

private theorem dispatch_wellFormed (maximum : Nat) (s : State) (input : Input)
    (hs : WellFormed maximum s) (hi : BoundedInput maximum input) :
    WellFormed maximum (dispatch maximum s input).2 := by
  rcases s with ⟨remaining, attempt, phase, reply, next, result, stopped, faulted⟩
  cases phase <;> cases input <;>
    simp_all only [WellFormed, BoundedInput]
  all_goals simp only [dispatch]
  all_goals repeat' (first
    | simp_all [latch, halt, announce, nextCall, recorded, record, reject, blocked]
    | split)
  all_goals rcases hs with ⟨hbound, hnext, hphase, hresult⟩
  all_goals try simp_all
  all_goals omega

theorem step_wellFormed (maximum : Nat) (s : State) (i : Observation)
    (hs : WellFormed maximum s) (hi : BoundedInput maximum i.input) :
    WellFormed maximum (step maximum s i).2 := by
  unfold step advance
  split
  · exact hs
  · exact dispatch_wellFormed maximum _ i.input (latch_wellFormed maximum s _ hs) hi

theorem record_identity (maximum : Nat) (s : State) (i : Observation)
    (h : (step maximum s i).1 = .record) :
    (step maximum s i).2.reply = s.reply ∧ (step maximum s i).2.next = s.next ∧
      (step maximum s i).2.phase = .publishingResult := by
  rcases s with ⟨remaining, attempt, phase, reply, next, result, stopped, faulted⟩
  rcases i with ⟨input, cancelled, hostStop⟩
  cases phase <;> cases input <;>
    simp only [step, advance, dispatch] at *
  all_goals repeat' (first
    | simp_all [latch, halt, announce, nextCall, recorded, record, reject, blocked]
    | split)

end MedenSpec.Conversation
