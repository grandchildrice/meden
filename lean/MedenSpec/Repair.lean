module
public import Lean.Elab.Tactic.Omega
public import Mathlib.Tactic.SplitIfs
@[expose] public section

/-! The outer turn protocol. Signals are host observations, not decoded model
commands. The model outcome carries no proof/evidence authority. -/
namespace MedenSpec.Repair

inductive Terminal where
  | finish | cancelled | failed | review | exhausted
  deriving DecidableEq

inductive Effect where
  | model | inspect | check | repair | stop (reason : Terminal)
  deriving DecidableEq

inductive Outcome where
  | done | failed | budget | cancelled | review
  deriving DecidableEq

inductive Failure where
  | unverified | unavailable | inconclusive
  deriving DecidableEq

inductive Signal where
  | start
  | conversation (outcome : Outcome) (accepted changed : Bool)
  | current (verified : Bool)
  | checked (verified : Bool)
  deriving DecidableEq

inductive Stage where
  | ready | conversation | inspect (fresh : Bool) | checking | stopped (reason : Terminal)
  deriving DecidableEq

structure State where
  remaining : Nat
  stage : Stage
  deriving DecidableEq

def initial : State := ⟨4, .ready⟩

def operational : Option Failure → Bool
  | some .unavailable | some .inconclusive => true
  | none | some .unverified => false

def stop (s : State) (reason : Terminal) : Effect × State :=
  (.stop reason, {s with stage := .stopped reason})

def repair (s : State) : Effect × State :=
  if 0 < s.remaining then (.repair, {s with stage := .ready}) else stop s .exhausted

def advance (s : State) (event : Signal) (cancelled : Bool)
    (failure : Option Failure) (review : Bool) : Effect × State :=
  match s.stage with
  | .stopped reason => (.stop reason, s)
  | _ =>
    if cancelled then stop s .cancelled
    else if operational failure then stop s .failed
    else if review then stop s .review
    else match s.stage, event with
      | .ready, .start =>
        if 0 < s.remaining then (.model, ⟨s.remaining - 1, .conversation⟩)
        else stop s .exhausted
      | .conversation, .conversation outcome accepted changed =>
        if accepted = true ∧ changed = true ∧ (outcome = .done ∨ outcome = .budget)
        then (.inspect, {s with stage := .inspect (decide (outcome = .budget))})
        else stop s .finish
      | .inspect fresh, .current verified =>
        if verified then stop s .finish
        else if fresh = false ∧ failure = some .unverified then repair s
        else (.check, {s with stage := .checking})
      | .checking, .checked verified =>
        if verified then stop s .finish
        else if failure = some .unverified then repair s else stop s .failed
      | _, _ => stop s .exhausted

structure Input where
  signal : Signal
  cancelled : Bool
  failure : Option Failure
  review : Bool

def step (s : State) (i : Input) : Effect × State :=
  advance s i.signal i.cancelled i.failure i.review

def conversations : Effect → Nat
  | .model => 1 | _ => 0

def run (s : State) : List Input → State × Nat
  | [] => (s, 0)
  | i :: rest =>
    let next := step s i
    let out := run next.2 rest
    (out.1, conversations next.1 + out.2)

theorem step_budget (s : State) (i : Input) :
    (step s i).2.remaining + conversations (step s i).1 ≤ s.remaining := by
  rcases s with ⟨n, stage⟩
  rcases i with ⟨event, cancelled, failure, review⟩
  cases stage <;> cases event <;>
    simp only [step, advance] <;>
    (try split_ifs) <;> simp_all [stop, repair, conversations]
  all_goals first | omega | (split_ifs <;> simp_all)

theorem inspecting_requires_acceptance (s : State) (event : Signal) (cancelled : Bool)
    (failure : Option Failure) (review : Bool)
    (h : (advance s event cancelled failure review).1 = .inspect) :
    ∃ outcome, event = .conversation outcome true true ∧ (outcome = .done ∨ outcome = .budget) := by
  cases hs : s.stage <;> cases event <;>
    simp only [advance, hs, stop, repair] at h
  all_goals try split_ifs at h
  all_goals simp_all

theorem trace_budget (s : State) (inputs : List Input) :
    (run s inputs).1.remaining + (run s inputs).2 ≤ s.remaining := by
  induction inputs generalizing s with
  | nil => simp [run]
  | cons i rest ih =>
    have hs := step_budget s i
    have hr := ih (step s i).2
    dsimp only [run]
    omega

theorem trace_stopped (n : Nat) (reason : Terminal) (inputs : List Input) :
    run ⟨n, .stopped reason⟩ inputs = (⟨n, .stopped reason⟩, 0) := by
  induction inputs with
  | nil => rfl
  | cons i rest ih => simp [run, step, advance, conversations, ih]

end MedenSpec.Repair
