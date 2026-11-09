import MedenSpec.Invocation
import MedenSpec.ConversationRefinement
import MedenSpec.PolicyRefinement

namespace MedenSpec.InvocationRefinement
open Aeneas Aeneas.Std Result MedenConversationImpl
open MedenPolicyImpl.invocation (Attempt)

universe u
abbrev State (α : Type u) := Attempt α invocation.Refusal
variable {α : Type u}

def rule : invocation.Rule → Invocation.Rule
  | .UnknownTool => .unknownTool | .InvalidArguments => .invalidArguments
  | .MissingArgument => .missingArgument | .Policy => .policy

def refusal (r : invocation.Refusal) : Invocation.Refusal := ⟨rule r.rule, r.reason⟩

def preparation : core.result.Result α invocation.Refusal → Except Invocation.Refusal α
  | .Ok request => .ok request | .Err reason => .error (refusal reason)

def state (s : State α) : Invocation.State α := ⟨s.checked.map preparation, s.announced⟩

def notice : MedenPolicyImpl.invocation.Notice invocation.Refusal → Invocation.Notice
  | .Allowed => .allowed | .Refused reason => .refused (refusal reason) | .Invalid => .invalid

def fault : MedenPolicyImpl.invocation.Error invocation.Refusal conversation.Status → Conversation.Check
  | .Err (some reason) => .refused reason.reason
  | .Ok none => .stopped .cancelled
  | .Ok (some value) => .stopped (ConversationRefinement.status value)
  | .Err none => .stopped .incomplete

def resolution : core.result.Result α (MedenPolicyImpl.invocation.Error invocation.Refusal conversation.Status) →
    Except Conversation.Check α
  | .Ok request => .ok request | .Err error => .error (fault error)

theorem execution_error_refines
    (error : MedenPolicyImpl.invocation.Error invocation.Refusal conversation.Status) :
    (do let value ← conversation.execution_error error
        ok (ConversationRefinement.check (.Err value))) = ok (fault error) := by
  cases error with
  | Ok stop => cases stop <;>
      simp [conversation.execution_error, meden_core.invocation.Error.into_parts,
        ConversationRefinement.check, ConversationRefinement.status, fault]
  | Err reason => cases reason <;>
      simp [conversation.execution_error, meden_core.invocation.Error.into_parts,
        ConversationRefinement.check, ConversationRefinement.status, fault]

def admission (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal) : Except Invocation.Refusal α :=
  match checked with
  | .Err reason => .error (refusal reason)
  | .Ok (request, requirements) =>
    if (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement)
    then .ok request else .error (refusal policy_refusal)

theorem new_refines
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal) :
    (do let s ← Attempt.new checked rules policy_refusal; ok (state s)) =
      ok (Invocation.initial (admission checked rules policy_refusal)) := by
  cases checked with
  | Err reason => simp [Attempt.new, state, admission, preparation, Invocation.initial]
  | Ok value =>
    rcases value with ⟨request, requirements⟩
    by_cases h : (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement) = true
    all_goals simp [Attempt.new, PolicyRefinement.owned_allowed_refines, alloc.vec.Vec.deref,
      state, admission, preparation, Invocation.initial, h]

theorem announce_refines (s : State α) :
    (do let (n, next) ← Attempt.announce s; ok (notice n, state next)) =
      ok (Invocation.announce (state s)) := by
  rcases s with ⟨checked, announced⟩
  cases announced <;> cases checked with
  | none => simp [Attempt.announce, state, notice, Invocation.announce]
  | some result => cases result <;> simp [Attempt.announce, state, notice, preparation, Invocation.announce]

theorem dispatch_refines (s : State α) (cancelled : Bool) (stop : Option conversation.Status) :
    (do let (r, next) ← Attempt.dispatch s cancelled stop; ok (resolution r, state next)) =
      ok (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)) := by
  rcases s with ⟨checked, announced⟩
  cases announced <;> cases checked with
  | none => simp [Attempt.dispatch, core.option.Option.take, state, resolution, fault,
      MedenPolicyImpl.invocation.Error.invalid,
      ConversationRefinement.status, Invocation.dispatch]
  | some result => cases result <;> cases cancelled <;> cases stop <;>
      simp [Attempt.dispatch, core.option.Option.take, state,
        MedenPolicyImpl.invocation.Error.invalid, MedenPolicyImpl.invocation.Error.cancelled,
        MedenPolicyImpl.invocation.Error.stopped, MedenPolicyImpl.invocation.Error.refused,
        preparation, resolution, fault, refusal, ConversationRefinement.status,
        Invocation.dispatch]

inductive Action where
  | announce | dispatch (cancelled : Bool) (stop : Option conversation.Status)

def action : Action → Invocation.Action
  | .announce => .announce
  | .dispatch cancelled stop => .dispatch cancelled (stop.map ConversationRefinement.status)

def one (s : State α) : Action → Result (Invocation.Event α × State α)
  | .announce => do
    let (n, next) ← Attempt.announce s
    ok (.notice (notice n), next)
  | .dispatch cancelled stop => do
    let (r, next) ← Attempt.dispatch s cancelled stop
    ok (.resolved (resolution r), next)

theorem one_refines (s : State α) (a : Action) :
    (do let (event, next) ← one s a; ok (event, state next)) =
      ok (Invocation.step (state s) (action a)) := by
  cases a with
  | announce =>
    have h := announce_refines s
    cases hr : Attempt.announce s with
    | ret out =>
      rcases out with ⟨n, next⟩
      simp only [hr, bind_ok] at h
      change ok (notice n, state next) = ok (Invocation.announce (state s)) at h
      simp only [Result.ok.injEq] at h
      simp [one, hr, action, Invocation.step, ← h]
    | vis effect continuation => simp [hr] at h
    | div => simp [hr] at h
  | dispatch cancelled stop =>
    have h := dispatch_refines s cancelled stop
    cases hr : Attempt.dispatch s cancelled stop with
    | ret out =>
      rcases out with ⟨r, next⟩
      simp only [hr, bind_ok] at h
      change ok (resolution r, state next) =
        ok (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)) at h
      simp only [Result.ok.injEq] at h
      simp [one, hr, action, Invocation.step, ← h]
    | vis effect continuation => simp [hr] at h
    | div => simp [hr] at h

def runExtracted (s : State α) : List Action → Result (State α × List α × Nat)
  | [] => ok (s, [], 0)
  | a :: rest => do
    let (event, next) ← one s a
    let out ← runExtracted next rest
    ok (out.1, Invocation.payloads event ++ out.2.1, Invocation.notifications event + out.2.2)

private theorem one_success (s : State α) (a : Action) :
    ∃ event next, one s a = ok (event, next) ∧
      (event, state next) = Invocation.step (state s) (action a) := by
  have h := one_refines s a
  cases hr : one s a with
  | ret out =>
    rcases out with ⟨event, next⟩
    exact ⟨event, next, rfl, by simpa [hr] using h⟩
  | vis effect continuation => simp [hr] at h
  | div => simp [hr] at h

theorem trace_refines (s : State α) (actions : List Action) :
    (do let out ← runExtracted s actions;
        ok (Invocation.Trace.mk (state out.1) out.2.1 out.2.2)) =
      ok (Invocation.run (state s) (actions.map action)) := by
  induction actions generalizing s with
  | nil => simp [runExtracted, Invocation.run]
  | cons a rest ih =>
    obtain ⟨event, next, hr, hs⟩ := one_success s a
    have hh := ih next
    cases ho : runExtracted next rest with
    | ret out =>
      simp only [ho, bind_ok, Result.ok.injEq] at hh
      simp [runExtracted, hr, ho, Invocation.run, ← hs, ← hh]
    | vis effect continuation => simp [ho] at hh
    | div => simp [ho] at hh

theorem admission_iff
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal) (request : α) :
    admission checked rules policy_refusal = .ok request ↔
      ∃ requirements, checked = .Ok (request, requirements) ∧
        (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement) = true := by
  cases checked with
  | Err reason => simp [admission]
  | Ok value =>
    rcases value with ⟨original, requirements⟩
    cases h : (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement) <;>
      simp [admission, h]

theorem extracted_safe
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal) (initial final : State α)
    (actions : List Action) (requests : List α) (count : Nat)
    (hi : Attempt.new checked rules policy_refusal = ok initial)
    (hr : runExtracted initial actions = ok (final, requests, count)) :
    requests.length ≤ 1 ∧ requests.length ≤ count ∧
      ∀ request ∈ requests, ∃ requirements, checked = .Ok (request, requirements) ∧
        (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement) = true := by
  have hn := new_refines checked rules policy_refusal
  simp only [hi, bind_ok, Result.ok.injEq] at hn
  have ht := trace_refines initial actions
  simp only [hr, bind_ok, Result.ok.injEq] at ht
  have hs := Invocation.trace_safe (admission checked rules policy_refusal) (actions.map action)
  dsimp only at hs
  rw [← hn, ← ht] at hs
  refine ⟨hs.1, hs.2.1, ?_⟩
  intro request member
  exact (admission_iff checked rules policy_refusal request).mp (hs.2.2 request member)

theorem extracted_policy_refusal (request : α)
    (requirements : alloc.vec.Vec MedenPolicyImpl.capability.Requirement)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal)
    (initial final : State α) (actions : List Action) (requests : List α) (count : Nat)
    (denied : (PolicyRefinement.owned rules).allowed
      (requirements.val.map PolicyRefinement.requirement) = false)
    (hi : Attempt.new (.Ok (request, requirements)) rules policy_refusal = ok initial)
    (hr : runExtracted initial actions = ok (final, requests, count)) : requests = [] := by
  have hs := extracted_safe _ rules policy_refusal initial final actions requests count hi hr
  cases requests with
  | nil => rfl
  | cons value rest =>
    obtain ⟨actual, same, granted⟩ := hs.2.2 value (List.mem_cons_self ..)
    cases same
    simp [denied] at granted

theorem extracted_prepare_refusal (reason : invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal)
    (initial final : State α) (actions : List Action) (requests : List α) (count : Nat)
    (hi : Attempt.new (.Err reason) rules policy_refusal = ok initial)
    (hr : runExtracted initial actions = ok (final, requests, count)) : requests = [] := by
  have hs := extracted_safe _ rules policy_refusal initial final actions requests count hi hr
  cases requests with
  | nil => rfl
  | cons value rest =>
    obtain ⟨actual, same, _⟩ := hs.2.2 value (List.mem_cons_self ..)
    cases same

private theorem queued_ok_iff (s : State α) (request : α) :
    (state s).queued = some (.ok request) ↔ s.checked = some (.Ok request) := by
  cases h : s.checked with
  | none => simp [state, h]
  | some result => cases result <;> simp [state, h, preparation]

theorem extracted_dispatch_condition (s next : State α) (request : α)
    (cancelled : Bool) (stop : Option conversation.Status)
    (hr : Attempt.dispatch s cancelled stop = ok (.Ok request, next)) :
    s.announced = true ∧ s.checked = some (.Ok request) ∧ cancelled = false ∧ stop = none := by
  have h := dispatch_refines s cancelled stop
  simp only [hr, bind_ok] at h
  change ok (Except.ok request, state next) =
    ok (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)) at h
  simp only [Result.ok.injEq] at h
  have he : (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)).1 =
      .ok request := (congrArg Prod.fst h).symm
  have hc := (Invocation.dispatch_iff _ _ _ _).mp he
  rw [queued_ok_iff] at hc
  simpa [state] using hc

end MedenSpec.InvocationRefinement
