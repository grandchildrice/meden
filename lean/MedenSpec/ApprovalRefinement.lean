import MedenSpec.Generated.Approval

/-! The production TUI approval predicate and review-target selector over
actual Crossterm events and screen states. Terminal decoding, physical input
provenance, screen transitions, rendering and I/O remain separate obligations.
No text is reclassified as a key by these functions. -/
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

open MedenUIImpl.tui (Screen)
open MedenUIImpl.tui.approval (Choice)

def review_screen : Choice → Screen
  | .Processor => .ReviewingProcessor
  | .Export => .ReviewingExport
  | .Contract => .ReviewingContract

def expected_route (screen : Screen) (e : Event) : Option Choice :=
  if expected e then
    match screen with
    | .ReviewingProcessor => some .Processor
    | .ReviewingExport => some .Export
    | .ReviewingContract => some .Contract
    | _ => none
  else none

theorem route_refines (screen : Screen) (e : Event) :
    MedenUIImpl.tui.approval.route screen e = ok (expected_route screen e) := by
  cases h : expected e <;> cases screen <;>
    simp [MedenUIImpl.tui.approval.route, approves_refines, expected_route, h]

/-- Every approval target is paired with its own review-screen state
and an actual unmodified lowercase-y key press. -/
theorem route_iff (screen : Screen) (e : Event) (target : Choice) :
    MedenUIImpl.tui.approval.route screen e = ok (some target) ↔
      expected e = true ∧ screen = review_screen target := by
  rw [route_refines]
  cases target <;> cases screen <;> cases h : expected e <;>
    simp [expected_route, review_screen, h]

theorem paste_cannot_route (screen : Screen) (text : String) :
    MedenUIImpl.tui.approval.route screen (.Paste text) = ok none := by
  rw [route_refines]
  rfl

theorem nonreview_cannot_route (screen : Screen) (e : Event)
    (hp : screen ≠ .ReviewingProcessor) (he : screen ≠ .ReviewingExport)
    (hc : screen ≠ .ReviewingContract) :
    MedenUIImpl.tui.approval.route screen e = ok none := by
  rw [route_refines]
  cases screen <;> cases h : expected e <;> simp_all [expected_route]

end MedenSpec.ApprovalRefinement
