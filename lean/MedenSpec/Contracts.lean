import MedenSpec.Completion
import Mathlib.Tactic.SplitIfs

/-! Contract selection owns its frozen meaning, review generation and jobs.
The adapter validates formula data and constructs complete input bindings.
This specification does not assume model-origin events can be host events. -/
namespace MedenSpec.Contracts
open Evidence

structure Versioned where
  revision : Nat
  gate : Evidence.Gate
  deriving DecidableEq

structure View where
  revision : Nat
  subject : Evidence.Subject
  obligations : List Evidence.Obligation
  deriving DecidableEq

structure State where
  session : List Nat
  sequence : Nat
  accepted : Option Versioned
  draft : Option Versioned
  jobs : CheckJobs.State
  deriving DecidableEq

def initial (session : List Nat) : State :=
  ⟨session, 0, none, none, CheckJobs.initial session⟩

def selected (s : State) : Option Versioned := s.draft.or s.accepted
def view (v : Versioned) : View := ⟨v.revision, v.gate.subject, v.gate.obligations⟩
def displayed (s : State) : Option View := (selected s).map view
def accepted (s : State) : Option View := s.accepted.map view
def pending (s : State) : Bool := s.draft.isSome
def isAccepted (s : State) : Bool :=
  s.draft.isNone && s.accepted.any (fun v => v.gate.phase == .accepted)

def sameProposal (v : Versioned) (contract : List Nat) (obligations : List Evidence.Obligation) : Prop :=
  v.gate.subject.contract = contract ∧ v.gate.obligations = obligations
instance (v : Versioned) (c : List Nat) (o : List Evidence.Obligation) : Decidable (sameProposal v c o) :=
  inferInstanceAs (Decidable (_ ∧ _))

def fresh (s : State) (contract : List Nat) (obligations : List Evidence.Obligation) : Bool × State :=
  if s.sequence < 18446744073709551615 then
    (true, {s with
      sequence := s.sequence + 1,
      draft := some ⟨s.sequence + 1, Evidence.propose ⟨s.session, contract, []⟩ obligations⟩,
      jobs := (CheckJobs.invalidate s.jobs .controller).2})
  else (false, s)

def propose (s : State) (origin : Origin) (contract : List Nat)
    (obligations : List Evidence.Obligation) : Bool × State :=
  if origin ≠ .controller ∨ contract = [] ∨ obligations = [] then (false, s)
  else if (selected s).any (fun v => decide (sameProposal v contract obligations)) then (true, s)
  else fresh s contract obligations

def reviewOne (v : Versioned) (origin : Origin) (revision : Nat) : Bool × Versioned :=
  if v.revision = revision then
    let (allowed, gate) := Evidence.review v.gate origin
    (allowed, {v with gate})
  else (false, v)

def review (s : State) (origin : Origin) (session : List Nat) (revision : Nat) : Bool × State :=
  if s.session ≠ session then (false, s)
  else match s.draft with
    | some d => let (allowed, d) := reviewOne d origin revision
                (allowed, {s with draft := some d})
    | none => match s.accepted with
      | some a => let (allowed, a) := reviewOne a origin revision
                  (allowed, {s with accepted := some a})
      | none => (false, s)

def accept (s : State) (origin : Origin) (session : List Nat) (revision : Nat) : Bool × State :=
  if s.session ≠ session then (false, s)
  else match s.draft with
    | none => (false, s)
    | some d =>
      if d.revision ≠ revision then (false, s)
      else let (allowed, gate) := Evidence.accept d.gate origin
           if allowed then (true, {s with
             accepted := some {d with gate}, draft := none,
             jobs := (CheckJobs.invalidate s.jobs .controller).2})
           else (false, s)

def reject (s : State) (origin : Origin) (session : List Nat) (revision : Nat) : Bool × State :=
  if origin ≠ .host ∨ s.session ≠ session then (false, s)
  else match s.draft with
    | some d => if d.revision = revision then (true, {s with draft := none}) else (false, s)
    | none => (false, s)

def withInputs (g : Evidence.Gate) (inputs : List Nat) : Evidence.Subject := {g.subject with inputs}

def refresh (s : State) (origin : Origin) (revision : Nat) (inputs : List Nat) : Bool × State :=
  match s.accepted with
  | none => (false, s)
  | some a =>
    if a.revision ≠ revision then (false, s)
    else let (allowed, gate) := Evidence.refresh a.gate origin (withInputs a.gate inputs)
         (allowed, {s with accepted := some {a with gate}})

def start (s : State) (origin : Origin) (revision : Nat) (inputs : List Nat) : Option CheckJobs.Ticket × State :=
  if origin ≠ .controller ∨ s.draft.isSome then (none, s)
  else match s.accepted with
    | none => (none, s)
    | some a =>
      if a.revision ≠ revision then (none, s)
      else let subject := withInputs a.gate inputs
           let jobs := (CheckJobs.invalidate s.jobs .controller).2
           let (allowed, gate) := Evidence.refresh a.gate origin subject
           let s := {s with jobs, accepted := some {a with gate}}
           if !allowed then (none, s)
           else let (ticket, jobs) := CheckJobs.start jobs origin gate subject
                (ticket, {s with jobs})

def invalidate (s : State) (origin : Origin) : Bool × State :=
  let (allowed, jobs) := CheckJobs.invalidate s.jobs origin
  (allowed, {s with jobs})

def edit (s : State) (origin : Origin) : Bool × State :=
  if origin ≠ .controller then (false, s)
  else (true, {s with
    jobs := (CheckJobs.invalidate s.jobs origin).2,
    accepted := s.accepted.map (fun a => {a with gate := {a.gate with completed := 0}})})

def abort (s : State) (origin : Origin) (ticket : CheckJobs.Ticket) : Bool × State :=
  let (allowed, jobs) := CheckJobs.abort s.jobs origin ticket
  (allowed, {s with jobs})

def complete (s : State) (origin : Origin) (revision : Nat) (ticket : CheckJobs.Ticket)
    (inputs : List Nat) (result : Completion.Output) : Bool × State :=
  if s.draft.isSome then (false, s)
  else match s.accepted with
    | none => (false, s)
    | some a =>
      if a.revision ≠ revision then (false, s)
      else let (allowed, jobs, gate) := Completion.complete s.jobs a.gate origin ticket
             (withInputs a.gate inputs) result
           (allowed, {s with jobs, accepted := some {a with gate}})

def eligible (s : State) (revision : Nat) (inputs : List Nat) : Bool :=
  if s.draft.isSome || s.jobs.active.isSome then false
  else match s.accepted with
    | none => false
    | some a => a.revision == revision && Evidence.eligible a.gate (withInputs a.gate inputs)

inductive Action where
  | propose (origin : Origin) (contract : List Nat) (obligations : List Evidence.Obligation)
  | review (origin : Origin) (session : List Nat) (revision : Nat)
  | accept (origin : Origin) (session : List Nat) (revision : Nat)
  | reject (origin : Origin) (session : List Nat) (revision : Nat)
  | refresh (origin : Origin) (revision : Nat) (inputs : List Nat)
  | start (origin : Origin) (revision : Nat) (inputs : List Nat)
  | invalidate (origin : Origin)
  | edit (origin : Origin)
  | abort (origin : Origin) (ticket : CheckJobs.Ticket)
  | complete (origin : Origin) (revision : Nat) (ticket : CheckJobs.Ticket)
      (inputs : List Nat) (result : Completion.Output)

def step (s : State) : Action → State
  | .propose o c obligations => (propose s o c obligations).2
  | .review o session revision => (review s o session revision).2
  | .accept o session revision => (accept s o session revision).2
  | .reject o session revision => (reject s o session revision).2
  | .refresh o revision inputs => (refresh s o revision inputs).2
  | .start o revision inputs => (start s o revision inputs).2
  | .invalidate o => (invalidate s o).2
  | .edit o => (edit s o).2
  | .abort o ticket => (abort s o ticket).2
  | .complete o revision ticket inputs result => (complete s o revision ticket inputs result).2

def run (s : State) : List Action → State
  | [] => s
  | a :: rest => run (step s a) rest

theorem acceptance_requires_review (s : State) (o : Origin) (session : List Nat) (revision : Nat)
    (h : (accept s o session revision).1 = true) :
    o = .host ∧ session = s.session ∧ ∃ d, s.draft = some d ∧ d.revision = revision ∧
      d.gate.phase = .reviewed ∧ d.gate.obligations ≠ [] ∧
      (accept s o session revision).2.accepted = some {d with gate := {d.gate with phase := .accepted}} := by
  unfold accept at h ⊢
  split at h
  · simp at h
  · rename_i hs
    cases hd : s.draft with
    | none => simp [hd] at h
    | some d =>
      simp only [hd] at h
      split at h
      · simp at h
      · rename_i hr
        have he : o = .host ∧ d.gate.phase = .reviewed ∧ d.gate.obligations ≠ [] := by
          by_cases he : o = .host ∧ d.gate.phase = .reviewed ∧ d.gate.obligations ≠ []
          · exact he
          · simp [Evidence.accept, he] at h
        exact ⟨he.1, (Classical.not_not.mp hs).symm, d, rfl, Classical.not_not.mp hr, he.2.1, he.2.2,
          by simp [hs, hr, Evidence.accept, he]⟩

theorem propose_preserves_accepted (s : State) (o : Origin) (c : List Nat) (obs : List Obligation) :
    (propose s o c obs).2.accepted = s.accepted := by
  unfold propose fresh
  split_ifs <;> rfl

theorem fresh_proposal_has_new_revision (s : State) (o : Origin) (c : List Nat) (obs : List Obligation)
    (h : (propose s o c obs).2.draft ≠ s.draft) :
    ∃ d, (propose s o c obs).2.draft = some d ∧ d.revision = s.sequence + 1 ∧
      (propose s o c obs).2.sequence = d.revision := by
  unfold propose fresh at h ⊢
  split_ifs <;> simp_all

theorem review_requires_host (s : State) (o : Origin) (session : List Nat) (revision : Nat)
    (h : (review s o session revision).1 = true) : o = .host := by
  by_cases ho : o = .host
  · exact ho
  · cases hd : s.draft <;> cases ha : s.accepted <;>
      simp [review, reviewOne, Evidence.review, ho, hd, ha] at h
    all_goals split_ifs at h <;> simp_all

theorem reject_preserves_accepted (s : State) (o : Origin) (session : List Nat) (revision : Nat) :
    (reject s o session revision).2.accepted = s.accepted := by
  unfold reject
  split_ifs <;> cases s.draft <;> simp
  split_ifs <;> rfl

theorem edit_discards_eligibility (s : State) (revision : Nat) (inputs : List Nat) :
    eligible (edit s .controller).2 revision inputs = false := by
  cases ha : s.accepted <;> cases hd : s.draft <;>
    simp [edit, eligible, ha, hd, Evidence.eligible]
  intro _ _ _ hn hl
  exact False.elim (hn (List.length_eq_zero_iff.mp hl.symm))

theorem pending_blocks_completion (s : State) (o : Origin) (revision : Nat) (t : CheckJobs.Ticket)
    (inputs : List Nat) (r : Completion.Output) (h : s.draft.isSome = true) :
    complete s o revision t inputs r = (false, s) := by simp [complete, h]

theorem step_sequence_monotone (s : State) (a : Action) : s.sequence ≤ (step s a).sequence := by
  cases a with
  | review o session revision =>
    simp only [step, review]
    split_ifs
    · exact Nat.le_refl _
    · cases hd : s.draft with
      | some d => cases reviewOne d o revision <;> simp
      | none =>
        cases ha : s.accepted with
        | none => exact Nat.le_refl _
        | some a => cases reviewOne a o revision <;> simp
  | _ =>
    simp only [step]
    first
      | (unfold propose fresh; split_ifs <;> simp)
      | (unfold accept; split_ifs <;> cases s.draft <;> simp; split_ifs <;> simp)
      | (unfold reject; split_ifs <;> cases s.draft <;> simp; split_ifs <;> simp)
      | (unfold refresh; cases s.accepted <;> simp; split_ifs <;> simp)
      | (unfold start; split_ifs <;> cases s.accepted <;> simp; split_ifs <;> simp)
      | (unfold invalidate; simp)
      | (unfold edit; split_ifs <;> simp)
      | (unfold abort; simp)
      | (unfold complete; split_ifs <;> cases s.accepted <;> simp; split_ifs <;> simp)

theorem run_sequence_monotone (s : State) (actions : List Action) : s.sequence ≤ (run s actions).sequence := by
  induction actions generalizing s with
  | nil => exact Nat.le_refl _
  | cons a rest ih => exact Nat.le_trans (step_sequence_monotone s a) (ih (step s a))

end MedenSpec.Contracts
