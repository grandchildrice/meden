/-! Export review semantics. Origins describe trusted input channels; this
module does not prove that a UI/parser attributes those channels correctly. -/
namespace MedenSpec.ExportControl

inductive Phase where
  | pending | reviewed | rejected | attempted | responseReceived | deliveryUncertain
  deriving DecidableEq, Repr

inductive Origin where
  | model | host | transport
  deriving DecidableEq, Repr

inductive Event where
  | propose | review | reject | approve | responseReceived | deliveryUncertain
  deriving DecidableEq, Repr

structure Decision where
  phase : Phase
  accepted : Bool
  dispatch : Bool
  deriving DecidableEq, Repr

def step (p : Phase) (o : Origin) (e : Event) : Decision :=
  let dispatch := decide (p = .reviewed ∧ o = .host ∧ e = .approve)
  let accepted := decide (
    (o = .model ∧ e = .propose ∧ p = .pending) ∨
    (o = .host ∧ (e = .review ∨
      (e = .reject ∧ (p = .pending ∨ p = .reviewed)) ∨
      (e = .approve ∧ p = .reviewed))) ∨
    (o = .transport ∧ p = .attempted ∧
      (e = .responseReceived ∨ e = .deliveryUncertain)))
  let next :=
    if dispatch then .attempted
    else if o = .host ∧ e = .review ∧ p = .pending then .reviewed
    else if o = .host ∧ e = .reject ∧ (p = .pending ∨ p = .reviewed) then .rejected
    else if o = .transport ∧ p = .attempted ∧ e = .responseReceived then .responseReceived
    else if o = .transport ∧ p = .attempted ∧ e = .deliveryUncertain then .deliveryUncertain
    else p
  ⟨next, accepted, dispatch⟩

/-- Exactly one state/origin/event combination can permit a dispatch. -/
theorem dispatch_iff (p : Phase) (o : Origin) (e : Event) :
    (step p o e).dispatch = true ↔ p = .reviewed ∧ o = .host ∧ e = .approve := by
  simp [step]

private def Closed : Phase → Prop
  | .pending | .reviewed => False
  | _ => True

private theorem closed_preserved (p : Phase) (o : Origin) (e : Event) (h : Closed p) :
    Closed (step p o e).phase := by
  cases p <;> cases o <;> cases e <;> simp_all [step, Closed]

private theorem closed_no_dispatch (p : Phase) (o : Origin) (e : Event) (h : Closed p) :
    (step p o e).dispatch = false := by
  cases p <;> cases o <;> cases e <;> simp_all [step, Closed]

private theorem dispatch_closes (p : Phase) (o : Origin) (e : Event)
    (h : (step p o e).dispatch = true) : Closed (step p o e).phase := by
  obtain ⟨rfl, rfl, rfl⟩ := (dispatch_iff p o e).mp h
  simp [step, Closed]

/-- Final state and total dispatch count for an arbitrary finite event stream. -/
def run (p : Phase) : List (Origin × Event) → Phase × Nat
  | [] => (p, 0)
  | (o, e) :: rest =>
    let d := step p o e
    let tail := run d.phase rest
    (tail.1, (if d.dispatch then 1 else 0) + tail.2)

private theorem closed_trace (p : Phase) (events : List (Origin × Event))
    (h : Closed p) : (run p events).2 = 0 := by
  induction events generalizing p with
  | nil => rfl
  | cons action rest ih =>
    rcases action with ⟨o, e⟩
    simp only [run]
    rw [closed_no_dispatch p o e h, ih _ (closed_preserved p o e h)]
    rfl

/-- No event stream, including later host reviews and forged model/transport
approvals, permits more than one dispatch for one evolving proposal. -/
theorem at_most_one_dispatch (p : Phase) (events : List (Origin × Event)) :
    (run p events).2 ≤ 1 := by
  induction events generalizing p with
  | nil => simp [run]
  | cons action rest ih =>
    rcases action with ⟨o, e⟩
    by_cases h : (step p o e).dispatch = true
    · simp [run, h, closed_trace _ rest (dispatch_closes p o e h)]
    · have hf : (step p o e).dispatch = false := by cases he : (step p o e).dispatch <;> simp_all
      simpa [run, hf] using ih (step p o e).phase

end MedenSpec.ExportControl
