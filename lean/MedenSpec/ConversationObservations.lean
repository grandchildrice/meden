import MedenSpec.ConversationRefinement
import MedenSpec.RepairRefinement

namespace MedenSpec.ConversationObservations
open Aeneas Aeneas.Std Result MedenConversationImpl
open flow ConversationRefinement

theorem pending_refines (s : Flow) :
    (do let out ← Flow.pending_call s; ok (out.map call)) =
      ok (Conversation.pending (state s)) := by
  cases hp : s.phase <;>
    simp [Flow.pending_call, hp, Conversation.pending, state, phase, reply,
      core.slice.Slice.get, alloc.vec.Vec.deref, List.getElem?_map]

theorem reply_refines (s : Flow) :
    (do let out ← Flow.impl.reply s; ok (reply out)) = ok (state s).reply := by
  simp [Flow.impl.reply, state]

theorem result_refines (s : Flow) :
    (do let out ← Flow.impl.result s; ok (out.map check)) =
      ok (Conversation.result (state s)) := by
  cases hp : s.phase <;> cases hr : s.result <;>
    simp [Flow.impl.result, hp, hr, Conversation.result, state, phase]

theorem outcome_refines (s : Flow) :
    (do let out ← Flow.into_outcome s; ok (outcome out)) =
      ok (Conversation.outcome (state s)) := by
  cases hs : s.stopped <;> cases hf : s.faulted <;>
    simp [Flow.into_outcome, hs, hf, Conversation.outcome, state, status, outcome, reply]

theorem inspection_bridge (t : MedenRepairImpl.verification.turn.Turn)
    (s : conversation.Status) (accepted changed cancelled review : Bool)
    (failure : Option MedenRepairImpl.verification.failure.FailureKind) :
    (do let inspection ← conversation.Status.inspection s
        let out ← MedenRepairImpl.verification.turn.Turn.advance t
          (.Conversation inspection accepted changed) cancelled failure review
        ok (RepairRefinement.effect out.1, RepairRefinement.state out.2)) =
      ok (Repair.advance (RepairRefinement.state t)
        (.conversation (Conversation.inspection (status s)) accepted changed)
        cancelled (failure.map RepairRefinement.failure) review) := by
  rw [inspection_refines]
  simpa [RepairRefinement.signal] using
    RepairRefinement.advance_refines t
      (.Conversation (Conversation.inspection (status s)) accepted changed) cancelled failure review

end MedenSpec.ConversationObservations
