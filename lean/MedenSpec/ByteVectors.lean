import Aeneas

/-! Exact byte-vector models shared by implementation refinements. -/
namespace MedenSpec.ByteVectors
open Aeneas Aeneas.Std Result

theorem vec_clone (v : alloc.vec.Vec U8) :
    alloc.vec.CloneVec.clone core.clone.CloneU8 v = ok v := by
  have h := Slice.clone_spec (s := v.slice)
    (clone := core.clone.CloneU8.clone) (by intros; rfl)
  unfold alloc.vec.CloneVec.clone
  cases hc : Slice.clone core.clone.CloneU8.clone v.slice with
  | ret out =>
    have he : v.slice = out := by simpa [hc] using h
    simp [← he]
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

theorem allM_eq (xs ys : List U8) (hlen : xs.length = ys.length) :
    List.allM (fun p : U8 × U8 => ok (decide (p.1 = p.2))) (List.zip xs ys) =
      ok (decide (xs = ys)) := by
  induction xs generalizing ys with
  | nil => cases ys <;> simp_all [pure]
  | cons x xs ih =>
    cases ys with
    | nil => simp_all
    | cons y ys =>
      have hl : xs.length = ys.length := by simpa using hlen
      by_cases h : x = y
      · subst y
        simp [ih ys hl, pure]
      · simp [h, pure]

theorem vec_eq (x y : alloc.vec.Vec U8) :
    alloc.vec.partial_eq.PartialEqVec.eq core.cmp.PartialEqU8 x y =
      ok (decide (x.val = y.val)) := by
  unfold alloc.vec.partial_eq.PartialEqVec.eq
  by_cases h : x.val.length = y.val.length
  · simp only [h, ↓reduceIte]
    exact allM_eq x.val y.val h
  · simp [h, show x.val ≠ y.val from fun e => h (congrArg List.length e)]

private theorem anyM_ne_eq {α : Type} [DecidableEq α] (xs ys : List α) (hlen : xs.length = ys.length) :
    List.anyM (fun p : α × α => ok (decide (p.1 ≠ p.2))) (List.zip xs ys) =
      ok (decide (xs ≠ ys)) := by
  induction xs generalizing ys with
  | nil => cases ys <;> simp_all [pure]
  | cons x xs ih =>
    cases ys with
    | nil => simp_all
    | cons y ys =>
      have hl : xs.length = ys.length := by simpa using hlen
      by_cases h : x = y
      · subst y
        simpa [pure] using ih ys hl
      · simp [h, pure]

theorem vec_ne (x y : alloc.vec.Vec U8) :
    alloc.vec.partial_eq.PartialEqVec.ne core.cmp.PartialEqU8 x y =
      ok (decide (x.val ≠ y.val)) := by
  unfold alloc.vec.partial_eq.PartialEqVec.ne
  by_cases h : x.val.length = y.val.length
  · simp only [h, ↓reduceIte]
    have f : (fun p : U8 × U8 => core.cmp.PartialEqU8.ne p.1 p.2) =
        (fun p => ok (decide (p.1 ≠ p.2))) := by
      funext p
      rfl
    exact (congrArg (fun f => List.anyM f (List.zip x.val y.val)) f).trans
      (anyM_ne_eq x.val y.val h)
  · simp [h, show x.val ≠ y.val from fun e => h (congrArg List.length e)]

theorem slice_eq (x y : Slice U8) :
    core.slice.cmp.PartialEqSlice.eq core.cmp.PartialEqU8 x y =
      ok (decide (x.val = y.val)) := by
  unfold core.slice.cmp.PartialEqSlice.eq
  by_cases h : x.val.length = y.val.length
  · simp only [h, ↓reduceIte]
    have f : (fun p : U8 × U8 => do
        let b ← liftFun2 core.cmp.impls.PartialEqU8.ne p.1 p.2
        ok (¬ b)) = (fun p => ok (decide (p.1 = p.2))) := by
      funext p
      simp [liftFun2, core.cmp.impls.PartialEqU8.ne, ← UScalar.eq_equiv]
    exact (congrArg (fun f => List.allM f (List.zip x.val y.val)) f).trans
      (allM_eq x.val y.val h)
  · simp [h, show x.val ≠ y.val from fun e => h (congrArg List.length e)]

/-- Interpret extracted Rust bytes as their exact numeric values. -/
def bytes (xs : List U8) : List Nat := xs.map (·.val)

theorem bytes_eq (xs ys : List U8) : bytes xs = bytes ys ↔ xs = ys := by
  exact ⟨fun h => List.map_injective_iff.mpr
      (fun _ _ h => UScalar.eq_of_val_eq h) h,
    congrArg bytes⟩


end MedenSpec.ByteVectors
