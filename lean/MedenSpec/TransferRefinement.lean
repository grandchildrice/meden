import MedenSpec.Transfer
import MedenSpec.Generated.Transfer

/-! Refinement of the generated production Rust transfer kernel. Extraction,
Rust's standard-library models and the compiler remain trust dependencies. -/
namespace MedenSpec.TransferRefinement
open Aeneas Aeneas.Std Result

private theorem allM_eq (xs ys : List U8) (hlen : xs.length = ys.length) :
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

private theorem vec_eq (x y : alloc.vec.Vec U8) :
    alloc.vec.partial_eq.PartialEqVec.eq core.cmp.PartialEqU8 x y =
      ok (decide (x.val = y.val)) := by
  unfold alloc.vec.partial_eq.PartialEqVec.eq
  by_cases h : x.val.length = y.val.length
  · simp only [h, ↓reduceIte]
    exact allM_eq x.val y.val h
  · simp [h, show x.val ≠ y.val from fun e => h (congrArg List.length e)]

private theorem slice_eq (x y : Slice U8) :
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

private theorem bytes_eq (xs ys : List U8) : bytes xs = bytes ys ↔ xs = ys := by
  exact ⟨fun h => List.map_injective_iff.mpr
      (fun _ _ h => UScalar.eq_of_val_eq h) h,
    congrArg bytes⟩

/-- Abstraction of the actual generated request, without hashing. -/
def request (r : MedenImpl.transfer.Request) : Transfer.Request :=
  ⟨bytes r.session.val, bytes r.recipient.val, bytes r.request.val⟩

/-- Abstraction of the generated permit, including its complete state. -/
def permit (p : MedenImpl.transfer.Permit) : Transfer.Permit :=
  ⟨request p.request, p.expires.val, p.spent⟩

private theorem request_eq (x y : MedenImpl.transfer.Request) :
    MedenImpl.transfer.Request.Insts.CoreCmpPartialEqRequest.eq x y =
      ok (decide (request x = request y)) := by
  unfold MedenImpl.transfer.Request.Insts.CoreCmpPartialEqRequest.eq
  simp only [vec_eq]
  by_cases hs : x.session.val = y.session.val <;>
    by_cases hr : x.recipient.val = y.recipient.val <;>
    by_cases hb : x.request.val = y.request.val <;>
    simp [hs, hr, hb, request, Transfer.Request.mk.injEq, bytes_eq]

/-- The extracted Rust decision terminates and agrees with the specification. -/
theorem allowed_refines (now : U64) (session : Slice U8)
    (p : MedenImpl.transfer.Permit) (r : MedenImpl.transfer.Request) :
    MedenImpl.transfer.allowed now session p r =
      ok (Transfer.allowed now.val (bytes session.val) (permit p) (request r)) := by
  dsimp only [MedenImpl.transfer.allowed]
  rw [slice_eq]
  simp only [request_eq]
  by_cases hs : r.session.val = session.val <;>
    by_cases he : p.request.session.val = r.session.val <;>
    by_cases hr : p.request.recipient.val = r.recipient.val <;>
    by_cases hc : p.request.request.val = r.request.val <;>
    by_cases ht : now.val < p.expires.val <;>
    cases hb : p.spent <;>
    simp [hs, he, hr, hc, ht, hb, Transfer.allowed, permit, request, bytes_eq,
      alloc.vec.Vec.deref]
  split <;> simp_all

/-- Both acceptance and the entire updated permit refine the Lean transition. -/
theorem reserve_refines (now : U64) (session : Slice U8)
    (p : MedenImpl.transfer.Permit) (r : MedenImpl.transfer.Request) :
    (do let out ← MedenImpl.transfer.reserve now session p r
        ok (permit out.2, out.1)) =
      ok (Transfer.reserve now.val (bytes session.val) (permit p) (request r)) := by
  unfold MedenImpl.transfer.reserve
  rw [allowed_refines]
  simp [Transfer.reserve, permit]

end MedenSpec.TransferRefinement
