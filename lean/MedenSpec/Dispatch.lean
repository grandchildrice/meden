import MedenSpec.ExportControl

/-! Review and reservation for one immutable typed outbound request. The HTTP
adapter consumes a borrowed dispatch; URI normalization and network I/O remain
separate trust boundaries. -/
namespace MedenSpec.Dispatch

structure Request where
  target : String
  body : List Nat
  deriving DecidableEq, Repr

structure State where
  phase : ExportControl.Phase
  session : List Nat
  request : Request
  deriving DecidableEq, Repr

structure Outcome where
  accepted : Bool
  effect : Option Request
  deriving DecidableEq, Repr

def initial (session : List Nat) (target : String) (body : List Nat) : State :=
  ⟨.pending, session, ⟨target, body⟩⟩

def authorized (s : State) (o : ExportControl.Origin) (e : ExportControl.Event)
    (now expires : Nat) (session : List Nat) : Prop :=
  s.phase = .reviewed ∧ o = .host ∧ e = .approve ∧
    s.session = session ∧ now < expires

instance (s : State) (o : ExportControl.Origin) (e : ExportControl.Event)
    (now expires : Nat) (session : List Nat) :
    Decidable (authorized s o e now expires session) := inferInstanceAs (Decidable (_ ∧ _ ∧ _ ∧ _ ∧ _))

def advance (s : State) (o : ExportControl.Origin) (e : ExportControl.Event)
    (now expires : Nat) (session : List Nat) : Outcome × State :=
  let d := ExportControl.step s.phase o e
  (⟨d.accepted, if authorized s o e now expires session then some s.request else none⟩,
    ⟨d.phase, s.session, s.request⟩)

theorem effect_iff (s : State) (o : ExportControl.Origin) (e : ExportControl.Event)
    (now expires : Nat) (session : List Nat) (r : Request) :
    (advance s o e now expires session).1.effect = some r ↔
      authorized s o e now expires session ∧ s.request = r := by
  by_cases h : authorized s o e now expires session <;> simp [advance, h]

theorem frozen_preserved (s : State) (o : ExportControl.Origin) (e : ExportControl.Event)
    (now expires : Nat) (session : List Nat) :
    (advance s o e now expires session).2.request = s.request := rfl

structure Action where
  origin : ExportControl.Origin
  event : ExportControl.Event
  now : Nat
  expires : Nat
  session : List Nat

def run (s : State) : List Action → State × List Request
  | [] => (s, [])
  | a :: rest =>
    let out := advance s a.origin a.event a.now a.expires a.session
    let tail := run out.2 rest
    (tail.1, out.1.effect.toList ++ tail.2)

theorem trace_frozen (s : State) (actions : List Action) :
    (run s actions).1.request = s.request ∧
      ∀ r ∈ (run s actions).2, r = s.request := by
  induction actions generalizing s with
  | nil => simp [run]
  | cons a rest ih =>
    have h := ih (advance s a.origin a.event a.now a.expires a.session).2
    simp only [run, List.mem_append] at *
    refine ⟨h.1, ?_⟩
    intro r hr
    rcases hr with hr | hr
    · have he := (effect_iff s a.origin a.event a.now a.expires a.session r).mp
        (by simpa using hr)
      exact he.2.symm
    · exact h.2 r hr

private theorem effect_count_le (s : State) (a : Action) :
    (advance s a.origin a.event a.now a.expires a.session).1.effect.toList.length ≤
      if (ExportControl.step s.phase a.origin a.event).dispatch then 1 else 0 := by
  by_cases h : authorized s a.origin a.event a.now a.expires a.session
  · rcases h with ⟨hp, ho, he, _, _⟩
    simp [advance, authorized, hp, ho, he, ExportControl.step]
  · simp [advance, h]

private theorem trace_count_le (s : State) (actions : List Action) :
    (run s actions).2.length ≤
      (ExportControl.run s.phase (actions.map fun a => (a.origin, a.event))).2 := by
  induction actions generalizing s with
  | nil => simp [run, ExportControl.run]
  | cons a rest ih =>
    have h := Nat.add_le_add (effect_count_le s a)
      (ih (advance s a.origin a.event a.now a.expires a.session).2)
    simpa [run, ExportControl.run, advance] using h

theorem at_most_one_effect (s : State) (actions : List Action) :
    (run s actions).2.length ≤ 1 :=
  Nat.le_trans (trace_count_le s actions) (ExportControl.at_most_one_dispatch _ _)

end MedenSpec.Dispatch
