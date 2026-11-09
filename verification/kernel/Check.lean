import Candidate
import Spec
open Aeneas Aeneas.Std Result
namespace KernelProof
set_option maxHeartbeats 2000000
set_option linter.unusedSimpArgs false
set_option linter.unusedTactic false
set_option linter.unreachableTactic false

@@ABSTRACT_STATE@@

-- Lean's core conditional elaborator also emits reflected Boolean cases.
-- Relate that encoding to the usual dependent conditional without changing
-- the supplied specification or adding assumptions about a candidate.
private theorem reflected_rec_false {α : Sort u} (p : Prop)
    (b : Bool) (h : b.Reflects p) (yes : p → α) (no : ¬p → α) (hp : ¬p) :
    Bool.rec (motive := fun b => b.Reflects p → α) no yes b h = no hp := by
  cases b with
  | false => rfl
  | true => exact (hp h).elim

private theorem reflected_rec_true {α : Sort u} (p : Prop)
    (b : Bool) (h : b.Reflects p) (yes : p → α) (no : ¬p → α) (hp : p) :
    Bool.rec (motive := fun b => b.Reflects p → α) no yes b h = yes hp := by
  cases b with
  | false => exact (h hp).elim
  | true => rfl

private theorem saturating_sub_val (a b : U64) :
    (core.num.U64.saturating_sub a b).val = a.val - b.val := by
  change (max 0 (a.val - b.val)) % (2^64) = a.val - b.val
  rw [Nat.zero_max]
  exact Nat.mod_eq_of_lt (by scalar_tac)

private theorem saturating_add_val (a b : U64) :
    (core.num.U64.saturating_add a b).val = min 18446744073709551615 (a.val + b.val) := by
  change (min (UScalar.max .U64) (a.val + b.val)) % (2^64) =
    min 18446744073709551615 (a.val + b.val)
  have hm : UScalar.max .U64 = 18446744073709551615 := by
    simp only [UScalar.max_UScalarTy_U64_eq, U64.max_eq]
  rw [hm]
  exact Nat.mod_eq_of_lt (Nat.lt_of_le_of_lt (Nat.min_le_left _ _) (by decide))

private theorem checked_add_some (a b r : U64) : a.checked_add b = some r ↔
    a.val + b.val ≤ 18446744073709551615 ∧ r.val = a.val + b.val := by
  have h := U64.checked_add_bv_spec a b
  cases he : a.checked_add b <;> simp_all [U64.max_eq, UScalar.eq_equiv]
  all_goals omega

private theorem checked_add_none (a b : U64) : a.checked_add b = none ↔
    18446744073709551615 < a.val + b.val := by
  have h := U64.checked_add_bv_spec a b
  cases he : a.checked_add b <;> simp_all [U64.max_eq]

private theorem checked_sub_some (a b r : U64) : a.checked_sub b = some r ↔
    b.val ≤ a.val ∧ r.val = a.val - b.val := by
  have h := U64.checked_sub_bv_spec a b
  cases he : a.checked_sub b <;> simp_all [UScalar.eq_equiv]
  all_goals omega

private theorem checked_sub_none (a b : U64) : a.checked_sub b = none ↔ a.val < b.val := by
  have h := U64.checked_sub_bv_spec a b
  cases he : a.checked_sub b <;> simp_all

private theorem checked_add_default (a b d : U64) :
    (core.option.Option.unwrap_or (a.checked_add b) d).val =
      if a.val + b.val ≤ 18446744073709551615 then a.val + b.val else d.val := by
  have h := U64.checked_add_bv_spec a b
  cases he : a.checked_add b <;> simp_all [U64.max_eq, core.option.Option.unwrap_or]
  all_goals (try split_ifs) <;> omega

private theorem checked_sub_default (a b d : U64) :
    (core.option.Option.unwrap_or (a.checked_sub b) d).val =
      if b.val ≤ a.val then a.val - b.val else d.val := by
  have h := U64.checked_sub_bv_spec a b
  cases he : a.checked_sub b <;> simp_all [core.option.Option.unwrap_or]
  all_goals (try split_ifs) <;> omega

theorem actual_spec (_state : @@STATE_TYPE@@)@@ARGUMENTS@@ :
    @@NATIVE_CALL@@ ⦃ out => abstractState out = @@SPEC_CALL@@ ⦄ := by
  unfold @@ENTRY@@
  try simp only [Aeneas.Std.lift, Aeneas.Std.bind_ok]
  step*
  all_goals simp_all [abstractState, KernelContract.transition,
    reflected_rec_false, reflected_rec_true,
    U64.rMax, UScalar.eq_equiv, saturating_sub_val, saturating_add_val,
    checked_add_some, checked_add_none, checked_sub_some, checked_sub_none,
    Option.isSome_iff_ne_none, checked_add_default, checked_sub_default]
  all_goals try simp (disch := omega) only [reflected_rec_false, reflected_rec_true]
  all_goals try split_ifs
  all_goals scalar_tac_preprocess
  all_goals try simp (disch := omega) only [reflected_rec_false, reflected_rec_true]
  all_goals repeat first | (split <;> try simp_all)
  all_goals try simp_all
  all_goals try simp (disch := omega) only [reflected_rec_false, reflected_rec_true]
  all_goals try congr 1
  all_goals scalar_tac_preprocess
  all_goals try simp (disch := omega) only [reflected_rec_false, reflected_rec_true]
  all_goals omega

theorem actual_refines (_state : @@STATE_TYPE@@)@@ARGUMENTS@@ :
    (do let out ← @@NATIVE_CALL@@; ok (abstractState out)) =
      ok (@@SPEC_CALL@@) := by
  have h := actual_spec _state @@ARG_NAMES@@
  cases he : @@NATIVE_CALL@@ with
  | ret out => simpa [he] using congrArg ok (by simpa [he] using h)
  | vis effect continuation => simp [he] at h
  | div => simp [he] at h
private theorem success (_state : @@STATE_TYPE@@)@@ARGUMENTS@@ :
    ∃ out, @@NATIVE_CALL@@ = ok out ∧ abstractState out = @@SPEC_CALL@@ := by
  have h := actual_refines _state @@ARG_NAMES@@
  cases he : @@NATIVE_CALL@@ with
  | ret out => exact ⟨out, by simp_all, by simpa [he] using h⟩
  | vis effect continuation => simp [he] at h
  | div => simp [he] at h
end KernelProof

