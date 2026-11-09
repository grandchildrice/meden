import MedenSpec.File
import MedenSpec.InvocationRefinement

namespace MedenSpec.FileRefinement
open Aeneas Aeneas.Std Result MedenPolicyImpl
universe u
variable {α : Type u}

def operation : file.Operation → File.Operation
  | .Read => .read | .List => .list | .Write => .write
  | .Edit => .edit | .Snapshot => .snapshot

def request (r : file.Request α) : File.Request α :=
  ⟨operation r.operation, PolicyRefinement.decoded r.path, r.payload⟩

theorem scope_clone (path : scope.Scope) :
    scope.Scope.Insts.CoreCloneClone.clone path = ok path := by
  have h := Slice.clone_spec (s := path.slice)
    (clone := Char.Insts.CoreCloneClone.clone) (by intros; rfl)
  unfold scope.Scope.Insts.CoreCloneClone.clone alloc.vec.CloneVec.clone
  cases hc : Slice.clone Char.Insts.CoreCloneClone.clone path.slice with
  | ret out =>
    have he : path.slice = out := by simpa [hc] using h
    simp [← he]
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

theorem new_refines (op : file.Operation) (path : scope.Scope) (payload : α) :
    (do let r ← file.Request.new op path payload; ok (request r)) =
      ok (File.Request.mk (operation op) (PolicyRefinement.decoded path) payload) := by
  simp [file.Request.new, request]

theorem prepare_refines (r : file.Request α) :
    (do let (prepared, requirements) ← file.Request.prepare r
        ok (prepared, requirements.val.map PolicyRefinement.requirement)) =
      ok (r, File.requirements (request r)) := by
  rcases r with ⟨op, path, payload⟩
  cases op <;>
    simp [file.Request.prepare, scope_clone, alloc.vec.Vec.push, alloc.vec.Vec.new,
      File.requirements, File.effects, request, operation, PolicyRefinement.requirement,
      PolicyRefinement.cap, U32.max, U32.numBits]

theorem consume_refines (r : file.Request α) :
    (do let (op, path, payload) ← file.Request.into_parts r
        ok (operation op, PolicyRefinement.decoded path, payload)) =
      ok ((request r).operation, (request r).path, (request r).payload) := by
  simp [file.Request.into_parts, request]

theorem prepare_preserves_request (r prepared : file.Request α)
    (requirements : alloc.vec.Vec capability.Requirement)
    (h : file.Request.prepare r = ok (prepared, requirements)) : prepared = r := by
  have hr := prepare_refines r
  simp [h] at hr
  exact hr.1

theorem prepared_allowed_iff (r prepared : file.Request α)
    (requirements : alloc.vec.Vec capability.Requirement) (p : policy.Policy)
    (h : file.Request.prepare r = ok (prepared, requirements)) :
    policy.allowed requirements.slice p =
      ok ((PolicyRefinement.owned p).allowed (File.requirements (request r))) := by
  have hr := prepare_refines r
  simp [h] at hr
  rw [PolicyRefinement.owned_allowed_refines]
  change ok ((PolicyRefinement.owned p).allowed
    (requirements.val.map PolicyRefinement.requirement)) = _
  rw [hr.2]

theorem admitted_trace_safe (r prepared : file.Request α)
    (requirements : alloc.vec.Vec capability.Requirement) (p : policy.Policy)
    (reason : MedenConversationImpl.invocation.Refusal)
    (initial final : InvocationRefinement.State (file.Request α))
    (actions : List InvocationRefinement.Action) (outputs : List (file.Request α)) (count : Nat)
    (hp : file.Request.prepare r = ok (prepared, requirements))
    (hi : invocation.Attempt.new (.Ok (prepared, requirements)) p reason = ok initial)
    (hr : InvocationRefinement.runExtracted initial actions = ok (final, outputs, count)) :
    outputs.length ≤ 1 ∧ outputs.length ≤ count ∧
      ∀ out ∈ outputs, out = r ∧
        (PolicyRefinement.owned p).allowed (File.requirements (request r)) = true := by
  have prepared_exact := prepare_refines r
  simp [hp] at prepared_exact
  have safe := InvocationRefinement.extracted_safe _ p reason initial final actions outputs count hi hr
  refine ⟨safe.1, safe.2.1, ?_⟩
  intro out member
  obtain ⟨actual, same, allowed⟩ := safe.2.2 out member
  cases same
  exact ⟨prepared_exact.1, by simpa only [prepared_exact.2] using allowed⟩

theorem denied_trace_empty (r prepared : file.Request α)
    (requirements : alloc.vec.Vec capability.Requirement) (p : policy.Policy)
    (reason : MedenConversationImpl.invocation.Refusal)
    (initial final : InvocationRefinement.State (file.Request α))
    (actions : List InvocationRefinement.Action) (outputs : List (file.Request α)) (count : Nat)
    (denied : (PolicyRefinement.owned p).allowed (File.requirements (request r)) = false)
    (hp : file.Request.prepare r = ok (prepared, requirements))
    (hi : invocation.Attempt.new (.Ok (prepared, requirements)) p reason = ok initial)
    (hr : InvocationRefinement.runExtracted initial actions = ok (final, outputs, count)) : outputs = [] := by
  have safe := admitted_trace_safe r prepared requirements p reason initial final actions outputs count hp hi hr
  cases outputs with
  | nil => rfl
  | cons out rest =>
    have granted := (safe.2.2 out (List.mem_cons_self ..)).2
    simp [denied] at granted

def resolved (r : file.Resolved α ρ) : File.Resolved α ρ :=
  ⟨request r.request, PolicyRefinement.decoded r.canonical, r.resource⟩

theorem bind_refines (r : file.Request α) (canonical : scope.Scope) (resource : ρ) :
    (do let bound ← file.Request.bind r canonical resource; ok (resolved bound)) =
      ok (File.Resolved.mk (request r) (PolicyRefinement.decoded canonical) resource) := by
  simp [file.Request.bind, resolved]

theorem resolved_consume_refines (r : file.Resolved α ρ) (p : policy.Policy) :
    file.Resolved.consume r p =
      ok (if File.resolvedAllowed (PolicyRefinement.owned p) (resolved r)
        then some (r.request, r.canonical, r.resource) else none) := by
  rcases r with ⟨⟨op, path, payload⟩, canonical, resource⟩
  cases op <;>
    simp [file.Resolved.consume, file.Request.prepare, file.Request.new,
      scope_clone, alloc.vec.Vec.push, alloc.vec.Vec.new, alloc.vec.Vec.deref,
      PolicyRefinement.owned_allowed_refines, PolicyRefinement.requirement, PolicyRefinement.cap,
      File.resolvedAllowed, File.canonicalRequest, File.requirements, File.effects,
      request, resolved, operation, U32.max, U32.numBits, ite_and]
  all_goals split_ifs <;> rfl

theorem extracted_resolved_iff (r : file.Resolved α ρ) (p : policy.Policy)
    (out : file.Request α × scope.Scope × ρ) :
    file.Resolved.consume r p = ok (some out) ↔
      out = (r.request, r.canonical, r.resource) ∧
      (PolicyRefinement.owned p).allowed (File.requirements (request r.request)) = true ∧
      (PolicyRefinement.owned p).allowed
        (File.requirements (File.canonicalRequest (resolved r))) = true := by
  rw [resolved_consume_refines]
  simp only [Result.ok.injEq]
  by_cases h : File.resolvedAllowed (PolicyRefinement.owned p) (resolved r) = true
  · rw [ite_eq_left h]
    have both : (PolicyRefinement.owned p).allowed (File.requirements (request r.request)) = true ∧
        (PolicyRefinement.owned p).allowed (File.requirements (File.canonicalRequest (resolved r))) = true := by
      have both := h
      simp only [File.resolvedAllowed, Bool.and_eq_true] at both
      change (PolicyRefinement.owned p).allowed (File.requirements (request r.request)) = true ∧
        (PolicyRefinement.owned p).allowed (File.requirements (File.canonicalRequest (resolved r))) = true at both
      exact both
    constructor
    · intro same
      exact ⟨(Option.some.inj same).symm, both.1, both.2⟩
    · rintro ⟨same, _, _⟩
      exact congrArg some same.symm
  · rw [ite_eq_right h]
    constructor
    · intro same; cases same
    · rintro ⟨_, original, canonical⟩
      exfalso
      apply h
      simp only [File.resolvedAllowed, Bool.and_eq_true]
      exact ⟨original, canonical⟩

theorem extracted_resolved_effects (r : file.Resolved α ρ) (p : policy.Policy)
    (out : file.Request α × scope.Scope × ρ)
    (h : file.Resolved.consume r p = ok (some out)) :
    ∀ cap ∈ File.effects (operation r.request.operation),
      ((PolicyRefinement.owned p).deniesCap cap = false ∧
        (PolicyRefinement.owned p).grantsReq ⟨cap, some (PolicyRefinement.decoded r.request.path)⟩ = true) ∧
      ((PolicyRefinement.owned p).deniesCap cap = false ∧
        (PolicyRefinement.owned p).grantsReq ⟨cap, some (PolicyRefinement.decoded r.canonical)⟩ = true) := by
  obtain ⟨_, original, canonical⟩ := (extracted_resolved_iff r p out).mp h
  intro cap member
  exact ⟨(File.permission_iff _ _).mp original cap member,
    (File.permission_iff _ _).mp canonical cap member⟩

theorem extracted_resolved_refused (r : file.Resolved α ρ) (p : policy.Policy)
    (h : (PolicyRefinement.owned p).allowed (File.requirements (request r.request)) = false ∨
      (PolicyRefinement.owned p).allowed
        (File.requirements (File.canonicalRequest (resolved r))) = false) :
    file.Resolved.consume r p = ok none := by
  rw [resolved_consume_refines]
  have refused : File.resolvedAllowed (PolicyRefinement.owned p) (resolved r) = false := by
    change ((PolicyRefinement.owned p).allowed (File.requirements (request r.request)) &&
      (PolicyRefinement.owned p).allowed (File.requirements (File.canonicalRequest (resolved r)))) = false
    rcases h with h | h <;> simp [h]
  rw [refused]
  rfl

end MedenSpec.FileRefinement
