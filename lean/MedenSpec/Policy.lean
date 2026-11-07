import MedenSpec.Capability
import MedenSpec.Glob

/-!
# Policies and decisions

Mirrors `meden_core::policy`.
-/

namespace MedenSpec

/-- A granted capability, optionally restricted to scopes matching a glob. -/
structure Grant where
  cap : Capability
  scope : Option String
  deriving DecidableEq, Repr, Inhabited

/-- Does the grant cover the requirement? -/
def Grant.covers (g : Grant) (r : Requirement) : Bool :=
  g.cap == r.cap &&
    match g.scope, r.scope with
    | none, _ => true
    | some _, none => false
    | some p, some s => globMatch p.toList s.toList

/-- A named set of grants and denies. -/
structure Policy where
  name : String
  grants : List Grant
  denies : List Capability
  deriving Repr, Inhabited

namespace Policy

/-- Is the capability denied? -/
def deniesCap (p : Policy) (c : Capability) : Bool := p.denies.contains c

/-- Does some grant cover the requirement? -/
def grantsReq (p : Policy) (r : Requirement) : Bool := p.grants.any (·.covers r)

/-- A requirement is violated when it is denied or not granted. -/
def violates (p : Policy) (r : Requirement) : Bool := p.deniesCap r.cap || !p.grantsReq r

/-- The decision: allowed when no requirement is violated. -/
def allowed (p : Policy) (rs : List Requirement) : Bool := rs.all fun r => !p.violates r

theorem allowed_nil (p : Policy) : p.allowed [] = true := rfl

/-- Allowed exactly when every requirement is granted and none is denied. -/
theorem allowed_iff (p : Policy) (rs : List Requirement) :
    p.allowed rs = true ↔ ∀ r ∈ rs, p.deniesCap r.cap = false ∧ p.grantsReq r = true := by
  simp [allowed, violates, List.all_eq_true]

/-- Deny wins: a requirement on a denied capability is never allowed. -/
theorem deny_wins (p : Policy) (rs : List Requirement) (r : Requirement)
    (hr : r ∈ rs) (hd : p.deniesCap r.cap = true) : p.allowed rs = false := by
  cases hx : p.allowed rs with
  | false => rfl
  | true =>
    have hc := ((allowed_iff p rs).mp hx) r hr
    rw [hd] at hc
    exact Bool.noConfusion hc.1

/-- Adding a grant never turns an allowance into a denial. -/
theorem grants_monotone (p : Policy) (g : Grant) (rs : List Requirement)
    (h : p.allowed rs = true) : ({ p with grants := g :: p.grants } : Policy).allowed rs = true := by
  rw [allowed_iff] at h ⊢
  intro r hr
  obtain ⟨hd, hg⟩ := h r hr
  refine ⟨hd, ?_⟩
  simp only [grantsReq, List.any_cons] at hg ⊢
  simp [hg]

/-- Removing a deny never turns an allowance into a denial. -/
theorem denies_antitone (p : Policy) (c : Capability) (rs : List Requirement)
    (h : ({ p with denies := c :: p.denies } : Policy).allowed rs = true) : p.allowed rs = true := by
  rw [allowed_iff] at h ⊢
  intro r hr
  obtain ⟨hd, hg⟩ := h r hr
  refine ⟨?_, hg⟩
  simp only [deniesCap] at hd ⊢
  simp at hd ⊢
  exact hd.2

/-- An unscoped grant covers every requirement on its capability. -/
theorem covers_unscoped (c : Capability) (r : Requirement) (h : r.cap = c) :
    (Grant.mk c none).covers r = true := by
  simp [Grant.covers, h]

/-- A scoped grant never covers an unscoped requirement. -/
theorem scoped_grant_not_covers_unscoped (c : Capability) (s : String) :
    (Grant.mk c (some s)).covers (Requirement.mk c none) = false := by
  simp [Grant.covers]

end Policy

end MedenSpec
