import MedenSpec.Generated.Repair
import MedenSpec.PageNetwork

namespace MedenSpec.PageNetworkRefinement
open Aeneas Aeneas.Std Result
set_option maxHeartbeats 4000000
set_option maxRecDepth 2048

private theorem literal8 (n : Nat) :
    (UScalar.mk (BitVec.ofNat 8 n) : U8).val = n % 256 := rfl

private theorem literal16 (n : Nat) :
    (UScalar.mk (BitVec.ofNat 16 n) : U16).val = n % 65536 := rfl

private theorem mask_bounds (bits : BitVec 16) :
    (bits &&& 0xe000#16 = 0x2000#16) ↔ (8192 ≤ bits.toNat ∧ bits.toNat ≤ 16383) := by
  let hi : BitVec 3 := bits.extractLsb' 13 3
  let lo : BitVec 13 := bits.extractLsb' 0 13
  have representation : hi ++ lo = bits := BitVec.extractLsb'_append_extractLsb'
  have value : bits.toNat = hi.toNat * 8192 + lo.toNat := by
    rw [← representation, BitVec.toNat_append,
      ← Nat.shiftLeft_add_eq_or_of_lt lo.isLt hi.toNat, Nat.shiftLeft_eq]
  have masked : (bits &&& 0xe000#16).toNat = hi.toNat * 8192 := by
    rw [← representation]
    change ((hi ++ lo) &&& (BitVec.allOnes 3 ++ 0#13)).toNat = _
    rw [BitVec.and_append, BitVec.and_allOnes, BitVec.and_zero, BitVec.toNat_append]
    simp [Nat.shiftLeft_eq]
  have low_bound : lo.toNat < 8192 := lo.isLt
  rw [BitVec.toNat_eq]
  change (bits &&& 0xe000#16).toNat = 8192 ↔ _
  rw [masked, value]
  omega

private theorem mask16 (a : U16) :
    (a &&& 57344#u16 = 8192#u16) ↔ (8192 ≤ a.val ∧ a.val ≤ 16383) := by
  change ((⟨a.bv &&& 0xe000#16⟩ : U16) = (⟨0x2000#16⟩ : U16)) ↔ _
  simp only [UScalar.mk.injEq]
  exact mask_bounds a.bv

private theorem mask16_val (a : U16) :
    a.val &&& 57344 = 8192 ↔ (8192 ≤ a.val ∧ a.val ≤ 16383) := by
  simpa only [UScalar.eq_equiv, UScalar.ofNatCore_val_eq, UScalar.val_and] using mask16 a

theorem v4_refines (a b c : U8) :
    MedenRepairImpl.pages.public_v4 a b c =
      ok (PageNetwork.v4 (a.val * 65536 + b.val * 256 + c.val)) := by
  have ha := a.le_max
  have hb := b.le_max
  have hc := c.le_max
  unfold MedenRepairImpl.pages.public_v4
  repeat' (split <;> try simp only [Aeneas.Std.bind_ok])
  all_goals apply congrArg Result.ok
  all_goals apply Bool.eq_iff_iff.mpr
  all_goals simp_all [PageNetwork.v4, PageNetwork.blocked4,
    UScalar.eq_equiv, UScalar.ofNatCore_val_eq, literal8]
  all_goals omega

theorem v6_refines (a b : U16) :
    MedenRepairImpl.pages.public_v6 a b =
      ok (PageNetwork.v6 (a.val * 65536 + b.val)) := by
  have ha := a.le_max
  have hb := b.le_max
  unfold MedenRepairImpl.pages.public_v6
  simp only [Aeneas.Std.lift, Aeneas.Std.bind_ok]
  repeat' split
  all_goals apply congrArg Result.ok
  all_goals apply Bool.eq_iff_iff.mpr
  all_goals simp_all [PageNetwork.v6, PageNetwork.accepted6, mask16_val,
    UScalar.eq_equiv, UScalar.ofNatCore_val_eq, literal16]
  all_goals omega

theorem v4_excludes (a b c : U8)
    (h : PageNetwork.blocked4 (a.val * 65536 + b.val * 256 + c.val)) :
    MedenRepairImpl.pages.public_v4 a b c = ok false := by
  rw [v4_refines, PageNetwork.v4_refuses _ h]

theorem v6_excludes (a b : U16)
    (h : ¬PageNetwork.accepted6 (a.val * 65536 + b.val)) :
    MedenRepairImpl.pages.public_v6 a b = ok false := by
  rw [v6_refines, PageNetwork.v6_refuses _ h]

end MedenSpec.PageNetworkRefinement
