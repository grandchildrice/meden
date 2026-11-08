import Candidate
import Spec

open Aeneas Aeneas.Std Result
namespace LedgerProof
set_option maxHeartbeats 2000000

def abstractState (s : LedgerImpl.ledger.LedgerState) : LedgerContract.State :=
  ⟨s.debit.val, s.credit.val, s.sequence.val⟩

private theorem saturating_sub_val (a b : U64) :
    (core.num.U64.saturating_sub a b).val = a.val - b.val := by
  change (max 0 (a.val - b.val)) % (2^64) = a.val - b.val
  rw [Nat.zero_max]
  exact Nat.mod_eq_of_lt (by scalar_tac)

private theorem saturating_add_val (a b : U64) :
    (core.num.U64.saturating_add a b).val = min 18446744073709551615 (a.val + b.val) := by
  change (min (UScalar.max .U64) (a.val + b.val)) % (2^64) =
    min 18446744073709551615 (a.val + b.val)
  have hm : UScalar.max .U64 = 18446744073709551615 := by
    simp only [UScalar.max_UScalarTy_U64_eq, U64.max_eq]
  rw [hm]
  exact Nat.mod_eq_of_lt (Nat.lt_of_le_of_lt (Nat.min_le_left _ _) (by decide))

theorem actual_spec (s : LedgerImpl.ledger.LedgerState) (actor id amount : U64) :
    LedgerImpl.ledger.transfer s actor id amount ⦃ out =>
      abstractState out = LedgerContract.transfer (abstractState s) actor.val id.val amount.val ⦄ := by
  unfold LedgerImpl.ledger.transfer
  try simp only [Aeneas.Std.lift, Aeneas.Std.bind_ok]
  step*
  all_goals simp_all [abstractState, LedgerContract.transfer, LedgerContract.permitted,
    LedgerContract.maximum, U64.rMax, UScalar.eq_equiv, saturating_sub_val, saturating_add_val]
  all_goals (try split_ifs) <;> (try congr 1) <;> scalar_tac

theorem actual_refines (s : LedgerImpl.ledger.LedgerState) (actor id amount : U64) :
    (do let out ← LedgerImpl.ledger.transfer s actor id amount; ok (abstractState out)) =
      ok (LedgerContract.transfer (abstractState s) actor.val id.val amount.val) := by
  have h := actual_spec s actor id amount
  cases he : LedgerImpl.ledger.transfer s actor id amount with
  | ret out => simpa [he] using congrArg ok (by simpa [he] using h)
  | vis effect continuation => simp [he] at h
  | div => simp [he] at h

end LedgerProof

#print axioms LedgerProof.actual_refines
#print axioms LedgerContract.conservation
#print axioms LedgerContract.rejection
#print axioms LedgerContract.valid_transfer
#print axioms LedgerContract.unauthorized
#print axioms LedgerContract.old_request
#print axioms LedgerContract.repeated_request
#print axioms LedgerContract.bounded_result
