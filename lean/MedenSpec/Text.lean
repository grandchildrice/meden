module
public import Mathlib.Tactic.SplitIfs
public import Lean.Elab.Tactic.Omega
@[expose] public section

/-! Numeric display policy. These are Unicode scalar values, not UTF-8 bytes
or terminal cells. The String/terminal adapters are outside this model. -/
namespace MedenSpec.Text

abbrev control (code : Nat) : Prop := code ≤ 0x1f ∨ (0x7f ≤ code ∧ code ≤ 0x9f)
abbrev directional (code : Nat) : Prop :=
  code = 0x061c ∨ code = 0x200e ∨ code = 0x200f ∨
    (0x202a ≤ code ∧ code ≤ 0x202e) ∨ (0x2066 ≤ code ∧ code ≤ 0x2069)
def forbidden (code : Nat) : Prop := (control code ∧ code ≠ 0x0a) ∨ directional code
def scalar (code : Nat) : Prop := code ≤ 0x10ffff ∧ (code < 0xd800 ∨ 0xdfff < code)

def render (code : Nat) : Nat :=
  if code = 0x0a then 0x0a
  else if code = 0x09 then 0x20
  else if code = 0x1b then 0x241b
  else if directional code then 0xfffd
  else if control code then 0xfffd
  else code

theorem safe (code : Nat) : ¬forbidden (render code) := by
  unfold render
  split_ifs <;> simp_all [forbidden, control, directional]

theorem preserves_safe (code : Nat) (h : ¬forbidden code) : render code = code := by
  unfold render
  split_ifs <;> simp_all [forbidden, control, directional]

theorem valid_output (code : Nat) (h : scalar code) : scalar (render code) := by
  unfold render
  split_ifs <;> simp_all [scalar]

theorem idempotent (code : Nat) : render (render code) = render code :=
  preserves_safe _ (safe code)

theorem newline_iff (code : Nat) : render code = 0x0a ↔ code = 0x0a := by
  unfold render
  split_ifs <;> simp_all

end MedenSpec.Text
