import MedenSpec.Conversation
import MedenSpec.Generated.Conversation

namespace MedenSpec.ConversationRefinement
set_option maxHeartbeats 4000000
open Aeneas Aeneas.Std Result MedenConversationImpl
open flow

def call (c : conversation.ToolCall) : Conversation.Call :=
  ⟨c.id, c.function.name, c.function.arguments⟩
def reply (r : conversation.Reply) : Conversation.Reply :=
  ⟨r.content, r.tool_calls.val.map call⟩
def status : conversation.Status → Conversation.Status
  | .Done text => .done text | .Failed text => .failed text
  | .Incomplete => .incomplete | .Cancelled => .cancelled | .NeedsReview => .review
def check : core.result.Result String conversation.ExecutionError → Conversation.Check
  | .Ok text => .ok text
  | .Err (.Refused text) => .refused text
  | .Err (.Tool text) => .tool text
  | .Err (.Stopped value) => .stopped (status value)
def phase : FlowPhase → Conversation.Phase
  | .Ready => .ready | .Announced => .announced | .Waiting => .waiting
  | .PublishingAssistant => .publishingAssistant | .Executing => .executing
  | .PublishingResult => .publishingResult | .Halted => .halted
def state (s : Flow) : Conversation.State :=
  ⟨s.remaining.val, s.attempt.val, phase s.phase, reply s.reply, s.next.val,
    s.result.map check, s.stopped.map status, s.faulted⟩
def input : FlowInput → Conversation.Input
  | .Start => .start | .Observed => .observed | .Reply value => .reply (reply value)
  | .ModelFailed value => .modelFailed (status value)
  | .ToolResult value => .toolResult (check value)
def effect : FlowEffect → Conversation.Effect
  | .Asking attempt => .asking attempt.val | .Model => .model | .Assistant => .assistant
  | .Tool execute => .tool execute | .Record => .record | .Stop => .stop
def outcome : FlowOutcome → Conversation.Outcome
  | .Finish content => .finish content | .Stopped value => .stopped (status value) | .Fault => .fault

private theorem sat_add_val (x y : Usize) :
    (core.num.Usize.saturating_add x y).val = min (x.val + y.val) Usize.max := by
  change (BitVec.ofNat _ (min (UScalar.max .Usize) (x.val + y.val))).toNat = _
  rw [BitVec.toNat_ofNat, Nat.mod_eq_of_lt]
  · simp only [UScalar.max_USize_eq, Nat.min_comm]
  · apply Nat.lt_of_le_of_lt (Nat.min_le_left _ _)
    rw [UScalar.max]
    have hp : 0 < 2 ^ UScalarTy.Usize.numBits := by positivity
    omega

private theorem sat_sub_val (x y : Usize) :
    (core.num.Usize.saturating_sub x y).val = x.val - y.val := by
  change (BitVec.ofNat _ (max 0 (x.val - y.val))).toNat = _
  rw [BitVec.toNat_ofNat]
  simp only [Nat.zero_le, max_eq_right]
  exact Nat.mod_eq_of_lt (Nat.lt_of_le_of_lt (Nat.sub_le _ _) x.bv.isLt)

private theorem zero_eq (x : Usize) : x = 0#usize ↔ x.val = 0 := by
  simpa using UScalar.eq_equiv x 0#usize

attribute [local simp] state phase reply call status check input effect outcome
  zero_eq sat_add_val sat_sub_val Aeneas.Std.lift

theorem inspection_refines (s : conversation.Status) :
    conversation.Status.inspection s = ok (Conversation.inspection (status s)) := by
  cases s <;> rfl

theorem begin_refines (groups : Usize) :
    (do let s ← Flow.begin groups; ok (state s)) =
      ok (Conversation.initial Usize.max groups.val) := by
  simp [Flow.begin, empty_reply, Conversation.initial]

private theorem latch_value (s : Flow) (host : Option conversation.Status) :
    Flow.latch s host = ok (if s.faulted then s else
      if s.stopped.isNone then {s with stopped := host} else s) := by
  cases hf : s.faulted <;> cases hs : s.stopped <;> simp [Flow.latch, hf, hs]

attribute [local simp] latch_value Flow.halt Conversation.halt Conversation.blocked Conversation.latch

private theorem dispatch_refines (s : Flow) (i : FlowInput) :
    (do let (e, out, _) ← Flow.dispatch s i; ok (effect e, state out)) =
      ok (Conversation.dispatch Usize.max (state s) (input i)) := by
  cases hp : s.phase <;> cases i <;> cases hf : s.faulted <;> cases hs : s.stopped <;>
    simp [Flow.dispatch, hp, hf, hs, Flow.reject, Flow.announce, Flow.next_call,
      Flow.recorded, Flow.record, empty_reply, Conversation.dispatch, Conversation.reject,
      Conversation.announce, Conversation.nextCall, Conversation.recorded, Conversation.record,
      alloc.vec.Vec.is_empty]
  all_goals repeat' (first | simp_all | split)

theorem interrupt_refines (cancelled : Bool) (host : Option conversation.Status) :
    (do let out ← flow.interrupt cancelled host; ok (out.map status)) =
      ok (if cancelled then some .cancelled else host.map status) := by
  cases cancelled <;> simp [flow.interrupt]

theorem advance_refines (s : Flow) (i : FlowInput) (cancelled : Bool)
    (host : Option conversation.Status) :
    (do let (e, out, _) ← Flow.advance s i cancelled host; ok (effect e, state out)) =
      ok (Conversation.advance Usize.max (state s) (input i) cancelled (host.map status)) := by
  cases hp : s.phase <;> cases cancelled <;> cases hf : s.faulted <;> cases hs : s.stopped <;>
    simp [Flow.advance, flow.interrupt, hp, hf, hs, Conversation.advance]
  all_goals first
    | simpa [Conversation.dispatch, Conversation.reject, Conversation.announce,
        Conversation.nextCall, Conversation.recorded, Conversation.record]
        using dispatch_refines s i
    | repeat' (first | simp_all [Flow.dispatch, Flow.reject, Flow.announce, Flow.next_call,
        Flow.recorded, Flow.record, empty_reply, Conversation.dispatch, Conversation.reject,
        Conversation.announce, Conversation.nextCall, Conversation.recorded, Conversation.record,
        alloc.vec.Vec.is_empty] | split)

end MedenSpec.ConversationRefinement
