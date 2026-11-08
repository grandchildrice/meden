import Candidate
import Spec

open Aeneas Aeneas.Std Result
namespace LedgerProof
set_option maxHeartbeats 2000000

def abstractState (s : LedgerImpl.ledger.LedgerState) : LedgerContract.State :=
  ⟨s.debit.val, s.credit.val, s.sequence.val⟩

theorem actual_spec (s : LedgerImpl.ledger.LedgerState) (actor id amount : U64) :
    LedgerImpl.ledger.transfer s actor id amount ⦃ out =>
      abstractState out = LedgerContract.transfer (abstractState s) actor.val id.val amount.val ⦄ := by
  unfold LedgerImpl.ledger.transfer
  step*
  all_goals simp_all [abstractState, LedgerContract.transfer, LedgerContract.permitted,
    LedgerContract.maximum, U64.rMax, UScalar.eq_equiv]
  all_goals scalar_tac

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
