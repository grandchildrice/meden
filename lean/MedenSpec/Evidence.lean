import Lean.Elab.Tactic.Omega

/-! Accepted obligations and exact evidence correspondence. Checker results are
trusted only after host-adapter validation; this module does not prove checker
soundness, byte encoding, snapshot construction or attribution of input origins. -/
namespace MedenSpec.Evidence

inductive Kind where
  | empirical | bounded | theorem | refinement
  deriving DecidableEq, Repr

structure Subject where
  session : List Nat
  contract : List Nat
  inputs : List Nat
  deriving DecidableEq, Repr

structure Obligation where
  statement : List Nat
  kind : Kind
  checker : List Nat
  deriving DecidableEq, Repr

inductive Origin where
  | model | host | checker | controller
  deriving DecidableEq, Repr

inductive Phase where
  | draft | reviewed | accepted
  deriving DecidableEq, Repr

structure Receipt where
  subject : Subject
  position : Nat
  obligation : Obligation
  passed : Bool
  active : Bool
  deriving DecidableEq, Repr

structure Gate where
  subject : Subject
  obligations : List Obligation
  phase : Phase
  completed : Nat
  deriving DecidableEq, Repr

def propose (subject : Subject) (obligations : List Obligation) : Gate :=
  ⟨subject, obligations, .draft, 0⟩

def review (g : Gate) (o : Origin) : Bool × Gate :=
  if o = .host then (true, if g.phase = .draft then {g with phase := .reviewed} else g)
  else (false, g)

def accept (g : Gate) (o : Origin) : Bool × Gate :=
  if o = .host ∧ g.phase = .reviewed ∧ g.obligations ≠ []
  then (true, {g with phase := .accepted}) else (false, g)

def refresh (g : Gate) (o : Origin) (s : Subject) : Bool × Gate :=
  if o = .controller ∧ g.subject.session = s.session ∧ g.subject.contract = s.contract
  then (true, if g.subject = s then g else {g with subject := s, completed := 0})
  else (false, g)

def corresponds (g : Gate) (o : Origin) (current : Subject) (r : Receipt) : Prop :=
  o = .checker ∧ g.phase = .accepted ∧ g.subject = current ∧ r.subject = current ∧
  r.position = g.completed ∧ r.passed = true ∧ r.active = true ∧ g.obligations[g.completed]? = some r.obligation

instance (g : Gate) (o : Origin) (current : Subject) (r : Receipt) :
    Decidable (corresponds g o current r) := inferInstanceAs (Decidable (_ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _ ∧ _))

def record (g : Gate) (o : Origin) (current : Subject) (r : Receipt) : Bool × Gate :=
  if corresponds g o current r then (true, {g with completed := g.completed + 1}) else (false, g)

def eligible (g : Gate) (current : Subject) : Bool :=
  decide (g.phase = .accepted ∧ g.obligations ≠ [] ∧
    g.completed = g.obligations.length ∧ g.subject = current)

theorem record_iff (g : Gate) (o : Origin) (s : Subject) (r : Receipt) :
    (record g o s r).1 = true ↔ corresponds g o s r := by
  by_cases h : corresponds g o s r <;> simp [record, h]

theorem model_cannot_record (g : Gate) (s : Subject) (r : Receipt) :
    record g .model s r = (false, g) := by simp [record, corresponds]

theorem stale_cannot_record (g : Gate) (o : Origin) (s : Subject) (r : Receipt)
    (h : g.subject ≠ s ∨ r.subject ≠ s) : record g o s r = (false, g) := by
  rcases h with h | h <;> simp [record, corresponds, h]

theorem no_replay (g : Gate) (o laterOrigin : Origin) (s laterSubject : Subject) (r : Receipt)
    (h : (record g o s r).1 = true) :
    (record (record g o s r).2 laterOrigin laterSubject r).1 = false := by
  have hm := (record_iff g o s r).mp h
  have hp := hm.2.2.2.2.1
  simp only [record, ite_eq_left hm]
  have hn : r.position ≠ g.completed + 1 := by omega
  simp [corresponds, hn]

theorem inactive_cannot_record (g : Gate) (o : Origin) (s : Subject) (r : Receipt)
    (h : r.active = false) : record g o s r = (false, g) := by
  simp [record, corresponds, h]

theorem no_kind_substitution (g : Gate) (o : Origin) (s : Subject) (r : Receipt)
    (expected : Obligation) (he : g.obligations[g.completed]? = some expected)
    (hk : expected.kind ≠ r.obligation.kind) : (record g o s r).1 = false := by
  have hn : ¬ corresponds g o s r := by
    intro h
    have eq := h.2.2.2.2.2.2.2
    rw [he] at eq
    have ho := Option.some.inj eq
    exact hk (congrArg Obligation.kind ho)
  simp [record, hn]

theorem changed_inputs_invalidate (g : Gate) (s : Subject)
    (hs : g.subject.session = s.session) (hc : g.subject.contract = s.contract)
    (hn : g.subject ≠ s) : eligible (refresh g .controller s).2 s = false := by
  by_cases he : g.obligations = []
  · simp [refresh, hs, hc, hn, eligible, he]
  · have hl : g.obligations.length ≠ 0 := fun h => he (List.length_eq_zero_iff.mp h)
    simp [refresh, hs, hc, hn, eligible, Ne.symm hl]

theorem eligible_iff (g : Gate) (s : Subject) : eligible g s = true ↔
    g.phase = .accepted ∧ g.obligations ≠ [] ∧
      g.completed = g.obligations.length ∧ g.subject = s := by simp [eligible]

/-- A ghost audit log: every completed position has successful evidence for
the exact subject and exact obligation. The log may retain older subjects. -/
def Covered (g : Gate) (history : List Receipt) : Prop :=
  g.completed ≤ g.obligations.length ∧ ∀ i, i < g.completed → ∃ r ∈ history,
    r.subject = g.subject ∧ r.position = i ∧ r.passed = true ∧ r.active = true ∧
      g.obligations[i]? = some r.obligation

private theorem propose_covered (s : Subject) (requirements : List Obligation) :
    Covered (propose s requirements) [] := by simp [Covered, propose]

private theorem record_covered (g : Gate) (o : Origin) (s : Subject) (r : Receipt)
    (history : List Receipt) (h : Covered g history) :
    Covered (record g o s r).2 (if (record g o s r).1 then r :: history else history) := by
  by_cases hm : corresponds g o s r
  · rcases hm with ⟨ho, hp, hgs, hrs, hrp, hpass, hactive, hget⟩
    have hm : corresponds g o s r := ⟨ho, hp, hgs, hrs, hrp, hpass, hactive, hget⟩
    rcases (List.getElem?_eq_some_iff.mp hget) with ⟨hlt, _⟩
    simp only [record, ite_eq_left hm, ite_true, Covered]
    refine ⟨Nat.succ_le_of_lt hlt, ?_⟩
    intro i hi
    by_cases he : i = g.completed
    · subst i
      exact ⟨r, by simp, hrs.trans hgs.symm, hrp, hpass, hactive, hget⟩
    · have hi' : i < g.completed := by omega
      obtain ⟨old, hold, hs, hpos, hpassed, hactive, hreq⟩ := h.2 i hi'
      exact ⟨old, by simp [hold], hs, hpos, hpassed, hactive, hreq⟩
  · simpa [record, hm] using h

inductive Event where
  | review (origin : Origin)
  | accept (origin : Origin)
  | refresh (origin : Origin) (subject : Subject)
  | record (origin : Origin) (subject : Subject) (receipt : Receipt)

def step (g : Gate) (history : List Receipt) : Event → Gate × List Receipt
  | .review o => ((review g o).2, history)
  | .accept o => ((accept g o).2, history)
  | .refresh o s => ((refresh g o s).2, history)
  | .record o s r => ((record g o s r).2, if (record g o s r).1 then r :: history else history)

private theorem step_covered (g : Gate) (history : List Receipt) (event : Event)
    (h : Covered g history) : Covered (step g history event).1 (step g history event).2 := by
  cases event with
  | review o =>
    simp only [step, review]
    split <;> simp_all [Covered]
    split <;> simpa [Covered] using h
  | accept o =>
    simp only [step, accept]
    split <;> simpa [Covered] using h
  | refresh o s =>
    simp only [step, refresh]
    split
    · split
      · exact h
      · simp [Covered]
    · exact h
  | record o s r => exact record_covered g o s r history h

def run (g : Gate) (history : List Receipt) : List Event → Gate × List Receipt
  | [] => (g, history)
  | event :: rest => let next := step g history event; run next.1 next.2 rest

theorem trace_covered (g : Gate) (history : List Receipt) (events : List Event)
    (h : Covered g history) : Covered (run g history events).1 (run g history events).2 := by
  induction events generalizing g history with
  | nil => exact h
  | cons event rest ih => exact ih _ _ (step_covered g history event h)

/-- Any eligible finite trace starting from an untrusted proposal has current,
successful evidence for every exact accepted obligation. No event count bound. -/
theorem eligible_requires_all_evidence (s current : Subject) (requirements : List Obligation)
    (events : List Event) (he : eligible (run (propose s requirements) [] events).1 current = true) :
    ∀ i, i < (run (propose s requirements) [] events).1.obligations.length →
      ∃ r ∈ (run (propose s requirements) [] events).2,
        r.subject = current ∧ r.position = i ∧ r.passed = true ∧ r.active = true ∧
          (run (propose s requirements) [] events).1.obligations[i]? = some r.obligation := by
  have h := trace_covered _ _ events (propose_covered s requirements)
  have hm := (eligible_iff _ _).mp he
  intro i hi
  obtain ⟨r, hr, hs, hp, hpass, hactive, hreq⟩ := h.2 i (by simpa [hm.2.2.1] using hi)
  exact ⟨r, hr, hs.trans hm.2.2.2, hp, hpass, hactive, hreq⟩

end MedenSpec.Evidence
