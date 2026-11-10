import MedenSpec.Contracts

/-! A project check retains its original agreement/data across preparation,
issue and completion. Checker/encoding/persistence observations are adapters. -/
namespace MedenSpec.CheckTransaction
open Evidence

structure Preparation (T : Type) where
  agreement : Contracts.View
  data : T

structure Running (T : Type) where
  agreement : Contracts.View
  ticket : CheckJobs.Ticket
  data : T

def identity (view : Contracts.View) :=
  (view.revision, view.subject.session, view.subject.contract, view.obligations)

def begin {T : Type} (current : Contracts.State) (agreement : Contracts.View) (data : T) :
    Option (Preparation T) × Contracts.State :=
  if Contracts.isAccepted current = true ∧ agreement.obligations.length = 1 ∧
      Contracts.accepted current = some agreement then
    (some ⟨agreement, data⟩, (Contracts.edit current .controller).2)
  else (none, current)

def issue {T : Type} (prepared : Preparation T) (current : Contracts.State) (inputs : List Nat) :
    Option (Running T) × Contracts.State :=
  if Contracts.isAccepted current then
    match Contracts.accepted current with
    | none => (none, current)
    | some view =>
      if identity view = identity prepared.agreement then
        let (ticket, next) := Contracts.start current .controller prepared.agreement.revision inputs
        (ticket.map (fun ticket => ⟨prepared.agreement, ticket, prepared.data⟩), next)
      else (none, current)
  else (none, current)

def refuse {U : Type} (running : Running T) (current : Contracts.State) :
    Option U × Contracts.State :=
  (none, (Contracts.abort current .controller running.ticket).2)

def complete {T U : Type} (running : Running T) (current : Contracts.State)
    (inputs outputInputs checker : List Nat) (output : U) : Option U × Contracts.State :=
  match running.agreement.obligations with
  | [] => refuse running current
  | required :: _ =>
    if checker ≠ required.checker then refuse running current
    else
      let result : Completion.Output :=
        ⟨{running.agreement.subject with inputs := outputInputs}, 0, required, true⟩
      let (admitted, next) := Contracts.complete current .checker
        running.agreement.revision running.ticket inputs result
      if admitted && Contracts.eligible next running.agreement.revision outputInputs then
        (some output, next)
      else refuse running next

theorem begin_original {T : Type} (current : Contracts.State) (agreement : Contracts.View)
    (data : T) (prepared : Preparation T) (next : Contracts.State)
    (h : begin current agreement data = (some prepared, next)) :
    prepared.agreement = agreement ∧ prepared.data = data ∧
      Contracts.accepted current = some agreement ∧ agreement.obligations.length = 1 := by
  simp only [begin] at h
  split at h <;> simp_all
  rcases h with ⟨eq, _⟩
  cases eq
  simp_all

theorem issue_original {T : Type} (prepared : Preparation T) (current : Contracts.State)
    (inputs : List Nat) (running : Running T) (next : Contracts.State)
    (h : issue prepared current inputs = (some running, next)) :
    running.agreement = prepared.agreement ∧ running.data = prepared.data := by
  unfold issue at h
  split at h <;> try simp_all
  split at h <;> try simp_all
  split at h <;> try simp_all
  generalize Contracts.start current .controller prepared.agreement.revision inputs = out at h
  rcases out with ⟨ticket, state⟩
  cases ticket <;> simp_all
  rcases h with ⟨eq, _⟩
  cases eq
  simp_all

theorem output_original {T U : Type} (running : Running T) (current : Contracts.State)
    (inputs outputInputs checker : List Nat) (output delivered : U) (next : Contracts.State)
    (h : complete running current inputs outputInputs checker output = (some delivered, next)) :
    delivered = output ∧ Contracts.eligible next running.agreement.revision outputInputs = true := by
  unfold complete at h
  split at h <;> simp only [refuse] at h <;> try simp_all
  split at h <;> try simp_all
  generalize Contracts.complete current .checker running.agreement.revision running.ticket inputs _ =
    out at h
  rcases out with ⟨admitted, state⟩
  split at h <;> simp_all

end MedenSpec.CheckTransaction
