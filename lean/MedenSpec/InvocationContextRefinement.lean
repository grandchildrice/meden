import MedenSpec.InvocationRefinement

/-! Preserve admission policy as part of the exact one-use invocation payload. -/
namespace MedenSpec.InvocationContextRefinement
open Aeneas Aeneas.Std Result MedenConversationImpl
open MedenPolicyImpl.invocation (Attempt Call)

universe u
variable {α : Type u}

def bound
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) :
    core.result.Result (Call α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal :=
  match checked with
  | .Ok (request, requirements) => .Ok (⟨request, rules⟩, requirements)
  | .Err reason => .Err reason

private theorem with_policy_is_new
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal) :
    Attempt.with_policy checked rules policy_refusal = Attempt.new (bound checked rules) rules policy_refusal := by
  cases checked with
  | Ok value => rcases value with ⟨request, requirements⟩; rfl
  | Err reason => rfl

theorem constructor_refines
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal) :
    (do let state ← Attempt.with_policy checked rules policy_refusal;
        ok (InvocationRefinement.state state)) =
      ok (Invocation.initial (InvocationRefinement.admission (bound checked rules) rules policy_refusal)) := by
  rw [with_policy_is_new]
  exact InvocationRefinement.new_refines _ _ _

theorem consume_refines (call : Call α) :
    Call.into_parts call = ok (call.request, call.rules) := by
  rfl

private theorem bound_iff
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (call : Call α)
    (requirements : alloc.vec.Vec MedenPolicyImpl.capability.Requirement) :
    bound checked rules = .Ok (call, requirements) ↔
      checked = .Ok (call.request, requirements) ∧ call.rules = rules := by
  cases checked with
  | Err reason => simp [bound]
  | Ok value =>
    rcases value with ⟨request, declared⟩
    rcases call with ⟨original, context⟩
    simp [bound, eq_comm, and_assoc, and_left_comm, and_comm]

theorem admission_iff
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal) (call : Call α) :
    InvocationRefinement.admission (bound checked rules) rules policy_refusal = .ok call ↔
      ∃ requirements, checked = .Ok (call.request, requirements) ∧ call.rules = rules ∧
        (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement) = true := by
  rw [InvocationRefinement.admission_iff]
  simp only [bound_iff, and_assoc]

theorem trace_safe
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal)
    (initial final : InvocationRefinement.State (Call α))
    (actions : List InvocationRefinement.Action) (calls : List (Call α)) (count : Nat)
    (hi : Attempt.with_policy checked rules policy_refusal = ok initial)
    (hr : InvocationRefinement.runExtracted initial actions = ok (final, calls, count)) :
    calls.length ≤ 1 ∧ calls.length ≤ count ∧
      ∀ call ∈ calls, ∃ requirements,
        checked = .Ok (call.request, requirements) ∧ call.rules = rules ∧
          (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement) = true := by
  rw [with_policy_is_new] at hi
  have safe := InvocationRefinement.extracted_safe _ rules policy_refusal initial final actions calls count hi hr
  refine ⟨safe.1, safe.2.1, ?_⟩
  intro call member
  obtain ⟨requirements, checked_bound, permitted⟩ := safe.2.2 call member
  obtain ⟨original, context⟩ := (bound_iff checked rules call requirements).mp checked_bound
  exact ⟨requirements, original, context, permitted⟩

theorem trace_context
    (checked : core.result.Result (α × alloc.vec.Vec MedenPolicyImpl.capability.Requirement) invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal)
    (initial final : InvocationRefinement.State (Call α))
    (actions : List InvocationRefinement.Action) (calls : List (Call α)) (count : Nat)
    (hi : Attempt.with_policy checked rules policy_refusal = ok initial)
    (hr : InvocationRefinement.runExtracted initial actions = ok (final, calls, count)) :
    ∀ call ∈ calls, Call.into_parts call = ok (call.request, rules) := by
  have safe := trace_safe checked rules policy_refusal initial final actions calls count hi hr
  intro call member
  obtain ⟨_, _, context, _⟩ := safe.2.2 call member
  simpa only [context] using consume_refines call

theorem policy_refusal_empty (request : α)
    (requirements : alloc.vec.Vec MedenPolicyImpl.capability.Requirement)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal)
    (initial final : InvocationRefinement.State (Call α))
    (actions : List InvocationRefinement.Action) (calls : List (Call α)) (count : Nat)
    (denied : (PolicyRefinement.owned rules).allowed (requirements.val.map PolicyRefinement.requirement) = false)
    (hi : Attempt.with_policy (.Ok (request, requirements)) rules policy_refusal = ok initial)
    (hr : InvocationRefinement.runExtracted initial actions = ok (final, calls, count)) : calls = [] := by
  rw [with_policy_is_new] at hi
  change Attempt.new (.Ok ((⟨request, rules⟩ : Call α), requirements)) rules policy_refusal = ok initial at hi
  exact InvocationRefinement.extracted_policy_refusal (⟨request, rules⟩ : Call α) requirements rules policy_refusal
    initial final actions calls count denied hi hr

theorem prepare_refused (reason : invocation.Refusal)
    (rules : MedenPolicyImpl.policy.Policy) (policy_refusal : invocation.Refusal)
    (initial final : InvocationRefinement.State (Call α))
    (actions : List InvocationRefinement.Action) (calls : List (Call α)) (count : Nat)
    (hi : Attempt.with_policy (.Err reason) rules policy_refusal = ok initial)
    (hr : InvocationRefinement.runExtracted initial actions = ok (final, calls, count)) : calls = [] := by
  rw [with_policy_is_new] at hi
  exact InvocationRefinement.extracted_prepare_refusal reason rules policy_refusal initial final actions calls count hi hr

end MedenSpec.InvocationContextRefinement
