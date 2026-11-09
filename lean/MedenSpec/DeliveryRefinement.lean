import MedenSpec.Delivery
import MedenSpec.ContractsRefinement
import MedenSpec.DispatchRefinement

/-! The actual Rust evidence/export composition. Source freshness, checked-file
packaging, host origin, clocks, locking and HTTP remain explicit adapters. -/
namespace MedenSpec.DeliveryRefinement
open Aeneas Aeneas.Std Result ByteVectors
open MedenEvidenceImpl
set_option maxHeartbeats 8000000

open ExportControlRefinement (phase origin event)
open DispatchRefinement (request outcome)
abbrev channel := DispatchRefinement.state
def state (s : evidence.Delivery) : Delivery.State :=
  ⟨s.contract.map ContractsRefinement.view, channel s.«export»⟩

private theorem accepted_success (s : evidence.Contracts) :
    ∃ out, evidence.Contracts.impl.accepted s = ok out ∧
      out.map ContractsRefinement.view = Contracts.accepted (ContractsRefinement.state s) := by
  have h := ContractsRefinement.accepted_refines s
  cases hc : evidence.Contracts.impl.accepted s with
  | ret out => exact ⟨out, by simp_all, by simpa [hc] using h⟩
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

private theorem displayed_success (s : evidence.Contracts) :
    ∃ out, evidence.Contracts.displayed s = ok out ∧
      out.map ContractsRefinement.view = Contracts.displayed (ContractsRefinement.state s) := by
  have h := ContractsRefinement.displayed_refines s
  cases hc : evidence.Contracts.displayed s with
  | ret out => exact ⟨out, by simp_all, by simpa [hc] using h⟩
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

theorem permitted_refines (s : evidence.Delivery) (current : evidence.Contracts) :
    evidence.Delivery.permitted s current =
      ok (Delivery.permitted (ContractsRefinement.state current) (state s).contract) := by
  cases hf : s.contract with
  | none =>
    obtain ⟨out, hc, hm⟩ := displayed_success current
    cases out <;> simp [evidence.Delivery.permitted, hf, hc, Delivery.permitted, state, ← hm]
  | some frozen =>
    obtain ⟨out, hc, hm⟩ := accepted_success current
    cases out <;>
      simp [evidence.Delivery.permitted, hf, hc, ContractsRefinement.view_eq,
        vec_clone, ContractsRefinement.eligible_refines, Delivery.permitted, state, ← hm]
    split <;> simp_all [ContractsRefinement.view, EvidenceRefinement.subject]

theorem prepare_refines (current : evidence.Contracts) (target : String) (body : alloc.vec.Vec U8) :
    (do let out ← evidence.Contracts.prepare_delivery current target body; ok (out.map state)) =
      ok (Delivery.prepare (ContractsRefinement.state current) target (bytes body.val)) := by
  obtain ⟨frozen, hc, hm⟩ := accepted_success current
  simp only [evidence.Contracts.prepare_delivery, hc, bind_ok, vec_clone,
    export_control.State.new]
  rw [permitted_refines]
  simp only [Delivery.prepare, ← hm, state, channel, request, Dispatch.initial, phase]
  split <;> simp_all [state, channel, DispatchRefinement.state, request, phase, ContractsRefinement.state]

private theorem underlying_success (s : export_control.State) (o : export_control.Origin)
    (e : export_control.ExportEvent) (now expires : U64) (session : Slice U8) :
    ∃ out, export_control.State.advance s o e now expires session = ok out ∧
      (outcome out.1, channel out.2) =
        Dispatch.advance (channel s) (origin o) (event e) now.val expires.val (bytes session.val) := by
  have h := DispatchRefinement.advance_refines s o e now expires session
  cases hc : export_control.State.advance s o e now expires session with
  | ret out => exact ⟨out, by simp_all, by simpa [hc] using h⟩
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

private theorem approve_eq (e : export_control.ExportEvent) :
    export_control.ExportEvent.Insts.CoreCmpPartialEqExportEvent.eq e .Approve =
      ok (decide (event e = .approve)) := by
  cases e <;> simp [export_control.ExportEvent.Insts.CoreCmpPartialEqExportEvent.eq,
    export_control.ExportEvent.read_discriminant, event]

theorem advance_refines (s : evidence.Delivery) (current : evidence.Contracts)
    (o : export_control.Origin) (e : export_control.ExportEvent)
    (now expires : U64) (session : Slice U8) :
    (do let out ← evidence.Delivery.advance s current o e now expires session
        ok (outcome out.1, state out.2)) =
      ok (Delivery.advance (state s) (ContractsRefinement.state current)
        (origin o) (event e) now.val expires.val (bytes session.val)) := by
  obtain ⟨out, hc, hm⟩ := underlying_success s.«export» o e now expires session
  rcases out with ⟨result, next⟩
  have ho := congrArg Prod.fst hm
  have hn := congrArg Prod.snd hm
  have hp := permitted_refines s current
  cases e <;> cases he : Delivery.permitted (ContractsRefinement.state current) (state s).contract <;>
    simp_all [evidence.Delivery.advance, approve_eq, event, Delivery.advance, state, outcome]

theorem getters_refine (s : evidence.Delivery) :
    (do let c ← evidence.Delivery.impl.contract s
        let r ← evidence.Delivery.request s
        let p ← evidence.Delivery.phase s
        ok (c.map ContractsRefinement.view, request r, phase p)) =
      ok ((state s).contract, (state s).requestState.request, (state s).requestState.phase) := by
  cases hc : s.contract <;>
    simp [evidence.Delivery.impl.contract, evidence.Delivery.request, evidence.Delivery.phase,
      export_control.State.impl.request, export_control.State.impl.phase, state, channel,
      DispatchRefinement.state, hc]

structure Action where
  current : evidence.Contracts
  origin : export_control.Origin
  event : export_control.ExportEvent
  now : U64
  expires : U64
  session : Slice U8

def action (a : Action) : Delivery.Action :=
  ⟨ContractsRefinement.state a.current,
    ⟨origin a.origin, event a.event, a.now.val, a.expires.val, bytes a.session.val⟩⟩

private theorem advance_success (s : evidence.Delivery) (a : Action) :
    ∃ out, evidence.Delivery.advance s a.current a.origin a.event a.now a.expires a.session = ok out ∧
      (outcome out.1, state out.2) = Delivery.advance (state s) (ContractsRefinement.state a.current)
        (origin a.origin) (event a.event) a.now.val a.expires.val (bytes a.session.val) := by
  have h := advance_refines s a.current a.origin a.event a.now a.expires a.session
  cases hc : evidence.Delivery.advance s a.current a.origin a.event a.now a.expires a.session with
  | ret out => exact ⟨out, by simp_all, by simpa [hc] using h⟩
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

def runExtracted (s : evidence.Delivery) : List Action → Result (evidence.Delivery × List Dispatch.Request)
  | [] => ok (s, [])
  | a :: rest => do
    let out ← evidence.Delivery.advance s a.current a.origin a.event a.now a.expires a.session
    let tail ← runExtracted out.2 rest
    ok (tail.1, (outcome out.1).effect.toList ++ tail.2)

theorem trace_refines (s : evidence.Delivery) (actions : List Action) :
    (do let out ← runExtracted s actions; ok (state out.1, out.2)) =
      ok (Delivery.run (state s) (actions.map action)) := by
  induction actions generalizing s with
  | nil => simp [runExtracted, Delivery.run]
  | cons a rest ih =>
    obtain ⟨out, hc, hm⟩ := advance_success s a
    have ho := congrArg Prod.fst hm
    have hs := congrArg Prod.snd hm
    have hr := ih out.2
    cases ht : runExtracted out.2 rest with
    | ret tail =>
      simp only [ht, bind_ok, Result.ok.injEq] at hr
      simp [runExtracted, hc, ht, Delivery.run, action, ← hr, ← hs, ← ho]
    | vis effect continuation => simp [ht] at hr
    | div => simp [ht] at hr

theorem extracted_trace_safe (s final : evidence.Delivery) (requests : List Dispatch.Request)
    (actions : List Action) (h : runExtracted s actions = ok (final, requests)) :
    requests.length ≤ 1 ∧ ∀ r ∈ requests, r = (state s).requestState.request := by
  have ht := trace_refines s actions
  simp only [h, bind_ok, Result.ok.injEq] at ht
  have hs := Delivery.trace_safe (state s) (actions.map action)
  rw [← ht] at hs
  exact hs

theorem dispatch_requires_current (s : evidence.Delivery) (a : Action)
    (out : export_control.Outcome) (next : evidence.Delivery) (dispatch : export_control.Dispatch)
    (h : evidence.Delivery.advance s a.current a.origin a.event a.now a.expires a.session = ok (out, next))
    (he : out.effect = some dispatch) :
    Delivery.permitted (ContractsRefinement.state a.current) (state s).contract = true ∧
      Dispatch.authorized (state s).requestState (origin a.origin) (event a.event)
        a.now.val a.expires.val (bytes a.session.val) ∧
      (state s).requestState.request = request dispatch.request := by
  have ht := advance_refines s a.current a.origin a.event a.now a.expires a.session
  simp only [h, bind_ok, Result.ok.injEq] at ht
  have effect := congrArg (fun pair => pair.1.effect) ht
  simp only [outcome, he, Option.map_some] at effect
  exact (Delivery.effect_iff _ _ _ _ _ _ _ _).mp effect.symm

end MedenSpec.DeliveryRefinement
