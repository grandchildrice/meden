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

end MedenSpec.FileRefinement
