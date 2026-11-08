import Lean.Elab.Tactic.Omega

namespace LedgerContract

def maximum : Nat := 18446744073709551615

structure State where
  debit : Nat
  credit : Nat
  sequence : Nat
  deriving DecidableEq

abbrev permitted (s : State) (actor id amount : Nat) : Prop :=
  actor = 0 ∧ s.sequence < id ∧ 0 < amount ∧ amount ≤ s.debit ∧
    s.credit + amount ≤ maximum

def transfer (s : State) (actor id amount : Nat) : State :=
  if permitted s actor id amount then
    ⟨s.debit - amount, s.credit + amount, id⟩
  else s

theorem conservation (s : State) (actor id amount : Nat) :
    (transfer s actor id amount).debit + (transfer s actor id amount).credit = s.debit + s.credit := by
  unfold transfer
  split
  · rename_i h
    dsimp only [permitted] at h
    dsimp
    omega
  · rfl

theorem rejection (s : State) (actor id amount : Nat) (h : ¬ permitted s actor id amount) :
    transfer s actor id amount = s := by simp [transfer, h]

theorem valid_transfer (s : State) (actor id amount : Nat) (h : permitted s actor id amount) :
    transfer s actor id amount = ⟨s.debit - amount, s.credit + amount, id⟩ := by simp [transfer, h]

theorem unauthorized (s : State) (actor id amount : Nat) (h : actor ≠ 0) :
    transfer s actor id amount = s := by simp [transfer, permitted, h]

theorem old_request (s : State) (actor id amount : Nat) (h : id ≤ s.sequence) :
    transfer s actor id amount = s := by
  have hn : ¬ s.sequence < id := by omega
  simp [transfer, permitted, hn]

theorem repeated_request (s : State) (actor id amount : Nat) :
    transfer (transfer s actor id amount) actor id amount = transfer s actor id amount := by
  unfold transfer
  split
  · simp [permitted]
  · simp_all

theorem bounded_result (s : State) (actor id amount : Nat)
    (hd : s.debit ≤ maximum) (hc : s.credit ≤ maximum)
    (hs : s.sequence ≤ maximum) (hi : id ≤ maximum) :
    (transfer s actor id amount).debit ≤ maximum ∧
    (transfer s actor id amount).credit ≤ maximum ∧
    (transfer s actor id amount).sequence ≤ maximum := by
  unfold transfer
  split
  · rename_i h
    dsimp only [permitted] at h
    dsimp
    omega
  · exact ⟨hd, hc, hs⟩

end LedgerContract
