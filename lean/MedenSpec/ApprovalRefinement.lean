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

abbrev has_content (width height : U16) : Prop := 2 < width.val ∧ 2 < height.val

abbrev heading_fits (width height rows : U16) : Prop :=
  3 < width.val ∧ 0 < rows.val ∧ rows.val + 2 ≤ height.val

def expected_route (screen : Screen) (e : Event) (width height rows : U16) : Option Choice :=
  if has_content width height ∧ heading_fits width height rows ∧ expected e = true then
    match screen with
    | .ReviewingProcessor => some .Processor
    | .ReviewingExport => some .Export
    | .ReviewingContract => some .Contract
    | _ => none
  else none

private theorem remaining_val (height : U16) :
    (core.num.U16.saturating_sub height (2#u16)).val = height.val - 2 := by
  change (max 0 (height.val - 2)) % (2^16) = height.val - 2
  rw [Nat.zero_max]
  exact Nat.mod_eq_of_lt (by scalar_tac)

theorem route_refines (screen : Screen) (e : Event) (width height rows : U16) :
    MedenUIImpl.tui.approval.route screen e width height rows =
      ok (expected_route screen e width height rows) := by
  by_cases hw : width.val ≤ 2 <;> by_cases hh : height.val ≤ 2 <;>
    by_cases hn : width.val = 3 <;> by_cases hr : rows.val = 0 <;>
    cases h : expected e <;> cases screen <;>
    simp [MedenUIImpl.tui.approval.route, approves_refines, expected_route,
      has_content, heading_fits, UScalar.le_equiv, UScalar.eq_equiv,
      UScalar.ofNatCore_val_eq, hw, hh, hn, hr, h, remaining_val, Aeneas.Std.lift, Aeneas.Std.bind_ok]

  all_goals (try split_ifs) <;> simp_all <;> omega

/-- Every target requires content cells and a complete heading inside the review border, its own
review screen and an actual unmodified lowercase-y key press. -/
theorem route_iff (screen : Screen) (e : Event) (width height rows : U16) (target : Choice) :
    MedenUIImpl.tui.approval.route screen e width height rows = ok (some target) ↔
      has_content width height ∧ heading_fits width height rows ∧ expected e = true ∧ screen = review_screen target := by
  rw [route_refines]
  by_cases hv : has_content width height <;> by_cases hf : heading_fits width height rows <;>
    cases target <;> cases screen <;> cases h : expected e <;>
    simp_all [expected_route, review_screen, has_content, heading_fits]

theorem paste_cannot_route (screen : Screen) (text : String) (width height rows : U16) :
    MedenUIImpl.tui.approval.route screen (.Paste text) width height rows = ok none := by
  rw [route_refines]
  simp [expected_route, expected]

theorem nonreview_cannot_route (screen : Screen) (e : Event) (width height rows : U16)
    (hp : screen ≠ .ReviewingProcessor) (he : screen ≠ .ReviewingExport)
    (hc : screen ≠ .ReviewingContract) :
    MedenUIImpl.tui.approval.route screen e width height rows = ok none := by
  rw [route_refines]
  cases screen <;> simp_all [expected_route]

theorem hidden_review_cannot_route (screen : Screen) (e : Event) (width height rows : U16)
    (h : width.val ≤ 2 ∨ height.val ≤ 2) :
    MedenUIImpl.tui.approval.route screen e width height rows = ok none := by
  rw [route_refines]
  have hn : ¬has_content width height := by unfold has_content; omega
  simp [expected_route, hn]

theorem clipped_heading_cannot_route (screen : Screen) (e : Event) (width height rows : U16)
    (h : width.val = 3 ∨ rows.val = 0 ∨ height.val < rows.val + 2) :
    MedenUIImpl.tui.approval.route screen e width height rows = ok none := by
  rw [route_refines]
  have hn : ¬heading_fits width height rows := by unfold heading_fits; omega
  simp [expected_route, hn]

end MedenSpec.ApprovalRefinement
