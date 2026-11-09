import MedenSpec.Conversation

namespace MedenSpec.Invocation
universe u
variable {α : Type u}

inductive Rule where
  | unknownTool | invalidArguments | missingArgument | policy
  deriving DecidableEq

structure Refusal where
  rule : Rule
  reason : String
  deriving DecidableEq

inductive Notice where
  | allowed | refused (reason : Refusal) | invalid
  deriving DecidableEq

structure State (α : Type u) where
  queued : Option (Except Refusal α)
  announced : Bool

def initial (checked : Except Refusal α) : State α := ⟨some checked, false⟩

def announce (s : State α) : Notice × State α :=
  if s.announced then (.invalid, {s with queued := none})
  else
    let next := {s with announced := true}
    match s.queued with
    | some (.ok _) => (.allowed, next)
    | some (.error refusal) => (.refused refusal, next)
    | none => (.invalid, next)

def dispatch (s : State α) (cancelled : Bool) (stop : Option Conversation.Status) :
    Except Conversation.Check α × State α :=
  let next := {s with queued := none}
  if s.announced then
    match s.queued with
    | some (.ok request) =>
      match (if cancelled then some .cancelled else stop) with
      | none => (.ok request, next)
      | some status => (.error (.stopped status), next)
    | some (.error refusal) => (.error (.refused refusal.reason), next)
    | none => (.error (.stopped .incomplete), next)
  else (.error (.stopped .incomplete), next)

inductive Action where
  | announce | dispatch (cancelled : Bool) (stop : Option Conversation.Status)

inductive Event (α : Type u) where
  | notice (value : Notice) | resolved (value : Except Conversation.Check α)

def step (s : State α) : Action → Event α × State α
  | .announce => let out := announce s; (.notice out.1, out.2)
  | .dispatch cancelled stop => let out := dispatch s cancelled stop; (.resolved out.1, out.2)

def payloads : Event α → List α
  | .resolved (.ok value) => [value] | _ => []

def notifications : Event α → Nat
  | .notice .allowed => 1 | _ => 0

def available (s : State α) : List α :=
  match s.queued with
  | some (.ok value) => [value] | _ => []

def credit (s : State α) : Nat := if s.announced then (available s).length else 0

structure Trace (α : Type u) where
  state : State α
  payloads : List α
  notifications : Nat

def run (s : State α) : List Action → Trace α
  | [] => ⟨s, [], 0⟩
  | action :: rest =>
    let (event, next) := step s action
    let out := run next rest
    ⟨out.state, payloads event ++ out.payloads, notifications event + out.notifications⟩

theorem dispatch_iff (s : State α) (cancelled : Bool) (stop : Option Conversation.Status)
    (request : α) :
    (dispatch s cancelled stop).1 = .ok request ↔
      s.announced = true ∧ s.queued = some (.ok request) ∧ cancelled = false ∧ stop = none := by
  rcases s with ⟨queued, announced⟩
  cases announced <;> cases queued with
  | none => simp [dispatch]
  | some result => cases result <;> cases cancelled <;> cases stop <;> simp [dispatch]

theorem dispatch_consumes (s : State α) (cancelled : Bool) (stop : Option Conversation.Status) :
    (dispatch s cancelled stop).2.queued = none := by
  rcases s with ⟨queued, announced⟩
  cases announced <;> cases queued with
  | none => simp [dispatch]
  | some result => cases result <;> cases cancelled <;> cases stop <;> simp [dispatch]

private theorem step_exact (s : State α) (action : Action) :
    List.Sublist (payloads (step s action).1 ++ available (step s action).2) (available s) := by
  rcases s with ⟨queued, announced⟩
  cases action with
  | announce =>
    cases announced <;> cases queued with
    | none => simp [step, announce, available, payloads]
    | some result => cases result <;> simp [step, announce, available, payloads]
  | dispatch cancelled stop =>
    cases announced <;> cases queued with
    | none => simp [step, dispatch, available, payloads]
    | some result => cases result <;> cases cancelled <;> cases stop <;>
        simp [step, dispatch, available, payloads]

private theorem step_budget (s : State α) (action : Action) :
    (payloads (step s action).1).length + credit (step s action).2 ≤
      credit s + notifications (step s action).1 := by
  rcases s with ⟨queued, announced⟩
  cases action with
  | announce =>
    cases announced <;> cases queued with
    | none => simp [step, announce, available, payloads, credit, notifications]
    | some result => cases result <;> simp [step, announce, available, payloads, credit, notifications]
  | dispatch cancelled stop =>
    cases announced <;> cases queued with
    | none => simp [step, dispatch, available, payloads, credit, notifications]
    | some result => cases result <;> cases cancelled <;> cases stop <;>
        simp [step, dispatch, available, payloads, credit, notifications]

theorem trace_exact (s : State α) (actions : List Action) :
    List.Sublist ((run s actions).payloads ++ available (run s actions).state) (available s) := by
  induction actions generalizing s with
  | nil => simp [run]
  | cons action rest ih =>
    rcases h : step s action with ⟨event, next⟩
    have hs := step_exact s action
    rw [h] at hs
    have hr := List.Sublist.append (List.Sublist.refl (payloads event)) (ih next)
    simpa [run, h, List.append_assoc] using hr.trans hs

theorem trace_budget (s : State α) (actions : List Action) :
    (run s actions).payloads.length + credit (run s actions).state ≤
      credit s + (run s actions).notifications := by
  induction actions generalizing s with
  | nil => simp [run]
  | cons action rest ih =>
    rcases h : step s action with ⟨event, next⟩
    have hs := step_budget s action
    rw [h] at hs
    have hr := ih next
    simp only [run, h, List.length_append]
    dsimp only at hs ⊢
    omega

theorem trace_safe (checked : Except Refusal α) (actions : List Action) :
    let out := run (initial checked) actions
    out.payloads.length ≤ 1 ∧ out.payloads.length ≤ out.notifications ∧
      ∀ request ∈ out.payloads, checked = .ok request := by
  dsimp only
  have he := trace_exact (initial checked) actions
  have hl := he.length_le
  have hb := trace_budget (initial checked) actions
  have hc : credit (initial checked) = 0 := rfl
  rw [hc, Nat.zero_add] at hb
  have ha : (available (initial checked)).length ≤ 1 := by
    cases checked <;> simp [available, initial]
  simp only [List.length_append] at hl
  refine ⟨?_, by omega, ?_⟩
  · omega
  · intro request member
    have hm := he.subset (List.mem_append_left _ member)
    cases checked with
    | error reason => simp [available, initial] at hm
    | ok original =>
      have h : request = original := by simpa [available, initial] using hm
      subst request
      rfl

end MedenSpec.Invocation
