import MedenSpec.Contracts
import MedenSpec.Dispatch

/-! Evidence and outbound review are independent requirements. A prepared
request freezes its complete contract view; current evidence is checked again
at reservation. File packaging, freshness inspection and I/O are adapters. -/
namespace MedenSpec.Delivery

structure State where
  contract : Option Contracts.View
  requestState : Dispatch.State
  deriving DecidableEq

def permitted (current : Contracts.State) (frozen : Option Contracts.View) : Bool :=
  match frozen with
  | none => (Contracts.displayed current).isNone
  | some v => decide (Contracts.accepted current = some v) &&
      Contracts.eligible current v.revision v.subject.inputs

def prepare (current : Contracts.State) (target : String) (body : List Nat) : Option State :=
  let frozen := Contracts.accepted current
  if permitted current frozen then
    some ⟨frozen, Dispatch.initial current.session target body⟩
  else none

def advance (s : State) (current : Contracts.State)
    (origin : ExportControl.Origin) (event : ExportControl.Event)
    (now expires : Nat) (session : List Nat) : Dispatch.Outcome × State :=
  if event = .approve ∧ permitted current s.contract = false then
    (⟨false, none⟩, s)
  else
    let out := Dispatch.advance s.requestState origin event now expires session
    (out.1, { s with requestState := out.2 })

theorem effect_iff (s : State) (current : Contracts.State)
    (origin : ExportControl.Origin) (event : ExportControl.Event)
    (now expires : Nat) (session : List Nat) (request : Dispatch.Request) :
    (advance s current origin event now expires session).1.effect = some request ↔
      permitted current s.contract = true ∧
      Dispatch.authorized s.requestState origin event now expires session ∧
      s.requestState.request = request := by
  cases event <;> cases hp : permitted current s.contract <;>
    simp [advance, hp, Dispatch.advance, Dispatch.authorized]

theorem raw_cannot_bypass_contract (s : State) (current : Contracts.State)
    (origin : ExportControl.Origin) (now expires : Nat) (session : List Nat)
    (hs : s.contract = none) (hc : (Contracts.displayed current).isSome = true) :
    (advance s current origin .approve now expires session).1.effect = none := by
  cases hd : Contracts.displayed current <;> simp_all [advance, permitted]

theorem frozen_preserved (s : State) (current : Contracts.State)
    (origin : ExportControl.Origin) (event : ExportControl.Event)
    (now expires : Nat) (session : List Nat) :
    (advance s current origin event now expires session).2.contract = s.contract ∧
    (advance s current origin event now expires session).2.requestState.request = s.requestState.request := by
  simp only [advance]
  split <;> exact ⟨rfl, rfl⟩

structure Action where
  current : Contracts.State
  operation : Dispatch.Action

def allowed (frozen : Option Contracts.View) (a : Action) : Bool :=
  decide (a.operation.event ≠ .approve) || permitted a.current frozen

def run (s : State) : List Action → State × List Dispatch.Request
  | [] => (s, [])
  | a :: rest =>
    let op := a.operation
    let out := advance s a.current op.origin op.event op.now op.expires op.session
    let tail := run out.2 rest
    (tail.1, out.1.effect.toList ++ tail.2)

theorem trace_filters_denied (s : State) (actions : List Action) :
    (run s actions).1.contract = s.contract ∧
    ((run s actions).1.requestState, (run s actions).2) =
      Dispatch.run s.requestState ((actions.filter (allowed s.contract)).map Action.operation) := by
  induction actions generalizing s with
  | nil => simp [run, Dispatch.run]
  | cons a rest ih =>
    have binding (t : State) := (ih t).1
    have nextState (t : State) :
        (run t rest).1.requestState =
          (Dispatch.run t.requestState ((rest.filter (allowed t.contract)).map Action.operation)).1 :=
      congrArg Prod.fst (ih t).2
    have requests (t : State) :
        (run t rest).2 =
          (Dispatch.run t.requestState ((rest.filter (allowed t.contract)).map Action.operation)).2 :=
      congrArg Prod.snd (ih t).2
    by_cases he : a.operation.event = .approve <;>
      cases hp : permitted a.current s.contract <;>
      simp [run, advance, allowed, he, hp, Dispatch.run, binding, nextState, requests]

theorem trace_safe (s : State) (actions : List Action) :
    (run s actions).2.length ≤ 1 ∧
      ∀ request ∈ (run s actions).2, request = s.requestState.request := by
  have hf := trace_filters_denied s actions
  have hb := Dispatch.at_most_one_effect s.requestState
    ((actions.filter (allowed s.contract)).map Action.operation)
  have hr := Dispatch.trace_frozen s.requestState
    ((actions.filter (allowed s.contract)).map Action.operation)
  rw [← hf.2] at hb hr
  exact ⟨hb, hr.2⟩

end MedenSpec.Delivery
