import MedenSpec.ToolOutcome
import MedenSpec.ConversationRefinement

namespace MedenSpec.ToolOutcomeRefinement
open Aeneas Aeneas.Std Result

/-- Both extracted copies erase the private newtype to the same standard sum. -/
def fault : MedenConversationImpl.tool.ToolError → ToolOutcome.Fault
  | .Ok (.Ok message) => .failed message | .Ok (.Err message) => .refused message
  | .Err (some (.Ok message)) => .unavailable message
  | .Err (some (.Err message)) => .inconclusive message
  | .Err none => .cancelled

def result : core.result.Result α MedenRepairImpl.meden_agent.tool.ToolError → Except ToolOutcome.Fault α
  | .Ok value => .ok value | .Err error => .error (fault error)

def host : MedenRepairImpl.process.Termination → ToolOutcome.Host
  | .Exited word => .exited word.bv | .TimedOut => .timedOut
  | .Cancelled => .cancelled | .OutputLimit => .outputLimit

def agentBuild : ToolOutcome.Fault → Result MedenConversationImpl.tool.ToolError
  | .failed message => MedenConversationImpl.tool.ToolError.failed message
  | .refused message => MedenConversationImpl.tool.ToolError.refused message
  | .unavailable message => MedenConversationImpl.tool.ToolError.unavailable message
  | .inconclusive message => MedenConversationImpl.tool.ToolError.inconclusive message
  | .cancelled => MedenConversationImpl.tool.ToolError.cancelled

def workerBuild : ToolOutcome.Fault → Result MedenRepairImpl.meden_agent.tool.ToolError
  | .failed message => MedenRepairImpl.meden_agent.tool.ToolError.failed message
  | .refused message => MedenRepairImpl.meden_agent.tool.ToolError.refused message
  | .unavailable message => MedenRepairImpl.meden_agent.tool.ToolError.unavailable message
  | .inconclusive message => MedenRepairImpl.meden_agent.tool.ToolError.inconclusive message
  | .cancelled => MedenRepairImpl.meden_agent.tool.ToolError.cancelled

attribute [local simp]
  MedenConversationImpl.tool.ToolError.failed MedenConversationImpl.tool.ToolError.refused
  MedenConversationImpl.tool.ToolError.unavailable MedenConversationImpl.tool.ToolError.inconclusive
  MedenConversationImpl.tool.ToolError.cancelled MedenConversationImpl.tool.ToolError.parts
  MedenConversationImpl.tool.ToolError.into_parts
  MedenRepairImpl.meden_agent.tool.ToolError.failed MedenRepairImpl.meden_agent.tool.ToolError.refused
  MedenRepairImpl.meden_agent.tool.ToolError.unavailable MedenRepairImpl.meden_agent.tool.ToolError.inconclusive
  MedenRepairImpl.meden_agent.tool.ToolError.cancelled MedenRepairImpl.meden_agent.tool.ToolError.parts
  MedenRepairImpl.meden_agent.tool.ToolError.into_parts

theorem agent_build_refines (f : ToolOutcome.Fault) :
    (do let native ← agentBuild f; ok (fault native)) = ok f := by
  cases f <;> simp [agentBuild, fault]

theorem worker_build_refines (f : ToolOutcome.Fault) :
    (do let native ← workerBuild f; ok (fault native)) = ok f := by
  cases f <;> simp [workerBuild, fault]

theorem accessors_refine (f : MedenConversationImpl.tool.ToolError) :
    MedenConversationImpl.tool.ToolError.parts f = ok f ∧
    MedenConversationImpl.tool.ToolError.into_parts f = ok f ∧
    MedenRepairImpl.meden_agent.tool.ToolError.parts f = ok f ∧
    MedenRepairImpl.meden_agent.tool.ToolError.into_parts f = ok f := by
  simp

theorem agent_refines (f : MedenConversationImpl.tool.ToolError) :
    (do let actual ← MedenConversationImpl.conversation.tool_error f;
        ok (ConversationRefinement.check (.Err actual))) = ok (ToolOutcome.agent (fault f)) := by
  rcases f with (message | message) | (_ | (message | message)) <;>
    simp [MedenConversationImpl.conversation.tool_error, ConversationRefinement.check,
      ConversationRefinement.status, ToolOutcome.agent, fault]

private theorem zero_eq (word : I32) : (word = 0#i32) ↔ word.bv = 0#32 := by
  change ((⟨word.bv⟩ : I32) = (⟨0#32⟩ : I32)) ↔ _
  simp only [IScalar.mk.injEq]

theorem complete_refines (termination : MedenRepairImpl.process.Termination) (truncated : Bool)
    (response : Option (core.result.Result α MedenRepairImpl.meden_agent.tool.ToolError))
    (diagnostic : String) :
    (do let actual ← MedenRepairImpl.file.complete termination truncated response diagnostic;
        ok (result actual)) =
      ok (ToolOutcome.complete (host termination) truncated (response.map result) diagnostic) := by
  cases termination <;> cases truncated <;>
    simp [MedenRepairImpl.file.complete, ToolOutcome.complete, host,
      MedenRepairImpl.std.process.ExitStatus.Insts.StdOsUnixProcessExitStatusExt.into_raw,
      Aeneas.Std.lift, Aeneas.Std.bind_ok, zero_eq, result, fault]
  all_goals split_ifs
  all_goals cases response with
    | none => simp [ToolOutcome.report]
    | some response => cases response with
      | Ok value => simp [result, ToolOutcome.report]
      | Err f => rcases f with (message | message) | (_ | (message | message)) <;>
          simp [result, ToolOutcome.report, fault]

theorem verification_refines (f : MedenRepairImpl.meden_agent.tool.ToolError) :
    (do let actual ← MedenRepairImpl.file.verification_kind f;
        ok (CheckFailureRefinement.failure actual)) = ok (ToolOutcome.verification (fault f)) := by
  rcases f with (message | message) | (_ | (message | message)) <;>
    simp [MedenRepairImpl.file.verification_kind, CheckFailureRefinement.failure,
      ToolOutcome.verification, fault]

private theorem response_ok (response : Option (core.result.Result α MedenRepairImpl.meden_agent.tool.ToolError))
    (value : α) : response.map result = some (.ok value) ↔ response = some (.Ok value) := by
  cases response with
  | none => simp
  | some response => cases response <;> simp [result]

/-- The original success payload is admitted only with a complete zero-exit response. -/
theorem success_iff (termination : MedenRepairImpl.process.Termination) (truncated : Bool)
    (response : Option (core.result.Result α MedenRepairImpl.meden_agent.tool.ToolError))
    (diagnostic : String) (value : α) :
    (do let actual ← MedenRepairImpl.file.complete termination truncated response diagnostic;
        ok (result actual)) = ok (.ok value) ↔
      host termination = .exited 0#32 ∧ truncated = false ∧ response = some (.Ok value) := by
  rw [complete_refines]
  simp only [Result.ok.injEq, ToolOutcome.complete_success_iff, response_ok]

theorem repair_iff (f : MedenRepairImpl.meden_agent.tool.ToolError) :
    (do let actual ← MedenRepairImpl.file.verification_kind f;
        ok (CheckFailureRefinement.failure actual)) = ok .unverified ↔
      ∃ message, fault f = .failed message := by
  rw [verification_refines]
  simp only [Result.ok.injEq, ToolOutcome.repair_iff]

end MedenSpec.ToolOutcomeRefinement
