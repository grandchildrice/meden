import MedenSpec.Dispatch
import MedenSpec.TransferRefinement
import MedenSpec.ExportControlRefinement

/-! Refinement of the production combined review/reservation operation. The
returned request is the executor's input; its decoding and I/O are not proved
by this module. -/
namespace MedenSpec.DispatchRefinement
open Aeneas Aeneas.Std Result ByteVectors
open MedenImpl.export_control (State Outcome Origin ExportEvent)
open ExportControlRefinement (phase origin event)
open TransferRefinement (request permit)
set_option maxHeartbeats 4000000

def state (s : State) : Dispatch.State := ⟨phase s.phase, request s.request⟩
def outcome (out : Outcome) : Dispatch.Outcome := ⟨out.accepted, out.effect.map request⟩

private theorem request_clone (r : MedenImpl.transfer.Request) :
    MedenImpl.transfer.Request.Insts.CoreCloneClone.clone r = ok r := by
  simp [MedenImpl.transfer.Request.Insts.CoreCloneClone.clone, vec_clone]

theorem new_refines (r : MedenImpl.transfer.Request) :
    (do let out ← MedenImpl.export_control.State.new r; ok (state out)) =
      ok (Dispatch.initial (request r)) := by
  simp [MedenImpl.export_control.State.new, state, Dispatch.initial, phase]

theorem request_refines (s : State) :
    (do let out ← MedenImpl.export_control.State.impl.request s; ok (request out)) =
      ok (state s).request := by
  simp [MedenImpl.export_control.State.impl.request, state]

theorem phase_refines (s : State) :
    (do let out ← MedenImpl.export_control.State.impl.phase s; ok (phase out)) =
      ok (state s).phase := by
  simp [MedenImpl.export_control.State.impl.phase, state]

theorem advance_refines (s : State) (o : Origin) (e : ExportEvent)
    (now expires : U64) (session : Slice U8) :
    (do let out ← MedenImpl.export_control.State.advance s o e now expires session
        ok (outcome out.1, state out.2)) =
      ok (Dispatch.advance (state s) (origin o) (event e) now.val expires.val (bytes session.val)) := by
  cases hp : s.phase <;> cases o <;> cases e <;>
    simp [MedenImpl.export_control.State.advance, MedenImpl.export_control.step,
      hp, request_clone, MedenImpl.transfer.reserve, TransferRefinement.allowed_refines,
      state, outcome, phase, origin, event, Dispatch.advance, Dispatch.authorized,
      ExportControl.step, Transfer.allowed, permit]
  all_goals
    by_cases h : (request s.request).session = bytes session.val ∧ now.val < expires.val <;> simp [h]

structure Action where
  origin : Origin
  event : ExportEvent
  now : U64
  expires : U64
  session : Slice U8

def action (a : Action) : Dispatch.Action :=
  ⟨origin a.origin, event a.event, a.now.val, a.expires.val, bytes a.session.val⟩

private theorem advance_success (s : State) (a : Action) :
    ∃ out, MedenImpl.export_control.State.advance s a.origin a.event a.now a.expires a.session = ok out ∧
      (outcome out.1, state out.2) =
        Dispatch.advance (state s) (origin a.origin) (event a.event) a.now.val a.expires.val (bytes a.session.val) := by
  have h := advance_refines s a.origin a.event a.now a.expires a.session
  cases hr : MedenImpl.export_control.State.advance s a.origin a.event a.now a.expires a.session with
  | ret out => exact ⟨out, by simp_all, by simpa [hr] using h⟩
  | vis effect continuation => simp [hr] at h
  | div => simp [hr] at h

/-- Composition of actual extracted transitions. This is not an extraction of
the UI, mutex or network executor. Its log observes complete emitted requests. -/
def runExtracted (s : State) : List Action → Result (State × List Transfer.Request)
  | [] => ok (s, [])
  | a :: rest => do
    let out ← MedenImpl.export_control.State.advance s a.origin a.event a.now a.expires a.session
    let tail ← runExtracted out.2 rest
    ok (tail.1, (outcome out.1).effect.toList ++ tail.2)

private theorem run_success (s : State) (actions : List Action) :
    ∃ out, runExtracted s actions = ok out ∧
      (state out.1, out.2) = Dispatch.run (state s) (actions.map action) := by
  induction actions generalizing s with
  | nil => exact ⟨(s, []), rfl, rfl⟩
  | cons a rest ih =>
    obtain ⟨out, hs, ho⟩ := advance_success s a
    obtain ⟨tail, ht, hob⟩ := ih out.2
    refine ⟨(tail.1, (outcome out.1).effect.toList ++ tail.2), ?_, ?_⟩
    · simp [runExtracted, hs, ht]
    · simp only [Dispatch.run, List.map_cons, action]
      rw [← ho]
      rw [← hob]

theorem trace_refines (s : State) (actions : List Action) :
    (do let out ← runExtracted s actions; ok (state out.1, out.2)) =
      ok (Dispatch.run (state s) (actions.map action)) := by
  obtain ⟨out, hr, ho⟩ := run_success s actions
  simp [hr, ho]

/-- Every finite extracted execution emits at most one request, always the
initial frozen request. In particular later review, transport events or model
approvals cannot authorize a different payload or another attempt. -/
theorem extracted_trace_safe (s : State) (actions : List Action) :
    ∃ out, runExtracted s actions = ok out ∧ out.2.length ≤ 1 ∧
      (state out.1).request = (state s).request ∧
      ∀ r ∈ out.2, r = (state s).request := by
  obtain ⟨out, hr, ho⟩ := run_success s actions
  have hc := Dispatch.at_most_one_effect (state s) (actions.map action)
  have hf := Dispatch.trace_frozen (state s) (actions.map action)
  rw [← ho] at hc hf
  exact ⟨out, hr, hc, hf⟩

end MedenSpec.DispatchRefinement
