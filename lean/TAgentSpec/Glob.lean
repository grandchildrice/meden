/-!
# Scope globs

`*` matches any sequence of characters (including the empty one), `?`
matches exactly one character, every other character matches itself.
Mirrors `tagent_core::glob`, which implements the same relation with an
iterative algorithm; this is the reference definition.
-/
namespace TAgentSpec

/-- Does the pattern match the text? -/
def globMatch : List Char → List Char → Bool
  | [], [] => true
  | [], _ :: _ => false
  | '*' :: pr, [] => globMatch pr []
  | '*' :: pr, tc :: tr => globMatch pr (tc :: tr) || globMatch ('*' :: pr) tr
  | '?' :: pr, _ :: tr => globMatch pr tr
  | '?' :: _, [] => false
  | pc :: pr, tc :: tr => pc == tc && globMatch pr tr
  | _ :: _, [] => false
termination_by p t => p.length + t.length

/-- A pattern with no wildcard. -/
def noWild (p : List Char) : Bool := p.all fun c => c != '*' && c != '?'

theorem globMatch_nil_nil : globMatch [] [] = true := by simp [globMatch]

theorem globMatch_nil_cons (c : Char) (t : List Char) : globMatch [] (c :: t) = false := by
  simp [globMatch]

/-- A lone `*` matches everything. -/
theorem globMatch_star (t : List Char) : globMatch ['*'] t = true := by
  induction t with
  | nil => simp [globMatch]
  | cons c tr ih => simp [globMatch, ih]

/-- A wildcard-free pattern matches exactly itself. -/
theorem globMatch_literal (p t : List Char) (h : noWild p = true) :
    globMatch p t = (p == t) := by
  induction p generalizing t with
  | nil => cases t <;> simp [globMatch]
  | cons pc pr ih =>
    rw [noWild, List.all_cons, Bool.and_eq_true] at h
    obtain ⟨hpc, hpr⟩ := h
    have hpr' : noWild pr = true := hpr
    have hstar : pc ≠ '*' := fun e => by subst e; simp at hpc
    have hq : pc ≠ '?' := fun e => by subst e; simp at hpc
    cases t with
    | nil => simp [globMatch]
    | cons tc tr => simp [globMatch, ih tr hpr']

end TAgentSpec
