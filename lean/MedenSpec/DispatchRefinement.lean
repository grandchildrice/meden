import MedenSpec.Dispatch
import MedenSpec.ByteVectors
import MedenSpec.ExportControlRefinement

/-! Refinement of the production borrowed review/dispatch operation and its
actual consuming/accessor path. Rust enforces ownership and shared borrowing;
Aeneas models their values. The HTTP executor itself is not extracted here. -/
namespace MedenSpec.DispatchRefinement
open Aeneas Aeneas.Std Result ByteVectors
open MedenImpl.export_control (State Outcome Origin ExportEvent FrozenRequest)
open ExportControlRefinement (phase origin event)
set_option maxHeartbeats 4000000

def request (r : FrozenRequest) : Dispatch.Request := ⟨r.target, bytes r.body.val⟩
def state (s : State) : Dispatch.State := ⟨phase s.phase, bytes s.session.val, request s.request⟩
def outcome (out : Outcome) : Dispatch.Outcome :=
  ⟨out.accepted, out.effect.map fun d => request d.request⟩

theorem new_refines (session body : alloc.vec.Vec U8) (target : String) :
    (do let out ← MedenImpl.export_control.State.new session target body; ok (state out)) =
      ok (Dispatch.initial (bytes session.val) target (bytes body.val)) := by
  simp [MedenImpl.export_control.State.new, state, request, Dispatch.initial, phase]

theorem target_refines (r : FrozenRequest) :
    MedenImpl.export_control.FrozenRequest.impl.target r = ok (request r).target := rfl

theorem body_refines (r : FrozenRequest) :
    (do let out ← MedenImpl.export_control.FrozenRequest.impl.body r; ok (bytes out.val)) =
      ok (request r).body := by
  simp [MedenImpl.export_control.FrozenRequest.impl.body, request, alloc.vec.Vec.deref]

/-- The exact values read by the runtime after consuming its dispatch token. -/
def observe (d : MedenImpl.export_control.Dispatch) : Result Dispatch.Request := do
  let r ← MedenImpl.export_control.Dispatch.into_request d
  let target ← MedenImpl.export_control.FrozenRequest.impl.target r
  let body ← MedenImpl.export_control.FrozenRequest.impl.body r
  ok ⟨target, bytes body.val⟩

theorem dispatch_refines (d : MedenImpl.export_control.Dispatch) :
    observe d = ok (request d.request) := by
  simp [observe, MedenImpl.export_control.Dispatch.into_request,
    MedenImpl.export_control.FrozenRequest.impl.target,
    MedenImpl.export_control.FrozenRequest.impl.body, request, alloc.vec.Vec.deref]

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
      hp, slice_eq, alloc.vec.Vec.deref,
      state, outcome, phase, origin, event, Dispatch.advance, Dispatch.authorized,
      ExportControl.step, bytes_eq]
  all_goals
    by_cases hs : s.session.val = session.val <;>
      by_cases ht : now.val < expires.val <;> simp_all
  all_goals split <;> simp_all

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
def runExtracted (s : State) : List Action → Result (State × List Dispatch.Request)
  | [] => ok (s, [])
  | a :: rest => do
    let out ← MedenImpl.export_control.State.advance s a.origin a.event a.now a.expires a.session
    let emitted ← match out.1.effect with
      | none => ok []
      | some d => do let r ← observe d; ok [r]
    let tail ← runExtracted out.2 rest
    ok (tail.1, emitted ++ tail.2)

private theorem run_success (s : State) (actions : List Action) :
    ∃ out, runExtracted s actions = ok out ∧
      (state out.1, out.2) = Dispatch.run (state s) (actions.map action) := by
  induction actions generalizing s with
  | nil => exact ⟨(s, []), rfl, rfl⟩
  | cons a rest ih =>
    obtain ⟨out, hs, ho⟩ := advance_success s a
    obtain ⟨tail, ht, hob⟩ := ih out.2
    refine ⟨(tail.1, (outcome out.1).effect.toList ++ tail.2), ?_, ?_⟩
    · simp [runExtracted, hs, ht, outcome]
      cases out.1.effect <;> simp [dispatch_refines]
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
