import MedenSpec.Policy
import MedenSpec.GlobRefinement

/-! Actual policy decisions over immutable, decoded scopes. String decoding,
JSON, diagnostic formatting and physical resource attribution are separate. -/
namespace MedenSpec.PolicyRefinement
open Aeneas Aeneas.Std Result ControlFlow MedenPolicyImpl
open Aeneas.Std.WP
set_option maxHeartbeats 4000000

def cap : capability.Capability → Capability
  | .FsRead => .fsRead | .FsWrite => .fsWrite | .NetEgress => .netEgress
  | .NetListen => .netListen | .ShellExec => .shellExec | .EnvRead => .envRead
  | .SecretRead => .secretRead | .ProcessSpawn => .processSpawn
  | .PackageInstall => .packageInstall | .Clipboard => .clipboard | .UiControl => .uiControl

def decoded (s : scope.Scope) : String := String.ofList s.val
def grant (g : policy.Grant) : Grant := ⟨cap g.capability, g.scope.map decoded⟩
def requirement (r : capability.Requirement) : Requirement :=
  ⟨cap r.capability, r.scope.map decoded⟩
def model (denies : List capability.Capability) (grants : List policy.Grant) : Policy :=
  ⟨"", grants.map grant, denies.map cap⟩
def owned (p : policy.Policy) : Policy := model p.denies.val p.grants.val

private theorem cap_eq (x y : capability.Capability) :
    capability.Capability.Insts.CoreCmpPartialEqCapability.eq x y = ok (decide (cap x = cap y)) := by
  cases x <;> cases y <;>
    simp [cap, capability.Capability.Insts.CoreCmpPartialEqCapability.eq,
      capability.Capability.read_discriminant]

theorem scope_matches_refines (pattern text : scope.Scope) :
    scope.Scope.matches pattern text = ok (globMatch pattern.val text.val) := by
  simp [scope.Scope.matches, GlobRefinement.glob_match_chars_refines, alloc.vec.Vec.deref]

theorem covers_refines (g : policy.Grant) (r : capability.Requirement) :
    policy.covers_ref g r = ok ((grant g).covers (requirement r)) := by
  unfold policy.covers_ref
  simp only [core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, cap_eq]
  by_cases he : cap g.capability = cap r.capability
  all_goals
    cases hg : g.scope <;> cases hr : r.scope <;>
      simp [hg, hr, he, Grant.covers, grant, requirement, decoded, scope_matches_refines]

theorem grant_covers_refines (g : policy.Grant) (r : capability.Requirement) :
    policy.Grant.covers g r = ok ((grant g).covers (requirement r)) := by
  exact covers_refines g r

private theorem total_result {α : Type} {m : Result α} {x : α}
    (h : m ⦃ y => y = x ⦄) : m = ok x := by
  cases hm : m with
  | ret value => simp only [hm, spec_ok] at h; simp [hm, h]
  | vis effect continuation => simp [hm] at h
  | div => simp [hm] at h

def covered (g : policy.Grant) (r : capability.Requirement) : Bool :=
  (grant g).covers (requirement r)

theorem grants_loop_refines (grants : Slice policy.Grant) (r : capability.Requirement)
    (index : Usize) :
    policy.grants_ref_loop grants r index ⦃ b =>
      b = (grants.val.drop index.val).any (fun g => covered g r) ⦄ := by
  unfold policy.grants_ref_loop
  apply Aeneas.Std.loop.spec_decr_nat
    (fun i : Usize => grants.val.length - i.val)
    (fun i : Usize => (grants.val.drop i.val).any (fun g => covered g r) =
        (grants.val.drop index.val).any (fun g => covered g r))
    (fun b => b = (grants.val.drop index.val).any (fun g => covered g r))
    (fun i => policy.grants_ref_loop.body grants r i) index
  · intro current hinv
    unfold policy.grants_ref_loop.body
    by_cases hlt : current.val < grants.val.length
    · have hlt' : current < Slice.len grants := by scalar_tac
      simp only [if_pos hlt']
      step as ⟨g, hg⟩
      rw [covers_refines]
      simp only [bind_ok]
      have hdrop := List.drop_eq_getElem_cons hlt
      have hany : (grants.val.drop current.val).any (fun g => covered g r) =
          ((grant g).covers (requirement r) ||
            (grants.val.drop (current.val + 1)).any (fun g => covered g r)) := by
        rw [hdrop]
        simp only [List.any_cons, covered, hg]
      cases hc : (grant g).covers (requirement r) with
      | false =>
        simp only [hc, Bool.false_eq_true, ↓reduceIte]
        step as ⟨next, hnext⟩
        constructor
        · have htail := (show (grants.val.drop (current.val + 1)).any (fun g => covered g r) =
              (grants.val.drop current.val).any (fun g => covered g r) by simpa [hc] using hany.symm)
          simpa [hnext] using htail.trans hinv
        · omega
      | true =>
        simp only [hc, ↓reduceIte]
        apply (spec_ok _).mpr
        have htrue : (grants.val.drop current.val).any (fun g => covered g r) = true := by
          simpa [hc] using hany
        exact htrue.symm.trans hinv
    · have hlt' : ¬current < Slice.len grants := by scalar_tac
      have hnil : grants.val.drop current.val = [] := List.drop_eq_nil_of_le (by omega)
      simpa [hlt', hnil] using hinv.symm
  · rfl

theorem grants_refines (grants : Slice policy.Grant) (r : capability.Requirement) :
    policy.grants_ref grants r = ok ((model [] grants.val).grantsReq (requirement r)) := by
  apply total_result
  apply spec_mono (grants_loop_refines grants r 0#usize)
  intro result hresult
  simpa [model, Policy.grantsReq, covered, Function.comp_def] using hresult

private theorem contains_refines (denies : Slice capability.Capability) (c : capability.Capability) :
    core.slice.Slice.contains capability.Capability.Insts.CoreCmpPartialEqCapability denies c =
      ok ((denies.val.map cap).contains (cap c)) := by
  unfold core.slice.Slice.contains
  have h : ∀ ds : List capability.Capability,
      List.anyM (capability.Capability.Insts.CoreCmpPartialEqCapability.eq c) ds =
        ok ((ds.map cap).contains (cap c)) := by
    intro ds
    induction ds with
    | nil => rfl
    | cons d ds ih =>
      simp only [List.anyM_cons, cap_eq, bind_ok]
      by_cases he : cap c = cap d
      · simp [he]; rfl
      · simp [he, Ne.symm he, ih]
  exact h denies.val

def refusal (denies : List capability.Capability) (grants : List policy.Grant)
    (r : capability.Requirement) : Option policy.Refusal :=
  if (model denies grants).deniesCap (requirement r).cap then some .Denied
  else if (model denies grants).grantsReq (requirement r) then none else some .NotGranted

theorem refusal_refines (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (r : capability.Requirement) :
    policy.refusal_ref denies grants r = ok (refusal denies.val grants.val r) := by
  simp [policy.refusal_ref, contains_refines, grants_refines, refusal, model,
    requirement, Policy.deniesCap, Policy.grantsReq]
  split <;> (try split) <;> rfl

theorem refusal_matches_violation (denies : List capability.Capability)
    (grants : List policy.Grant) (r : capability.Requirement) :
    (refusal denies grants r).isSome = (model denies grants).violates (requirement r) := by
  unfold refusal Policy.violates
  cases hd : (model denies grants).deniesCap (requirement r).cap <;>
    cases hg : (model denies grants).grantsReq (requirement r) <;> simp [hd, hg]

theorem violates_refines (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (r : capability.Requirement) :
    policy.violates_ref denies grants r = ok ((model denies.val grants.val).violates (requirement r)) := by
  simp only [policy.violates_ref, refusal_refines, bind_ok]
  change ok ((refusal denies.val grants.val r).isSome) = _
  rw [refusal_matches_violation]

def rows (denies : List capability.Capability) (grants : List policy.Grant)
    (reqs : List capability.Requirement) : List (Option policy.Refusal) :=
  reqs.map (refusal denies grants)

def permits (denies : List capability.Capability) (grants : List policy.Grant)
    (reqs : List capability.Requirement) : Bool :=
  reqs.all (fun r => !(refusal denies grants r).isSome)

theorem evaluate_loop_refines (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (reqs : Slice capability.Requirement) (allowed : Bool)
    (refusals : alloc.vec.Vec (Option policy.Refusal)) (index : Usize)
    (hspace : refusals.val.length + (reqs.val.drop index.val).length ≤ Usize.max) :
    policy.evaluate_ref_loop denies grants reqs allowed refusals index ⦃ out nextRows =>
      out = (allowed && permits denies.val grants.val (reqs.val.drop index.val)) ∧
      nextRows.val = refusals.val ++ rows denies.val grants.val (reqs.val.drop index.val) ⦄ := by
  unfold policy.evaluate_ref_loop
  apply Aeneas.Std.loop.spec_decr_nat
    (fun s : Bool × alloc.vec.Vec (Option policy.Refusal) × Usize => reqs.val.length - s.2.2.val)
    (fun s : Bool × alloc.vec.Vec (Option policy.Refusal) × Usize =>
      s.2.1.val.length + (reqs.val.drop s.2.2.val).length ≤ Usize.max ∧
      (s.1 && permits denies.val grants.val (reqs.val.drop s.2.2.val)) =
        (allowed && permits denies.val grants.val (reqs.val.drop index.val)) ∧
      s.2.1.val ++ rows denies.val grants.val (reqs.val.drop s.2.2.val) =
        refusals.val ++ rows denies.val grants.val (reqs.val.drop index.val))
    (fun out : Bool × alloc.vec.Vec (Option policy.Refusal) =>
      out.1 = (allowed && permits denies.val grants.val (reqs.val.drop index.val)) ∧
      out.2.val = refusals.val ++ rows denies.val grants.val (reqs.val.drop index.val))
    (fun (a, rs, i) => policy.evaluate_ref_loop.body denies grants reqs a rs i)
    (allowed, refusals, index)
  · rintro ⟨currentAllowed, currentRows, current⟩ ⟨hbound, hflag, hresult⟩
    dsimp only at *
    unfold policy.evaluate_ref_loop.body
    by_cases hlt : current.val < reqs.val.length
    · have hlt' : current < Slice.len reqs := by scalar_tac
      simp only [if_pos hlt']
      step as ⟨req, hreq⟩
      rw [refusal_refines]
      simp only [bind_ok]
      have hdrop := List.drop_eq_getElem_cons hlt
      have hrows : rows denies.val grants.val (reqs.val.drop current.val) =
          refusal denies.val grants.val req :: rows denies.val grants.val (reqs.val.drop (current.val + 1)) := by
        rw [hdrop]
        simp only [rows, List.map_cons, hreq]
      have hpermits : permits denies.val grants.val (reqs.val.drop current.val) =
          (!(refusal denies.val grants.val req).isSome &&
            permits denies.val grants.val (reqs.val.drop (current.val + 1))) := by
        rw [hdrop]
        simp only [permits, List.all_cons, hreq]
      have hpush : currentRows.val.length < Usize.max := by
        simp only [List.length_drop] at hbound
        omega
      cases hf : refusal denies.val grants.val req <;>
        simp +implicitDefEqProofs only [hf, core.option.Option.is_some,
          Option.isSome_none, Option.isSome_some, Bool.false_eq_true, bind_ok, ↓reduceIte]
      all_goals
        step as ⟨nextRows, hnextRows⟩
        step as ⟨next, hnext⟩
        refine ⟨?_, ?_, ?_, ?_⟩
        · simp only [hnextRows, List.length_append, List.length_singleton, List.length_drop] at *
          omega
        · simpa [hnext, hpermits, hf] using hflag
        · rw [hrows] at hresult
          simpa [hnextRows, hnext, hf, List.append_assoc] using hresult
        · omega
    · have hlt' : ¬current < Slice.len reqs := by scalar_tac
      have hnil : reqs.val.drop current.val = [] := List.drop_eq_nil_of_le (by omega)
      simp only [if_neg hlt', spec_ok]
      exact ⟨by simpa [hnil, permits] using hflag, by simpa [hnil, rows] using hresult⟩
  · exact ⟨hspace, rfl, rfl⟩

theorem permits_spec (denies : List capability.Capability) (grants : List policy.Grant)
    (reqs : List capability.Requirement) :
    permits denies grants reqs = (model denies grants).allowed (reqs.map requirement) := by
  simp [permits, Policy.allowed, refusal_matches_violation, Function.comp_def]

theorem evaluate_refines (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (reqs : Slice capability.Requirement) :
    policy.evaluate_ref denies grants reqs ⦃ e =>
      e.allowed = (model denies.val grants.val).allowed (reqs.val.map requirement) ∧
      e.refusals.val = rows denies.val grants.val reqs.val ⦄ := by
  unfold policy.evaluate_ref
  apply spec_bind (evaluate_loop_refines denies grants reqs true (alloc.vec.Vec.new _) 0#usize (by simp))
  rintro ⟨a, rs⟩ ⟨ha, hrs⟩
  apply (spec_ok _).mpr
  exact ⟨by simpa [permits_spec] using ha, by simpa using hrs⟩

theorem allowed_refines (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (reqs : Slice capability.Requirement) :
    policy.allowed_ref denies grants reqs =
      ok ((model denies.val grants.val).allowed (reqs.val.map requirement)) := by
  apply total_result
  unfold policy.allowed_ref
  apply spec_bind (evaluate_refines denies grants reqs)
  intro e he
  simpa only [spec_ok] using he.1

theorem owned_evaluate_refines (p : policy.Policy) (reqs : Slice capability.Requirement) :
    policy.evaluate reqs p ⦃ e =>
      e.allowed = (owned p).allowed (reqs.val.map requirement) ∧
      e.refusals.val = rows p.denies.val p.grants.val reqs.val ⦄ := by
  simpa [policy.evaluate, owned, alloc.vec.Vec.deref] using
    evaluate_refines (alloc.vec.Vec.deref p.denies) (alloc.vec.Vec.deref p.grants) reqs

theorem owned_allowed_refines (p : policy.Policy) (reqs : Slice capability.Requirement) :
    policy.allowed reqs p = ok ((owned p).allowed (reqs.val.map requirement)) := by
  apply total_result
  unfold policy.allowed
  apply spec_bind (owned_evaluate_refines p reqs)
  intro e he
  simpa only [spec_ok] using he.1

theorem owned_violates_refines (p : policy.Policy) (r : capability.Requirement) :
    policy.violates p r = ok ((owned p).violates (requirement r)) := by
  simp [policy.violates, violates_refines, owned, alloc.vec.Vec.deref]

theorem denies_method_refines (p : policy.Policy) (c : capability.Capability) :
    policy.Policy.impl.denies p c = ok ((owned p).deniesCap (cap c)) := by
  simp [policy.Policy.impl.denies, contains_refines, owned, model, Policy.deniesCap,
    alloc.vec.Vec.deref]

theorem grants_method_refines (p : policy.Policy) (r : capability.Requirement) :
    policy.Policy.impl.grants p r = ok ((owned p).grantsReq (requirement r)) := by
  simp [policy.Policy.impl.grants, grants_refines, owned, model, Policy.grantsReq,
    alloc.vec.Vec.deref]

theorem extracted_refusal_count (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (reqs : Slice capability.Requirement) :
    policy.evaluate_ref denies grants reqs ⦃ e => e.refusals.val.length = reqs.val.length ⦄ := by
  apply spec_mono (evaluate_refines denies grants reqs)
  intro e he
  simp [he.2, rows]

theorem extracted_deny_wins (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (reqs : Slice capability.Requirement) (r : capability.Requirement)
    (hr : r ∈ reqs.val) (hd : r.capability ∈ denies.val) :
    policy.allowed_ref denies grants reqs = ok false := by
  rw [allowed_refines]
  have hreq : requirement r ∈ reqs.val.map requirement := List.mem_map_of_mem hr
  have hdenied : (model denies.val grants.val).deniesCap (requirement r).cap = true := by
    simp only [model, Policy.deniesCap, requirement, List.contains_eq_mem, decide_eq_true_eq,
      List.mem_map]
    exact ⟨r.capability, hd, rfl⟩
  rw [Policy.deny_wins _ _ _ hreq hdenied]

theorem extracted_allowed_iff (denies : Slice capability.Capability) (grants : Slice policy.Grant)
    (reqs : Slice capability.Requirement) :
    policy.allowed_ref denies grants reqs = ok true ↔
      ∀ r ∈ reqs.val.map requirement,
        (model denies.val grants.val).deniesCap r.cap = false ∧
        (model denies.val grants.val).grantsReq r = true := by
  rw [allowed_refines, Result.ok.injEq, Policy.allowed_iff]

theorem extracted_grants_monotone (denies : Slice capability.Capability)
    (grants extended : Slice policy.Grant) (g : policy.Grant)
    (reqs : Slice capability.Requirement) (hext : extended.val = g :: grants.val)
    (h : policy.allowed_ref denies grants reqs = ok true) :
    policy.allowed_ref denies extended reqs = ok true := by
  rw [allowed_refines, Result.ok.injEq] at h ⊢
  have hs := Policy.grants_monotone (model denies.val grants.val) (grant g)
    (reqs.val.map requirement) h
  simpa [model, hext] using hs

theorem owned_metadata_irrelevant (p : policy.Policy)
    (reqs other : Slice capability.Requirement) (label : String)
    (h : reqs.val.map requirement = other.val.map requirement) :
    policy.allowed reqs p = policy.allowed other { p with «name» := label } := by
  simp only [owned_allowed_refines, owned, h]

theorem covers_evidence_irrelevant (g : policy.Grant) (r : capability.Requirement)
    (evidence : Option String) :
    policy.covers_ref g { r with evidence := evidence } = policy.covers_ref g r := by
  simp [covers_refines, requirement]

end MedenSpec.PolicyRefinement
