import MedenSpec.Transfer
import MedenSpec.Generated.Control
import MedenSpec.ByteVectors

/-! Refinement of the generated production Rust transfer kernel. Extraction,
Rust's standard-library models and the compiler remain trust dependencies. -/
namespace MedenSpec.TransferRefinement
open Aeneas Aeneas.Std Result

open ByteVectors

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

/-- Two actual extracted Rust reservations cannot both accept the same
permission, even when the second request/session/time are arbitrary. -/
theorem extracted_no_replay (now later : U64) (session otherSession : Slice U8)
    (p : MedenImpl.transfer.Permit) (r other : MedenImpl.transfer.Request) :
    (do let first ← MedenImpl.transfer.reserve now session p r
        let second ← MedenImpl.transfer.reserve later otherSession first.2 other
        ok (first.1 && second.1)) = ok false := by
  unfold MedenImpl.transfer.reserve
  rw [allowed_refines]
  simp only [Aeneas.Std.bind_ok]
  rw [allowed_refines]
  cases h : Transfer.allowed now.val (bytes session.val) (permit p) (request r) <;>
    simp [Transfer.allowed, permit]

end MedenSpec.TransferRefinement
