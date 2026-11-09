import MedenSpec.Conversation
import MedenSpec.CheckFailureRefinement

namespace MedenSpec.ToolOutcome

inductive Fault where
  | failed (message : String) | refused (message : String)
  | unavailable (message : String) | inconclusive (message : String) | cancelled
  deriving DecidableEq

inductive Host where
  | exited (word : BitVec 32) | timedOut | cancelled | outputLimit
  deriving DecidableEq

def report (response : Option (Except Fault α)) (diagnostic : String) : Except Fault α :=
  match response with
  | some (.ok value) => .ok value
  | some (.error (.failed message)) => .error (.failed message)
  | some (.error (.refused message)) => .error (.refused message)
  | some (.error (.unavailable message)) => .error (.unavailable message)
  | some (.error (.inconclusive _)) | some (.error .cancelled) | none => .error (.unavailable diagnostic)

def complete (host : Host) (truncated : Bool) (response : Option (Except Fault α))
    (diagnostic : String) : Except Fault α :=
  match host with
  | .cancelled => .error .cancelled
  | .timedOut | .outputLimit => .error (.inconclusive diagnostic)
  | .exited word =>
    if truncated then .error (.inconclusive diagnostic)
    else if word &&& 127#32 ≠ 0#32 then .error (.inconclusive diagnostic)
    else if word ≠ 0#32 then .error (.unavailable diagnostic)
    else report response diagnostic

def agent : Fault → Conversation.Check
  | .failed message => .tool message
  | .refused message => .refused message
  | .unavailable message | .inconclusive message => .stopped (.failed message)
  | .cancelled => .stopped .cancelled

def verification : Fault → CheckFailureRefinement.Failure
  | .failed _ => .unverified
  | .refused _ | .unavailable _ => .unavailable
  | .cancelled | .inconclusive _ => .inconclusive

theorem report_success_iff (response : Option (Except Fault α)) (diagnostic : String) (value : α) :
    report response diagnostic = .ok value ↔ response = some (.ok value) := by
  cases response with
  | none => simp [report]
  | some result => cases result with
    | ok result => simp [report]
    | error fault => cases fault <;> simp [report]

theorem complete_success_iff (host : Host) (truncated : Bool)
    (response : Option (Except Fault α)) (diagnostic : String) (value : α) :
    complete host truncated response diagnostic = .ok value ↔
      host = .exited 0#32 ∧ truncated = false ∧ response = some (.ok value) := by
  cases host <;> cases truncated <;> simp [complete]
  all_goals split_ifs <;> simp_all [report_success_iff]
  all_goals intro zero; subst_vars; contradiction

theorem cancellation_wins (truncated : Bool) (response : Option (Except Fault α)) (diagnostic : String) :
    complete .cancelled truncated response diagnostic = .error .cancelled := rfl

theorem worker_stop_refused (diagnostic message : String) :
    report (some (.error .cancelled : Except Fault α)) diagnostic = .error (.unavailable diagnostic) ∧
    report (some (.error (.inconclusive message) : Except Fault α)) diagnostic = .error (.unavailable diagnostic) := by
  exact ⟨rfl, rfl⟩

theorem repair_iff (fault : Fault) :
    verification fault = .unverified ↔ ∃ message, fault = .failed message := by
  cases fault <;> simp [verification]

end MedenSpec.ToolOutcome
