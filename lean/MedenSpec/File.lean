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

end MedenSpec.File
