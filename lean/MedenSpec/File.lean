import MedenSpec.Policy

/-! Declared content effects of a file operation, before filesystem resolution. -/
namespace MedenSpec.File

inductive Operation where
  | read | list | write | edit | snapshot
  deriving DecidableEq, Repr

structure Request (α : Type u) where
  operation : Operation
  path : String
  payload : α

def effects : Operation → List Capability
  | .read | .list | .snapshot => [.fsRead]
  | .write => [.fsWrite]
  | .edit => [.fsRead, .fsWrite]

def requirements (r : Request α) : List Requirement :=
  (effects r.operation).map fun cap => ⟨cap, some r.path⟩

theorem all_scoped (r : Request α) (requirement : Requirement)
    (member : requirement ∈ requirements r) : requirement.scope = some r.path := by
  obtain ⟨cap, _, same⟩ := List.mem_map.mp member
  cases same
  rfl

theorem permission_iff (p : Policy) (r : Request α) :
    p.allowed (requirements r) = true ↔
      ∀ cap ∈ effects r.operation,
        p.deniesCap cap = false ∧ p.grantsReq ⟨cap, some r.path⟩ = true := by
  simp [Policy.allowed_iff, requirements]

theorem edit_requires_both (p : Policy) (path : String) (payload : α) :
    p.allowed (requirements ⟨.edit, path, payload⟩) = true ↔
      (p.deniesCap .fsRead = false ∧ p.grantsReq ⟨.fsRead, some path⟩ = true) ∧
      (p.deniesCap .fsWrite = false ∧ p.grantsReq ⟨.fsWrite, some path⟩ = true) := by
  simp [permission_iff, effects]

structure Resolved (α : Type u) (ρ : Type v) where
  request : Request α
  canonical : String
  resource : ρ

def canonicalRequest (r : Resolved α ρ) : Request α :=
  { r.request with path := r.canonical }

def resolvedAllowed (p : Policy) (r : Resolved α ρ) : Bool :=
  p.allowed (requirements r.request) && p.allowed (requirements (canonicalRequest r))

def consume (p : Policy) (r : Resolved α ρ) : Option (Request α × String × ρ) :=
  if resolvedAllowed p r then some (r.request, r.canonical, r.resource) else none

theorem consume_iff (p : Policy) (r : Resolved α ρ) (out : Request α × String × ρ) :
    consume p r = some out ↔
      out = (r.request, r.canonical, r.resource) ∧
      p.allowed (requirements r.request) = true ∧
      p.allowed (requirements (canonicalRequest r)) = true := by
  by_cases h : resolvedAllowed p r = true
  · simp only [resolvedAllowed, Bool.and_eq_true] at h
    simp [consume, resolvedAllowed, h, eq_comm]
  · simp only [resolvedAllowed, Bool.and_eq_true] at h
    simp [consume, resolvedAllowed, h]

theorem resolved_effects (p : Policy) (r : Resolved α ρ)
    (out : Request α × String × ρ) (h : consume p r = some out) :
    ∀ cap ∈ effects r.request.operation,
      (p.deniesCap cap = false ∧ p.grantsReq ⟨cap, some r.request.path⟩ = true) ∧
      (p.deniesCap cap = false ∧ p.grantsReq ⟨cap, some r.canonical⟩ = true) := by
  obtain ⟨_, original, canonical⟩ := (consume_iff p r out).mp h
  intro cap member
  exact ⟨(permission_iff p r.request).mp original cap member,
    (permission_iff p (canonicalRequest r)).mp canonical cap member⟩

theorem refused_path_empty (p : Policy) (r : Resolved α ρ)
    (h : p.allowed (requirements r.request) = false ∨
      p.allowed (requirements (canonicalRequest r)) = false) : consume p r = none := by
  rcases h with h | h <;> simp [consume, resolvedAllowed, h]

end MedenSpec.File
