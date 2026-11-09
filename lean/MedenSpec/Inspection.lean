import MedenSpec.Contracts

/-! A prepared read may consume only its original complete contract context. -/
namespace MedenSpec.Inspection

structure Snapshot where
  session : List Nat
  sequence : Nat
  accepted : Option Contracts.View
  displayed : Option Contracts.View
  ready : Bool
  deriving DecidableEq

def capture (s : Contracts.State) : Snapshot :=
  ⟨s.session, s.sequence, Contracts.accepted s, Contracts.displayed s, Contracts.isAccepted s⟩

def consume (frozen : Snapshot) (current : Contracts.State) : Option (Option Contracts.View) :=
  if frozen = capture current then some frozen.accepted else none

theorem consume_iff (frozen : Snapshot) (current : Contracts.State) (view : Option Contracts.View) :
    consume frozen current = some view ↔ frozen = capture current ∧ view = frozen.accepted := by
  unfold consume
  by_cases same : frozen = capture current
  · rw [if_pos same]
    constructor
    · intro success
      exact ⟨same, (Option.some.inj success).symm⟩
    · rintro ⟨_, result⟩
      exact congrArg some result.symm
  · rw [if_neg same]
    simp [same]

theorem original_only (frozen : Snapshot) (current : Contracts.State) (view : Option Contracts.View)
    (success : consume frozen current = some view) : view = frozen.accepted :=
  ((consume_iff frozen current view).mp success).2

theorem changed_refused (frozen : Snapshot) (current : Contracts.State)
    (changed : frozen ≠ capture current) : consume frozen current = none := by
  simp [consume, changed]

theorem generation_refused (frozen : Snapshot) (current : Contracts.State)
    (advanced : frozen.sequence < current.sequence) : consume frozen current = none := by
  apply changed_refused
  intro same
  have h := congrArg Snapshot.sequence same
  simp only [capture] at h
  omega

theorem stays_stale (frozen : Snapshot) (current : Contracts.State) (actions : List Contracts.Action)
    (advanced : frozen.sequence < current.sequence) :
    consume frozen (Contracts.run current actions) = none := by
  apply generation_refused
  exact Nat.lt_of_lt_of_le advanced (Contracts.run_sequence_monotone current actions)

theorem no_new_accepted (frozen : Snapshot) (current : Contracts.State) (view : Option Contracts.View)
    (empty : frozen.accepted = none) (success : consume frozen current = some view) : view = none := by
  exact (original_only frozen current view success).trans empty

end MedenSpec.Inspection
