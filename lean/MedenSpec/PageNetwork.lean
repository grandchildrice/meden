module
public import Mathlib.Tactic.SplitIfs
public import Lean.Elab.Tactic.Omega
@[expose] public section

namespace MedenSpec.PageNetwork

/-- Excluded IPv4 intervals, expressed as the first 24 address bits.
The final octet does not affect this deliberately conservative policy. -/
def blocked4 (address : Nat) : Prop :=
  address ≤ 0x00ffff ∨ (0x0a0000 ≤ address ∧ address ≤ 0x0affff) ∨
  (0x7f0000 ≤ address ∧ address ≤ 0x7fffff) ∨ address ≥ 0xe00000 ∨
  (0x644000 ≤ address ∧ address ≤ 0x647fff) ∨
  (0xa9fe00 ≤ address ∧ address ≤ 0xa9feff) ∨
  (0xac1000 ≤ address ∧ address ≤ 0xac1fff) ∨
  (0xc0a800 ≤ address ∧ address ≤ 0xc0a8ff) ∨
  address = 0xc00000 ∨ address = 0xc00002 ∨ address = 0xc05863 ∨
  (0xc61200 ≤ address ∧ address ≤ 0xc613ff) ∨
  address = 0xc63364 ∨ address = 0xcb0071
  deriving Decidable

def v4 (address : Nat) : Bool := decide (¬blocked4 address)

/-- Accepted global IPv6 region, excluding special/transition/documentation
prefixes. The remaining 96 bits do not affect this policy. -/
def accepted6 (address : Nat) : Prop :=
  0x20000000 ≤ address ∧ address ≤ 0x3fffffff ∧
  ¬(0x20010000 ≤ address ∧ address ≤ 0x200101ff) ∧
  address ≠ 0x20010db8 ∧ ¬(0x20020000 ≤ address ∧ address ≤ 0x2002ffff) ∧
  ¬(0x3fff0000 ≤ address ∧ address ≤ 0x3fff0fff)
  deriving Decidable

def v6 (address : Nat) : Bool := decide (accepted6 address)

theorem v4_refuses (address : Nat) (h : blocked4 address) : v4 address = false := by
  simp [v4, h]

theorem v6_refuses (address : Nat) (h : ¬accepted6 address) : v6 address = false := by
  simp [v6, h]

end MedenSpec.PageNetwork
