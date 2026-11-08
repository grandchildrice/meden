import MedenSpec.Evidence

namespace MedenSpec.CheckJobs
open Evidence

structure Ticket where
  id : Nat
  subject : Subject
  deriving DecidableEq

structure State where
  session : List Nat
  sequence : Nat
  active : Option Ticket
  deriving DecidableEq

def initial (session : List Nat) : State := ⟨session, 0, none⟩

def permitted (j : State) (g : Gate) (s : Subject) : Prop :=
  g.phase = .accepted ∧ j.session = s.session ∧
    g.subject.session = s.session ∧ g.subject.contract = s.contract

instance (j : State) (g : Gate) (s : Subject) : Decidable (permitted j g s) :=
  inferInstanceAs (Decidable (_ ∧ _ ∧ _ ∧ _))

def start (j : State) (o : Origin) (g : Gate) (s : Subject) : Option Ticket × State :=
  if o = .controller ∧ permitted j g s ∧ j.sequence < 18446744073709551615 then
    let t := Ticket.mk (j.sequence + 1) s
    (some t, {j with sequence := j.sequence + 1, active := some t})
  else (none, j)

def invalidate (j : State) (o : Origin) : Bool × State :=
  if o = .controller then (true, {j with active := none}) else (false, j)

def abort (j : State) (o : Origin) (t : Ticket) : Bool × State :=
  if j.active = some t then invalidate j o else (false, j)

def finish (j : State) (o : Origin) (g : Gate) (t : Ticket) (current : Subject) : Bool × State :=
  if o = .checker ∧ permitted j g current ∧ j.active = some t ∧ t.subject = current
  then (true, {j with active := none}) else (false, j)

inductive Action where
  | start (o : Origin) (gate : Gate) (subject : Subject)
  | invalidate (o : Origin)
  | abort (o : Origin) (ticket : Ticket)
  | finish (o : Origin) (gate : Gate) (ticket : Ticket) (current : Subject)

def step (j : State) : Action → State
  | .start o g s => (start j o g s).2
  | .invalidate o => (invalidate j o).2
  | .abort o t => (abort j o t).2
  | .finish o g t s => (finish j o g t s).2

def log (j : State) (history : List Ticket) : Action → List Ticket
  | .start o g s => match (start j o g s).1 with
    | some t => t :: history | none => history
  | _ => history

def run (j : State) (history : List Ticket) : List Action → State × List Ticket
  | [] => (j, history)
  | a :: rest => run (step j a) (log j history a) rest

/-- No active job can refer to an already retired generation. -/
def After (floor : Nat) (j : State) : Prop :=
  floor ≤ j.sequence ∧ ∀ t, j.active = some t → floor < t.id

private theorem step_after (floor : Nat) (j : State) (a : Action) (h : After floor j) :
    After floor (step j a) := by
  cases a <;> simp only [step]
  · unfold start
    split
    · have hb := h.1
      simp [After]; omega
    · exact h
  · unfold invalidate
    split
    · simpa [After] using h.1
    · exact h
  · unfold abort
    split
    · unfold invalidate
      split
      · simpa [After] using h.1
      · exact h
    · exact h
  · unfold finish
    split
    · simpa [After] using h.1
    · exact h

theorem trace_after (floor : Nat) (j : State) (history : List Ticket) (actions : List Action)
    (h : After floor j) : After floor (run j history actions).1 := by
  induction actions generalizing j history with
  | nil => exact h
  | cons a rest ih => exact ih _ _ (step_after floor j a h)

theorem no_late_result (j : State) (history : List Ticket) (actions : List Action)
    (hi : j.active = none) (t : Ticket) (ht : t.id ≤ j.sequence)
    (o : Origin) (g : Gate) (s : Subject) :
    (finish (run j history actions).1 o g t s).1 = false := by
  have h := trace_after j.sequence j history actions (by simp [After, hi])
  unfold finish
  split
  · rename_i hm
    have := h.2 t hm.2.2.1
    omega
  · rfl

def Issued (j : State) (history : List Ticket) : Prop :=
  ∀ t, j.active = some t → t ∈ history

private theorem step_issued (j : State) (history : List Ticket) (a : Action)
    (h : Issued j history) : Issued (step j a) (log j history a) := by
  cases a <;> simp only [step, log]
  · unfold start
    split <;> simp_all [Issued]
  · unfold invalidate
    split <;> simp_all [Issued]
  · unfold abort
    split
    · unfold invalidate; split <;> simp_all [Issued]
    · exact h
  · unfold finish
    split <;> simp_all [Issued]

theorem trace_issued (j : State) (history : List Ticket) (actions : List Action)
    (h : Issued j history) : Issued (run j history actions).1 (run j history actions).2 := by
  induction actions generalizing j history with
  | nil => exact h
  | cons a rest ih => exact ih _ _ (step_issued j history a h)

end MedenSpec.CheckJobs
