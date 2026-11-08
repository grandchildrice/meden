import MedenSpec.Generated.Repair

/-! Classification of actual worker termination values. The pinned Linux/Unix
wait-word layout has seven low status bits; normal completion has them clear.
OS attribution, I/O adapters and diagnostic rendering remain separate obligations. -/
namespace MedenSpec.CheckFailureRefinement
open Aeneas Aeneas.Std Result
open MedenRepairImpl

inductive Failure where
  | unverified | unavailable | inconclusive
  deriving DecidableEq

def mayRepair : Failure → Bool
  | .unverified => true | .unavailable | .inconclusive => false

def stops : Option Failure → Bool
  | none => false | some f => !mayRepair f

open verification.failure

def failure : FailureKind → Failure
  | .Unverified => .unverified | .Unavailable => .unavailable | .Inconclusive => .inconclusive

theorem repairable_refines (f : FailureKind) :
    FailureKind.repairable f = ok (mayRepair (failure f)) := by
  cases f <;> simp [FailureKind.repairable,
    FailureKind.Insts.CoreCmpPartialEqFailureKind.eq, FailureKind.read_discriminant,
    mayRepair, failure]

theorem verification_stops_refines (f : Option FailureKind) :
    verification.failure.verification_stops f = ok (stops (f.map failure)) := by
  cases f with
  | none => rfl
  | some f => cases f <;> simp [verification.failure.verification_stops,
      repairable_refines, stops, mayRepair, failure]

theorem operational_stops :
    verification.failure.verification_stops (some .Unavailable) = ok true ∧
    verification.failure.verification_stops (some .Inconclusive) = ok true := by
  simp [verification_stops_refines, stops, mayRepair, failure]

/-- Automatic source repair requires complete output and normal completion.
The caller has already established that the operation did not pass. -/
def repairable (t : process.Termination) (truncated : Bool) : Bool :=
  !truncated && match t with
    | .Exited raw => raw.bv &&& 127#32 == 0#32
    | .TimedOut | .Cancelled | .OutputLimit => false

private theorem mask_eq (raw : I32) :
    (raw &&& 127#i32 = 0#i32) ↔ (raw.bv &&& 127#32 = 0#32) := by
  change ((⟨raw.bv &&& 127#32⟩ : I32) = (⟨0#32⟩ : I32)) ↔ _
  simp only [IScalar.mk.injEq]

theorem failed_worker_refines (t : process.Termination) (truncated : Bool) :
    verification.failure.failed_worker t truncated =
      ok (if repairable t truncated then .Unverified else .Inconclusive) := by
  cases t <;> cases truncated <;>
    simp [verification.failure.failed_worker, repairable,
      std.process.ExitStatus.Insts.StdOsUnixProcessExitStatusExt.into_raw,
      Aeneas.Std.lift, Aeneas.Std.bind_ok, mask_eq]
  all_goals split_ifs <;> rfl

theorem truncated_is_inconclusive (t : process.Termination) :
    verification.failure.failed_worker t true = ok .Inconclusive := by
  rfl

end MedenSpec.CheckFailureRefinement
