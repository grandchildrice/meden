import MedenSpec.Glob
import MedenSpec.Generated.Policy
import Mathlib.Tactic.SplitIfs

namespace MedenSpec.GlobRefinement
open Aeneas Aeneas.Std Result ControlFlow MedenPolicyImpl
open Aeneas.Std.WP
set_option maxHeartbeats 2000000

def cell (pc c : Char) (old oldRight newRight : Bool) : Bool :=
  if pc = '*' then old || newRight
  else if pc = '?' then oldRight else if pc = c then oldRight else false

private theorem drop_head (text : List Char) (i : Nat) (hi : i < text.length) :
    text.drop i = text[i]! :: text.drop (i + 1) := by
  induction text generalizing i with
  | nil => simp at hi
  | cons c rest ih =>
    cases i with
    | zero => simp
    | succ i => simpa using ih i (by simpa using hi)

theorem cell_refines (pc : Char) (pattern text : List Char) (i : Nat)
    (hi : i < text.length) :
    cell pc text[i]! (globMatch pattern (text.drop i))
      (globMatch pattern (text.drop (i + 1))) (globMatch (pc :: pattern) (text.drop (i + 1))) =
        globMatch (pc :: pattern) (text.drop i) := by
  rw [drop_head text i hi]
  by_cases hs : pc = '*'
  · subst pc; simp [cell, globMatch]
  · by_cases hq : pc = '?'
    · subst pc; simp [cell, globMatch]
    · simp [cell, globMatch, hs, hq]
      apply Bool.eq_iff_iff.mpr
      simp

-- A mathematical evaluation of one row, used only in the refinement proof.
def rowLoop (pc : Char) (text : List Char) (row : List Bool)
    (oldRight newRight : Bool) : Nat → List Bool
  | 0 => row
  | i + 1 =>
    let old := row[i]!
    let value := cell pc text[i]! old oldRight newRight
    rowLoop pc text (row.set i value) old value i

theorem rowLoop_length (pc : Char) (text : List Char) (row : List Bool)
    (oldRight newRight : Bool) (i : Nat) :
    (rowLoop pc text row oldRight newRight i).length = row.length := by
  induction i generalizing row oldRight newRight with
  | zero => rfl
  | succ i ih => simp [rowLoop, ih]

private theorem get_set_other (row : List Bool) (i j : Nat) (value : Bool) (hne : j ≠ i) :
    (row.set i value)[j]! = row[j]! := by
  simp [List.getElem!_eq_getElem?_getD, hne, Ne.symm hne]

theorem rowLoop_correct (pc : Char) (pattern text : List Char) (row : List Bool)
    (oldRight newRight : Bool) (i : Nat) (hl : row.length = text.length)
    (hi : i ≤ text.length)
    (hp : ∀ j < i, row[j]! = globMatch pattern (text.drop j))
    (ho : oldRight = globMatch pattern (text.drop i))
    (hn : newRight = globMatch (pc :: pattern) (text.drop i)) :
    ∀ j < text.length,
      (rowLoop pc text row oldRight newRight i)[j]! =
        if j < i then globMatch (pc :: pattern) (text.drop j) else row[j]! := by
  induction i generalizing row oldRight newRight with
  | zero => simp [rowLoop]
  | succ i ih =>
    have hit : i < text.length := by omega
    have hir : i < row.length := by omega
    have hval : cell pc text[i]! row[i]! oldRight newRight = globMatch (pc :: pattern) (text.drop i) := by
      rw [hp i (by omega), ho, hn]
      exact cell_refines pc pattern text i hit
    have hprefix : ∀ j < i, (row.set i (cell pc text[i]! row[i]! oldRight newRight))[j]! =
        globMatch pattern (text.drop j) := by
      intro j hj
      rw [get_set_other _ _ _ _ (by omega)]
      exact hp j (by omega)
    have hrec := ih (row.set i (cell pc text[i]! row[i]! oldRight newRight)) row[i]!
      (cell pc text[i]! row[i]! oldRight newRight) (by simpa using hl) (by omega)
      hprefix (hp i (by omega)) hval
    intro j hj
    rw [rowLoop, hrec j hj]
    by_cases hji : j < i
    · simp [hji, show j < i + 1 by omega]
    · by_cases he : j = i
      · subst j
        simpa [hir] using hval
      · rw [get_set_other _ _ _ _ he]
        simp [hji, show ¬j < i + 1 by omega]

def GoodRow (pattern text : List Char) (row : alloc.vec.Vec Bool) : Prop :=
  row.val.length = text.length ∧ ∀ j < text.length, row.val[j]! = globMatch pattern (text.drop j)

private theorem native_body (pc : Char) (text : Slice Char) (row : alloc.vec.Vec Bool)
    (oldRight newRight : Bool) (index : Usize)
    (hl : row.val.length = text.val.length) (hi : index.val ≤ text.val.length)
    (hpos : 0 < index.val) :
    glob.advance_row_loop.body pc text row oldRight newRight index ⦃ out =>
      ∃ j : Usize, j.val + 1 = index.val ∧
        out = .cont (row.set j (cell pc text.val[j.val]! row.val[j.val]! oldRight newRight),
          row.val[j.val]!, cell pc text.val[j.val]! row.val[j.val]! oldRight newRight, j) ⦄ := by
  unfold glob.advance_row_loop.body
  have hg : index > 0#usize := by scalar_tac
  simp only [if_pos hg]
  step as ⟨j, hj⟩
  have hjt : j.val < text.val.length := by omega
  have hjr : j.val < row.val.length := by omega
  step as ⟨old, hold⟩
  repeat' (first | step | split)
  all_goals
    have hit : index.val - 1 < text.val.length := by omega
    have hir : index.val - 1 < row.val.length := by omega
    simp_all [cell, List.getElem!_eq_getElem?_getD,
      List.getElem?_eq_getElem hit, List.getElem?_eq_getElem hir]

@[step] theorem advance_loop_refines (pc : Char) (text : Slice Char) (row : alloc.vec.Vec Bool)
    (oldRight newRight : Bool) (index : Usize)
    (hl : row.val.length = text.val.length) (hi : index.val ≤ text.val.length) :
    glob.advance_row_loop pc text row oldRight newRight index ⦃ out =>
      out.val = rowLoop pc text.val row.val oldRight newRight index.val ⦄ := by
  unfold glob.advance_row_loop
  apply Aeneas.Std.loop.spec_decr_nat
    (fun s : alloc.vec.Vec Bool × Bool × Bool × Usize => s.2.2.2.val)
    (fun s : alloc.vec.Vec Bool × Bool × Bool × Usize => s.1.val.length = text.val.length ∧ s.2.2.2.val ≤ text.val.length ∧
      rowLoop pc text.val s.1.val s.2.1 s.2.2.1 s.2.2.2.val =
        rowLoop pc text.val row.val oldRight newRight index.val)
    (fun out : alloc.vec.Vec Bool => out.val = rowLoop pc text.val row.val oldRight newRight index.val)
    (fun (r, o, n, i) => glob.advance_row_loop.body pc text r o n i) (row, oldRight, newRight, index)
  · rintro ⟨current, old, new, i⟩ ⟨hsize, hbound, heval⟩
    dsimp only at *
    by_cases hp : 0 < i.val
    · apply spec_mono (native_body pc text current old new i hsize hbound hp)
      rintro out ⟨j, hj, rfl⟩
      simp only
      refine ⟨⟨by simpa using hsize, by omega, ?_⟩, by omega⟩
      rw [alloc.vec.Vec.set_val_eq]
      simpa only [← hj, rowLoop] using heval
    · have hz : i = 0#usize := by scalar_tac
      subst i
      simpa [glob.advance_row_loop.body, rowLoop] using heval
  · exact ⟨hl, hi, rfl⟩

private theorem empty_cons (pc : Char) (pattern : List Char) :
    globMatch (pc :: pattern) [] = if pc = '*' then globMatch pattern [] else false := by
  by_cases hs : pc = '*'
  · subst pc; simp [globMatch]
  · by_cases hq : pc = '?'
    · subst pc; simp [globMatch]
    · simp [globMatch, hs, hq]

@[step] theorem advance_row_refines (pc : Char) (pattern : List Char) (text : Slice Char)
    (row : alloc.vec.Vec Bool) (empty : Bool) (hr : GoodRow pattern text.val row)
    (he : empty = globMatch pattern []) :
    glob.advance_row pc text row empty ⦃ out nextEmpty =>
      GoodRow (pc :: pattern) text.val out ∧ nextEmpty = globMatch (pc :: pattern) [] ⦄ := by
  have he' : (if pc = '*' then empty else false) = globMatch (pc :: pattern) [] := by
    simp [empty_cons, he]
  have hexpand : glob.advance_row pc text row empty = (do
      let out ← glob.advance_row_loop pc text row empty (if pc = '*' then empty else false) (Slice.len text)
      ok (out, if pc = '*' then empty else false)) := by
    by_cases hs : pc = '*' <;> simp [glob.advance_row, hs]
  rw [hexpand]
  apply spec_bind (advance_loop_refines pc text row empty _ (Slice.len text) hr.1 (by simp))
  intro out hout
  simp only [spec_ok, uncurry'_pair]
  constructor
  · constructor
    · rw [hout, rowLoop_length, hr.1]
    · intro j hj
      have hc := rowLoop_correct pc pattern text.val row.val empty
        (if pc = '*' then empty else false) text.val.length hr.1
        (by rfl) (fun k hk => hr.2 k hk) (by simpa using he) (by simpa using he') j hj
      simpa [hout, hj] using hc
  · exact he'

@[step] theorem match_loop_refines (pattern text : Slice Char) (row : alloc.vec.Vec Bool)
    (empty : Bool) (index : Usize) (hi : index.val ≤ pattern.val.length)
    (hr : GoodRow (pattern.val.drop index.val) text.val row)
    (he : empty = globMatch (pattern.val.drop index.val) []) :
    glob.glob_match_chars_loop pattern text row empty index ⦃ out nextEmpty =>
      GoodRow pattern.val text.val out ∧ nextEmpty = globMatch pattern.val [] ⦄ := by
  unfold glob.glob_match_chars_loop
  apply Aeneas.Std.loop.spec_decr_nat
    (fun s : alloc.vec.Vec Bool × Bool × Usize => s.2.2.val)
    (fun s : alloc.vec.Vec Bool × Bool × Usize =>
      s.2.2.val ≤ pattern.val.length ∧ GoodRow (pattern.val.drop s.2.2.val) text.val s.1 ∧
        s.2.1 = globMatch (pattern.val.drop s.2.2.val) [])
    (fun out : alloc.vec.Vec Bool × Bool => GoodRow pattern.val text.val out.1 ∧
      out.2 = globMatch pattern.val [])
    (fun (r, e, i) => glob.glob_match_chars_loop.body pattern text r e i) (row, empty, index)
  · rintro ⟨current, e, i⟩ ⟨hbound, hrow, hempty⟩
    dsimp only at *
    unfold glob.glob_match_chars_loop.body
    by_cases hp : 0 < i.val
    · have hg : i > 0#usize := by scalar_tac
      simp only [if_pos hg]
      step as ⟨j, hj⟩
      have hjlt : j.val < pattern.val.length := by omega
      have hji : j.val + 1 = i.val := by omega
      step as ⟨pc, hpc⟩
      apply spec_bind (advance_row_refines pc (pattern.val.drop i.val) text current e hrow hempty)
      rintro ⟨next, nextEmpty⟩ ⟨hnext, hne⟩
      have hsuffix : pc :: pattern.val.drop i.val = pattern.val.drop j.val := by
        rw [drop_head pattern.val j.val hjlt]
        simp [← hji, hpc, getElem!_pos pattern.val j.val hjlt]
      apply (spec_ok _).mpr
      dsimp only
      exact ⟨by omega, by simpa [hsuffix] using hnext, by simpa [hsuffix] using hne, by omega⟩
    · have hz : i = 0#usize := by scalar_tac
      subst i
      simpa [GoodRow, glob.glob_match_chars_loop.body] using And.intro hrow hempty
  · exact ⟨hi, hr, he⟩

theorem glob_match_chars_refines (pattern text : Slice Char) :
    glob.glob_match_chars pattern text = ok (globMatch pattern.val text.val) := by
  have total : glob.glob_match_chars pattern text ⦃ result => result = globMatch pattern.val text.val ⦄ := by
    unfold glob.glob_match_chars
    step as ⟨row, hrow, hlen⟩
    have hinit : GoodRow (pattern.val.drop pattern.val.length) text.val row := by
      constructor
      · simpa using hlen
      · intro j hj
        have hdrop : text.val.drop j ≠ [] := by
          intro h
          have hle := List.drop_eq_nil_iff.mp h
          omega
        cases hd : text.val.drop j with
        | nil => contradiction
        | cons c rest => simp [hrow, hj, globMatch, hd]
    apply spec_bind (match_loop_refines pattern text row true (Slice.len pattern) (by simp) hinit (by simp [globMatch]))
    rintro ⟨out, empty⟩ ⟨hout, he⟩
    step as ⟨isEmpty, hEmpty⟩
    split <;> rename_i hb
    · simp_all
    · have hzero : text.length ≠ 0 := by
        intro hz
        exact hb (Eq.mpr hEmpty hz)
      have hnonempty : 0 < text.val.length := by simpa using Nat.pos_of_ne_zero hzero
      rcases hout with ⟨houtLen, houtValues⟩
      have hbound : 0 < out.val.length := by omega
      step as ⟨value, hvalue⟩
      simpa [hvalue, getElem!_pos out.val 0 hbound] using houtValues 0 hnonempty
  cases h : glob.glob_match_chars pattern text with
  | ret value => simp only [h, spec_ok] at total; simp [h, total]
  | vis effect continuation => simp [h] at total
  | div => simp [h] at total

theorem extracted_star (pattern text : Slice Char) (hp : pattern.val = ['*']) :
    glob.glob_match_chars pattern text = ok true := by
  rw [glob_match_chars_refines]
  simp [hp, globMatch_star]

theorem extracted_literal (pattern text : Slice Char) (h : noWild pattern.val = true) :
    glob.glob_match_chars pattern text = ok (pattern.val == text.val) := by
  rw [glob_match_chars_refines, globMatch_literal _ _ h]

end MedenSpec.GlobRefinement
