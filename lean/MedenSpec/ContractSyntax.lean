import Terms

/-! The actual offline checker is imported from verification/kernel/Terms.lean.
The relation describes admitted syntax trees, not parser or elaborator behavior.
Hygiene metadata is explicitly opaque; label nodes have their own grammar. -/
namespace MedenSpec.ContractSyntax
open Lean MedenTerms
set_option linter.unusedSimpArgs false

-- Normative grammar: this list is independent of the executable whitelist.
def kinds : List String := [
  "null", "num", "termIfThenElse", "Lean.Parser.Term.ifThenElse",
  "Lean.Parser.Term.app", "Lean.Parser.Term.paren",
  "Lean.Parser.Term.typeAscription", "Lean.Parser.Term.hygienicLParen",
  "Lean.Parser.Term.structInst", "Lean.Parser.Term.structInstFields",
  "Lean.Parser.Term.structInstField", "Lean.Parser.Term.structInstLVal",
  "Lean.Parser.Term.structInstFieldDef", "Lean.Parser.Term.optEllipsis",
  "Lean.Parser.Term.arrow", "term_+_", "term_-_", "term_*_", "term_/_", "term_%_", "term_=_", "term_≠_",
  "term_<_", "term_≤_", "term_>_", "term_≥_", "term_∧_", "term_∨_",
  "term¬_", "term_&&_", "term_||_", "term_==_", "term_!=_", "term!_"
]

theorem kinds_exact : allowedKinds = kinds := by decide

inductive AtomWithin (token : String) : Nat → Syntax → Prop where
  | here (fuel info) : AtomWithin token (fuel + 1) (.atom info token)
  | child {fuel stx arg} : arg ∈ stx.getArgs.toList →
      AtomWithin token fuel arg → AtomWithin token (fuel + 1) stx

inductive Admissible (allowed fields : List Name) : Syntax → Prop where
  | atom (info token) : Admissible allowed fields (.atom info token)
  | ident (info raw name pre) : name ∈ allowed →
      Admissible allowed fields (.ident info raw name pre)
  | hygiene (info args) : Admissible allowed fields (.node info `hygieneInfo args)
  | label (info idInfo raw name pre tailInfo) : name ∈ fields →
      Admissible allowed fields (.node info `Lean.Parser.Term.structInstLVal
        #[.ident idInfo raw name pre, .node tailInfo `null #[]])
  | node (info kind args) :
      kind ≠ `hygieneInfo → kind ≠ `Lean.Parser.Term.structInstLVal →
      kind.toString false ∈ kinds →
      (kind = `Lean.Parser.Term.structInstField →
        AtomWithin ":=" 128 (.node info kind args)) →
      (kind = `Lean.Parser.Term.optEllipsis →
        ¬AtomWithin ".." 128 (.node info kind args)) →
      (∀ arg ∈ args.toList, Admissible allowed fields arg) →
      Admissible allowed fields (.node info kind args)

private theorem except_ok {α β : Type} (value : α) (f : α → Except String β) :
    (Except.ok value >>= f) = f value := rfl

private theorem except_error {α β : Type} (message : String) (f : α → Except String β) :
    (Except.error message >>= f) = .error message := rfl

private theorem except_pure {α : Type} (value : α) :
    (pure value : Except String α) = .ok value := rfl

private theorem except_throw {α : Type} (message : String) :
    (throw message : Except String α) = .error message := rfl

theorem hasAtom_iff (token : String) (fuel : Nat) (stx : Syntax) :
    hasAtom token fuel stx = true ↔ AtomWithin token fuel stx := by
  induction fuel generalizing stx with
  | zero => simp only [hasAtom, Bool.false_eq_true, false_iff]; intro h; cases h
  | succ fuel ih =>
    simp only [hasAtom, Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq]
    constructor
    · rintro (h | h)
      · rcases h with ⟨h, eq⟩
        cases stx <;> simp_all [Syntax.isAtom, Syntax.getAtomVal]
        subst_vars
        exact .here _ _
      · rw [← Array.any_toList] at h
        have ⟨arg, mem, yes⟩ := List.any_eq_true.mp h
        exact .child mem ((ih arg).mp yes)
    · intro h
      cases h with
      | here => exact Or.inl (by simp [Syntax.isAtom, Syntax.getAtomVal])
      | child mem yes =>
        apply Or.inr
        rw [← Array.any_toList]
        exact List.any_eq_true.mpr ⟨_, mem, (ih _).mpr yes⟩

private theorem children_checked (f : Syntax → Except String Unit)
    (args : List Syntax)
    (h : args.foldlM (fun _ arg => f arg) () = .ok ()) :
    ∀ arg ∈ args, f arg = .ok () := by
  induction args with
  | nil => simp
  | cons arg rest ih =>
    simp only [List.foldlM_cons, bind, Except.bind] at h
    cases he : f arg with
    | error error => simp [he] at h
    | ok result =>
      cases result
      simp only [he] at h
      simp only [List.mem_cons, forall_eq_or_imp]
      exact ⟨he, ih h⟩

private theorem loop_fold (f : Syntax → Except String Unit) (args : Array Syntax) :
    (forIn args () fun arg _ => do f arg; pure (.yield ())) =
      args.foldlM (fun _ arg => f arg) () := by
  have unit_map (r : Except String Unit) : (fun _ => ()) <$> r = r := by
    cases r with
    | error => rfl
    | ok result => cases result; rfl
  change (forIn args () fun arg _ => (fun _ : Unit => .yield ()) <$> f arg) = _
  simpa only [unit_map] using
    (Array.forIn_yield_eq_foldlM (fun arg _ => f arg) (fun _ _ _ => ()) ())

theorem accepted_admissible (allowed fields : List Name) (fuel : Nat) (stx : Syntax)
    (h : checkSyntax allowed fields fuel stx = .ok ()) :
    Admissible allowed fields stx := by
  induction fuel generalizing stx with
  | zero => simp [checkSyntax] at h
  | succ fuel ih =>
    cases stx with
    | missing => simp [checkSyntax] at h
    | atom info token => exact .atom info token
    | ident info raw name pre =>
      rw [checkSyntax.eq_4] at h
      simp only [Bool.not_eq_true, Bool.and_eq_true, beq_iff_eq,
        List.contains_iff_mem, except_ok, except_error, except_pure, except_throw] at h
      split at h <;> simp_all
      exact .ident _ _ _ _ (by assumption)
    | node info kind args =>
      rw [checkSyntax.eq_5] at h
      simp only [Bool.not_eq_true, Bool.and_eq_true, beq_iff_eq,
        List.contains_iff_mem, except_ok, except_error, except_pure, except_throw] at h
      split at h
      · rename_i hk; subst kind; exact .hygiene _ _
      · rename_i hk
        split at h
        · simp at h
        · rename_i hp
          split at h
          · simp at h
          · rename_i hd
            split at h
            · rename_i ha
              split at h
              · rename_i hl
                subst kind
                split at h
                · rename_i idInfo raw name pre tailInfo tailKind tail shape
                  split at h
                  · rename_i ht
                    split at h
                    · rename_i hf
                      have eq : args = #[.ident idInfo raw name pre,
                          .node tailInfo `null #[]] := by
                        simp only [Array.isEmpty_iff] at ht
                        rcases ht with ⟨rfl, rfl⟩
                        simpa using congrArg List.toArray shape
                      rw [eq]
                      exact .label _ _ _ _ _ _ hf
                    · simp at h
                  · simp at h
                · simp at h
              · rename_i hl
                apply Admissible.node _ _ _ hk hl (by simpa only [kinds_exact] using ha)
                · intro eq
                  apply (hasAtom_iff _ _ _).mp
                  simp_all
                · intro eq yes
                  have := (hasAtom_iff _ _ _).mpr yes
                  simp_all
                · intro arg mem
                  apply ih arg
                  apply children_checked (checkSyntax allowed fields fuel) args.toList
                  · have checked : ((forIn args () fun arg _ => do
                      checkSyntax allowed fields fuel arg
                      pure (.yield ())) >>= fun _ => pure ()) = .ok () := h
                    rw [loop_fold] at checked
                    cases he : args.foldlM
                        (fun _ arg => checkSyntax allowed fields fuel arg) () with
                    | error error => simp [he, bind, Except.bind] at checked
                    | ok result =>
                      cases result
                      simpa only [Array.foldlM_toList] using he
                  · exact mem
            · simp at h

theorem accepted_identifier (allowed fields : List Name) (fuel info raw name pre)
    (h : checkSyntax allowed fields fuel (.ident info raw name pre) = .ok ()) :
    name ∈ allowed := by
  cases accepted_admissible _ _ _ _ h with
  | ident _ _ _ _ mem => exact mem

theorem accepted_node_kind (allowed fields : List Name) (fuel info kind args)
    (h : checkSyntax allowed fields fuel (.node info kind args) = .ok ()) :
    kind = `hygieneInfo ∨ kind.toString false ∈ kinds := by
  cases accepted_admissible _ _ _ _ h with
  | hygiene => exact Or.inl rfl
  | label => exact Or.inr (by decide)
  | node _ _ _ _ _ mem => exact Or.inr mem

theorem accepted_label (allowed fields : List Name) (fuel info args)
    (h : checkSyntax allowed fields fuel
      (.node info `Lean.Parser.Term.structInstLVal args) = .ok ()) :
    ∃ idInfo raw name pre tailInfo, name ∈ fields ∧
      args = #[.ident idInfo raw name pre, .node tailInfo `null #[]] := by
  cases accepted_admissible _ _ _ _ h with
  | label info idInfo raw name pre tailInfo mem => exact ⟨_, _, _, _, _, mem, rfl⟩
  | node _ _ _ _ notLabel => exact (notLabel rfl).elim

private theorem record_guards (allowed fields : List Name) (fuel info kind args)
    (h : checkSyntax allowed fields fuel (.node info kind args) = .ok ()) :
    (kind = `Lean.Parser.Term.structInstField →
      AtomWithin ":=" 128 (.node info kind args)) ∧
    (kind = `Lean.Parser.Term.optEllipsis →
      ¬AtomWithin ".." 128 (.node info kind args)) := by
  cases accepted_admissible _ _ _ _ h with
  | hygiene => simp
  | label => simp
  | node _ _ _ _ _ _ explicit noDefault => exact ⟨explicit, noDefault⟩

theorem accepted_record_field (allowed fields : List Name) (fuel info args)
    (h : checkSyntax allowed fields fuel
      (.node info `Lean.Parser.Term.structInstField args) = .ok ()) :
    AtomWithin ":=" 128 (.node info `Lean.Parser.Term.structInstField args) :=
  (record_guards _ _ _ _ _ _ h).1 rfl

theorem accepted_no_defaults (allowed fields : List Name) (fuel info args)
    (h : checkSyntax allowed fields fuel
      (.node info `Lean.Parser.Term.optEllipsis args) = .ok ()) :
    ¬AtomWithin ".." 128 (.node info `Lean.Parser.Term.optEllipsis args) :=
  (record_guards _ _ _ _ _ _ h).2 rfl

end MedenSpec.ContractSyntax
