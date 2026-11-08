import MedenSpec.Generated.Approval

/-! The production TUI decision over actual Crossterm event types. Terminal
decoding, physical input provenance, screen routing and rendering remain
separate obligations; no text is reclassified as a key by this function. -/
namespace MedenSpec.ApprovalRefinement
open Aeneas Aeneas.Std Result
open MedenUIImpl.crossterm.event
set_option maxHeartbeats 4000000

def expected : Event → Bool
  | .Key key =>
    match key.kind, key.code with
    | .Press, .Char c => decide (key.modifiers.val = 0 ∧ c = 'y')
    | _, _ => false
  | _ => false

private theorem code_y (code : KeyCode) :
    KeyCode.Insts.CoreCmpPartialEqKeyCode.eq code (.Char 'y') =
      ok (match code with | .Char c => decide (c = 'y') | _ => false) := by
  cases code <;> simp [KeyCode.Insts.CoreCmpPartialEqKeyCode.eq,
    KeyCode.read_discriminant, MedenUIImpl.Char.Insts.CoreCmpPartialEqChar.eq]

theorem approves_refines (e : Event) :
    MedenUIImpl.tui.approval.approves e = ok (expected e) := by
  cases e with
  | Key key =>
    cases hk : key.kind <;> cases hc : key.code <;>
      simp [MedenUIImpl.tui.approval.approves, expected,
        KeyEventKind.Insts.CoreCmpPartialEqKeyEventKind.eq, KeyEventKind.read_discriminant,
        __1.KeyModifiers.is_empty, __1.InternalBitFlags.is_empty,
        MedenUIImpl.U8.Insts.BitflagsTraitsBits.EMPTY, code_y, hk, hc, UScalar.eq_equiv]
    all_goals by_cases hm : key.modifiers.val = 0 <;> simp [hm]
  | _ => rfl

theorem paste_cannot_approve (text : String) :
    MedenUIImpl.tui.approval.approves (.Paste text) = ok false := rfl

theorem modified_key_cannot_approve (key : KeyEvent) (h : key.modifiers.val ≠ 0) :
    MedenUIImpl.tui.approval.approves (.Key key) = ok false := by
  rw [approves_refines]
  cases hk : key.kind <;> cases hc : key.code <;> simp [expected, hk, hc, h]

theorem nonpress_cannot_approve (key : KeyEvent) (h : key.kind ≠ .Press) :
    MedenUIImpl.tui.approval.approves (.Key key) = ok false := by
  rw [approves_refines]
  cases hk : key.kind <;> simp_all [expected]

end MedenSpec.ApprovalRefinement
