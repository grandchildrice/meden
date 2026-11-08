import MedenSpec.ConversationObservations
import MedenSpec.ConversationSafety

namespace MedenSpec.ConversationTrace
open Aeneas Aeneas.Std Result MedenConversationImpl ConversationRefinement
open flow

structure Input where
  value : FlowInput
  cancelled : Bool
  stop : Option conversation.Status

def input (i : Input) : Conversation.Observation :=
  ⟨ConversationRefinement.input i.value, i.cancelled, i.stop.map status⟩

def runExtracted (s : Flow) : List Input → Result (Flow × Nat)
  | [] => ok (s, 0)
  | i :: rest => do
    let (effect, next, _) ← Flow.advance s i.value i.cancelled i.stop
    let final ← runExtracted next rest
    ok (final.1, Conversation.models (ConversationRefinement.effect effect) + final.2)

private theorem call_success (s : Flow) (i : Input) :
    ∃ out, Flow.advance s i.value i.cancelled i.stop = ok out ∧
      (effect out.1, state out.2.1) = Conversation.step Usize.max (state s) (input i) := by
  have hm := advance_refines s i.value i.cancelled i.stop
  cases hc : Flow.advance s i.value i.cancelled i.stop with
  | ret out =>
    rcases out with ⟨e, next, after⟩
    exact ⟨(e, next, after), rfl, by simpa [hc, input, Conversation.step] using hm⟩
  | vis effect continuation => simp [hc] at hm
  | div => simp [hc] at hm

theorem trace_refines (s : Flow) (inputs : List Input) :
    (do let out ← runExtracted s inputs; ok (state out.1, out.2)) =
      ok (Conversation.run Usize.max (state s) (inputs.map input)) := by
  induction inputs generalizing s with
  | nil => simp [runExtracted, Conversation.run]
  | cons i rest ih =>
    obtain ⟨⟨e, next, after⟩, hc, hm⟩ := call_success s i
    have he := congrArg Prod.fst hm
    have hs := congrArg Prod.snd hm
    dsimp only at he hs
    simp only [runExtracted, hc, bind_ok]
    have hr := ih next
    cases hn : runExtracted next rest with
    | ret final =>
      simp only [hn, bind_ok, Result.ok.injEq] at hr
      simp [hn, Conversation.run, ← hr, ← hs, ← he]
    | vis effect continuation => simp [hn] at hr
    | div => simp [hn] at hr

theorem extracted_request_budget (initial final : Flow) (groups : Usize) (count : Nat)
    (inputs : List Input) (hi : Flow.begin groups = ok initial)
    (hr : runExtracted initial inputs = ok (final, count)) : count ≤ groups.val + 1 := by
  have hn := begin_refines groups
  simp only [hi, bind_ok, Result.ok.injEq] at hn
  have ht := trace_refines initial inputs
  simp only [hr, bind_ok, Result.ok.injEq] at ht
  have hb := Conversation.trace_budget Usize.max (state initial) (inputs.map input)
  rw [← ht, hn] at hb
  simp only [Conversation.initial] at hb
  omega

theorem extracted_halted (s : Flow) (inputs : List Input) (hs : s.phase = .Halted) :
    runExtracted s inputs = ok (s, 0) := by
  induction inputs with
  | nil => rfl
  | cons i rest ih => simp [runExtracted, Flow.advance, hs, ih, effect, Conversation.models]

theorem extracted_stop_blocks (s next : Flow) (i : Input) (e : FlowEffect) (after : FlowInput)
    (reason : conversation.Status) (hs : s.stopped = some reason)
    (hr : Flow.advance s i.value i.cancelled i.stop = ok (e, next, after)) :
    e ≠ .Model ∧ e ≠ .Tool true := by
  obtain ⟨out, hc, hm⟩ := call_success s i
  have ho : out = (e, next, after) := by simpa [hr] using hc.symm
  subst out
  have hb := Conversation.blocked_has_no_effects Usize.max (state s) (input i)
    (by simp [Conversation.blocked, state, hs])
  simp only [← hm] at hb
  cases e <;> simp_all [effect]

theorem extracted_stop_persists (s next : Flow) (i : Input) (e : FlowEffect)
    (after : FlowInput) (reason : conversation.Status) (hs : s.stopped = some reason)
    (hr : Flow.advance s i.value i.cancelled i.stop = ok (e, next, after)) :
    next.stopped = some reason := by
  obtain ⟨out, hc, hm⟩ := call_success s i
  have ho : out = (e, next, after) := by simpa [hr] using hc.symm
  subst out
  have hp := Conversation.stop_persists Usize.max (state s) (input i) (status reason)
    (by simp [state, hs])
  rw [← hm] at hp
  change next.stopped.map status = some (status reason) at hp
  cases hn : next.stopped with
  | none => simp [hn] at hp
  | some value =>
    cases value <;> cases reason <;> simp_all [status]

private theorem input_bounded (i : Input) :
    Conversation.BoundedInput Usize.max (input i).input := by
  rcases i with ⟨value, cancelled, stop⟩
  cases value <;> simp [input, ConversationRefinement.input, Conversation.BoundedInput, reply]

theorem extracted_wellFormed (s : Flow) (inputs : List Input)
    (hs : Conversation.WellFormed Usize.max (state s)) :
    ∃ final count, runExtracted s inputs = ok (final, count) ∧
      Conversation.WellFormed Usize.max (state final) := by
  induction inputs generalizing s with
  | nil => exact ⟨s, 0, rfl, hs⟩
  | cons i rest ih =>
    obtain ⟨⟨e, next, after⟩, hc, hm⟩ := call_success s i
    have hw := Conversation.step_wellFormed Usize.max (state s) (input i) hs (input_bounded i)
    rw [← hm] at hw
    obtain ⟨final, count, hr, hf⟩ := ih next hw
    exact ⟨final, Conversation.models (effect e) + count, by simp [runExtracted, hc, hr], hf⟩

theorem initialized_wellFormed (s : Flow) (groups : Usize) (inputs : List Input)
    (hi : Flow.begin groups = ok s) :
    ∃ final count, runExtracted s inputs = ok (final, count) ∧
      Conversation.WellFormed Usize.max (state final) := by
  have hb := begin_refines groups
  simp only [hi, bind_ok, Result.ok.injEq] at hb
  apply extracted_wellFormed s inputs
  rw [hb]
  exact Conversation.initial_wellFormed _ _

theorem extracted_record_pairing (s next : Flow) (i : Input) (after : FlowInput)
    (hs : Conversation.WellFormed Usize.max (state s))
    (hr : Flow.advance s i.value i.cancelled i.stop = ok (.Record, next, after)) :
    (do let selected ← Flow.pending_call next; ok (selected.map call)) =
      ok ((state s).reply.calls[(state s).next]?) ∧
    (do let value ← Flow.impl.result next; ok (value.map check)) = ok (state next).result ∧
    (state next).next < (state next).reply.calls.length ∧ (state next).result.isSome = true := by
  obtain ⟨out, hc, hm⟩ := call_success s i
  have ho : out = (.Record, next, after) := by simpa [hr] using hc.symm
  subst out
  have he : (Conversation.step Usize.max (state s) (input i)).1 = .record := by
    rw [← hm]
    rfl
  have hid := Conversation.record_identity Usize.max (state s) (input i) he
  have hw := Conversation.step_wellFormed Usize.max (state s) (input i) hs (input_bounded i)
  simp only [← hm] at hid hw
  obtain ⟨hreply, hnext, hphase⟩ := hid
  have hbound : (state next).next < (state next).reply.calls.length ∧
      (state next).result.isSome = true := by
    simpa [Conversation.WellFormed, hphase] using hw.2.2
  refine ⟨?_, ?_, hbound⟩
  · simpa [Conversation.pending, hphase, hreply, hnext] using
      ConversationObservations.pending_refines next
  · simpa [Conversation.result, hphase] using ConversationObservations.result_refines next

end MedenSpec.ConversationTrace
