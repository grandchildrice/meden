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

inductive Failure where
  | unverified | unavailable | inconclusive
  deriving DecidableEq

inductive Observation where
  | ok (verified : Bool)
  | error (kind : Failure)
  deriving DecidableEq

inductive Signal where
  | start
  | conversation (inspection : Option Bool) (accepted changed proposalFailed : Bool)
  | current (result : Observation)
  | checked (result : Observation)
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
      | .conversation, .conversation inspection accepted changed proposalFailed =>
        if accepted = true ∧ changed = true then
          match inspection with
          | some fresh => (.inspect, {s with stage := .inspect fresh})
          | none => stop s .finish
        else if accepted = false ∧ proposalFailed = true ∧ inspection.isSome then repair s
        else stop s .finish
      | .inspect fresh, .current result =>
        match result with
        | .ok true => stop s .finish
        | .ok false =>
          if fresh = false ∧ failure = some .unverified then repair s
          else (.check, {s with stage := .checking})
        | .error .unverified => (.check, {s with stage := .checking})
        | .error .unavailable | .error .inconclusive => stop s .failed
      | .checking, .checked result =>
        match result with
        | .ok true => stop s .finish
        | .error .unverified => repair s
        | .ok false | .error .unavailable | .error .inconclusive => stop s .failed
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
    simp only [step, advance]
  all_goals repeat' (first | simp_all [stop, repair, conversations] | split)
  all_goals omega

theorem inspecting_requires_acceptance (s : State) (event : Signal) (cancelled : Bool)
    (failure : Option Failure) (review : Bool)
    (h : (advance s event cancelled failure review).1 = .inspect) :
    ∃ fresh proposalFailed, event = .conversation (some fresh) true true proposalFailed := by
  cases hs : s.stage <;> cases event <;>
    simp only [advance, hs, stop, repair] at h
  all_goals repeat' (first | simp_all | split at h)

theorem unaccepted_repair_requires_proposal_failure (s : State)
    (inspection : Option Bool) (changed proposalFailed cancelled review : Bool)
    (failure : Option Failure)
    (h : (advance s (.conversation inspection false changed proposalFailed)
      cancelled failure review).1 = .repair) :
    proposalFailed = true ∧ inspection.isSome = true := by
  cases hs : s.stage <;>
    simp only [advance, hs, stop, repair] at h
  all_goals repeat' (first | simp_all | split at h)

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
