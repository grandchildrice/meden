/-! Independent specification of a session's frozen inference connection.
Authorization covers repeated conversation exchanges until the fixed deadline;
it is distinct from single-use artifact export. Time is host-provided. -/
namespace MedenSpec.Processor

inductive Origin where
  | host | model
  deriving DecidableEq, Repr

inductive Phase where
  | pending | granted | revoked
  deriving DecidableEq, Repr

structure Local where
  engine : List Nat
  weights : List Nat
  deriving DecidableEq, Repr

structure Http where
  provider : List Nat
  endpoint : List Nat
  model : List Nat
  credential : Option (List Nat)
  deriving DecidableEq, Repr

inductive Binding where
  | local : Local → Binding
  | http : Http → Binding
  deriving DecidableEq, Repr

structure State where
  session : List Nat
  binding : Binding
  phase : Phase
  expires : Nat
  deriving DecidableEq, Repr

def lifetime : Nat := 28800000
def maxTime : Nat := 18446744073709551615

def initial (session : List Nat) (binding : Binding) : State :=
  ⟨session, binding, .pending, 0⟩

def approve (s : State) (o : Origin) (now : Nat) : Bool × State :=
  if o = .host ∧ s.phase = .pending ∧ now + lifetime ≤ maxTime then
    (true, { s with phase := .granted, expires := now + lifetime })
  else (false, s)

def revoke (s : State) (o : Origin) : Bool × State :=
  if o = .host then (true, { s with phase := .revoked }) else (false, s)

def authorized (s : State) (session : List Nat) (now : Nat) : Bool :=
  decide (s.phase = .granted ∧ s.session = session ∧ now < s.expires)

def dispatch (s : State) (session : List Nat) (now : Nat) : Option Binding × State :=
  let s' := if s.phase = .granted ∧ s.expires ≤ now then
    { s with phase := .revoked } else s
  if authorized s' session now then (some s'.binding, s') else (none, s')

theorem approve_iff (s : State) (o : Origin) (now : Nat) :
    (approve s o now).1 = true ↔
      o = .host ∧ s.phase = .pending ∧ now + lifetime ≤ maxTime := by
  by_cases h : o = .host ∧ s.phase = .pending ∧ now + lifetime ≤ maxTime <;>
    simp [approve, h]

theorem model_cannot_approve (s : State) (now : Nat) :
    approve s .model now = (false, s) := by simp [approve]

theorem model_cannot_revoke (s : State) : revoke s .model = (false, s) := by simp [revoke]

theorem authorized_iff (s : State) (session : List Nat) (now : Nat) :
    authorized s session now = true ↔
      s.phase = .granted ∧ s.session = session ∧ now < s.expires := by simp [authorized]

theorem no_foreign_session (s : State) (session : List Nat) (now : Nat)
    (h : s.session ≠ session) : authorized s session now = false := by simp [authorized, h]

theorem no_expired (s : State) (session : List Nat) (now : Nat)
    (h : s.expires ≤ now) : authorized s session now = false := by
  simp [authorized, Nat.not_lt.mpr h]

theorem no_revoked (s : State) (session : List Nat) (now : Nat)
    (h : s.phase = .revoked) : authorized s session now = false := by simp [authorized, h]

theorem dispatch_effect_iff (s : State) (session : List Nat) (now : Nat) (b : Binding) :
    (dispatch s session now).1 = some b ↔
      s.phase = .granted ∧ s.session = session ∧ now < s.expires ∧ s.binding = b := by
  cases hp : s.phase <;> by_cases hs : s.session = session <;>
    by_cases ht : now < s.expires <;>
    simp [dispatch, authorized, hp, hs, ht, show s.expires ≤ now ↔ ¬ now < s.expires from Nat.not_lt.symm]

theorem expired_dispatch_closes (s : State) (session : List Nat) (now : Nat)
    (hp : s.phase = .granted) (ht : s.expires ≤ now) :
    dispatch s session now = (none, { s with phase := .revoked }) := by
  simp [dispatch, authorized, hp, ht]

inductive Action where
  | approve : Origin → Nat → Action
  | revoke : Origin → Action
  | dispatch : List Nat → Nat → Action
  deriving Repr

structure Observation where
  accepted : Bool
  effect : Option Binding
  deriving DecidableEq, Repr

def step (s : State) : Action → Observation × State
  | .approve o t => let r := approve s o t; (⟨r.1, none⟩, r.2)
  | .revoke o => let r := revoke s o; (⟨r.1, none⟩, r.2)
  | .dispatch session t => let r := dispatch s session t; (⟨r.1.isSome, r.1⟩, r.2)

def run (s : State) : List Action → List Observation × State
  | [] => ([], s)
  | a :: rest => let r := step s a; let tail := run r.2 rest; (r.1 :: tail.1, tail.2)

/-- Neither administrative operations nor inference can alter a binding. -/
theorem step_preserves_binding (s : State) (a : Action) :
    (step s a).2.binding = s.binding := by
  cases a <;> simp only [step, approve, revoke, dispatch]
  all_goals repeat first | rfl | split

theorem run_preserves_binding (s : State) (as : List Action) :
    (run s as).2.binding = s.binding := by
  induction as generalizing s with
  | nil => rfl
  | cons a as ih => exact (ih (step s a).2).trans (step_preserves_binding s a)

theorem revoked_step (s : State) (h : s.phase = .revoked) (a : Action) :
    (step s a).2 = s ∧ (step s a).1.effect = none := by
  cases a <;> simp [step, approve, revoke, dispatch, authorized, h]
  case revoke o => cases o <;> simp [show ({ s with phase := .revoked } : State) = s by cases s; simp_all]

/-- Revocation is terminal even if the host clock later moves backwards. -/
theorem revoked_run (s : State) (h : s.phase = .revoked) (as : List Action) :
    (run s as).2 = s ∧ ∀ r ∈ (run s as).1, r.effect = none := by
  induction as generalizing s with
  | nil => simp [run]
  | cons a as ih =>
    have hs := revoked_step s h a
    simpa [run, hs.1, hs.2] using ih s h

/-- Every effect retains the originally reviewed transport and all parameters. -/
theorem step_effect_exact (s : State) (a : Action) (b : Binding)
    (h : (step s a).1.effect = some b) : b = s.binding := by
  cases a <;> simp only [step, dispatch] at h
  all_goals try simp_all
  all_goals split at h <;> simp_all
  all_goals split at h <;> simp_all

theorem run_effect_exact (s : State) (as : List Action) (b : Binding)
    (r : Observation) (hr : r ∈ (run s as).1) (he : r.effect = some b) :
    b = s.binding := by
  induction as generalizing s with
  | nil => simp [run] at hr
  | cons a as ih =>
    simp only [run, List.mem_cons] at hr
    rcases hr with hr | hr
    · subst r; exact step_effect_exact s a b he
    · exact (ih (step s a).2 hr).trans (step_preserves_binding s a)

end MedenSpec.Processor
