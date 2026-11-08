module
public import Lean.Elab.Tactic.Omega
public import Mathlib.Tactic.SplitIfs
@[expose] public section

namespace MedenSpec.Conversation

structure Call where
  id : String
  tool : String
  arguments : String
  deriving DecidableEq

structure Reply where
  content : Option String
  calls : List Call
  deriving DecidableEq

inductive Status where
  | done (text : String) | failed (text : String) | incomplete | cancelled | review
  deriving DecidableEq

def inspection : Status → Option Bool
  | .done _ => some false | .incomplete => some true | _ => none

inductive Check where
  | ok (text : String) | refused (text : String) | tool (text : String) | stopped (status : Status)
  deriving DecidableEq

inductive Phase where
  | ready | announced | waiting | publishingAssistant | executing | publishingResult | halted
  deriving DecidableEq

structure State where
  remaining : Nat
  attempt : Nat
  phase : Phase
  reply : Reply
  next : Nat
  result : Option Check
  stopped : Option Status
  faulted : Bool
  deriving DecidableEq

inductive Input where
  | start | observed | reply (value : Reply) | modelFailed (status : Status) | toolResult (value : Check)

inductive Effect where
  | asking (attempt : Nat) | model | assistant | tool (execute : Bool) | record | stop
  deriving DecidableEq

inductive Outcome where
  | finish (content : Option String) | stopped (status : Status) | fault
  deriving DecidableEq

def initial (maximum groups : Nat) : State :=
  ⟨min (groups + 1) maximum, 0, .ready, ⟨none, []⟩, 0, none, none, false⟩

def blocked (s : State) : Bool := s.faulted || s.stopped.isSome

def latch (s : State) (status : Option Status) : State :=
  if s.faulted = false ∧ s.stopped = none then {s with stopped := status} else s

def halt (s : State) : Effect × State := (.stop, {s with phase := .halted})

def announce (maximum : Nat) (s : State) : Effect × State :=
  if blocked s ∨ s.remaining = 0 then halt s
  else
    let attempt := min (s.attempt + 1) maximum
    (.asking attempt, {s with attempt, phase := .announced})

def nextCall (maximum : Nat) (s : State) : Effect × State :=
  if s.next < s.reply.calls.length then
    (.tool (!blocked s), {s with phase := .executing})
  else if blocked s ∨ s.reply.calls.isEmpty then halt s
  else announce maximum s

def recorded (maximum : Nat) (s : State) : Effect × State :=
  nextCall maximum {s with next := min (s.next + 1) maximum, result := none}

def record (s : State) (result : Check) : Effect × State :=
  let (s, stored) : State × Check := match result with
    | .stopped status => (latch s (some status), .stopped .incomplete)
    | value => (s, value)
  if s.next < s.reply.calls.length then
    (.record, {s with result := some stored, phase := .publishingResult})
  else halt {s with faulted := true}

def reject (maximum : Nat) (s : State) : Effect × State :=
  let s := {s with faulted := true}
  match s.phase with
  | .executing => record s (.stopped .incomplete)
  | .publishingAssistant => nextCall maximum s
  | .publishingResult => recorded maximum s
  | _ => halt s

def dispatch (maximum : Nat) (s : State) (input : Input) : Effect × State :=
    match s.phase, input with
    | .ready, .start => announce maximum s
    | .announced, .observed =>
      if blocked s ∨ s.remaining = 0 then halt s
      else (.model, {s with remaining := s.remaining - 1, phase := .waiting})
    | .waiting, .reply reply =>
      let s := {s with reply, next := 0}
      let s := if s.remaining = 0 ∧ reply.calls.isEmpty = false then latch s (some .incomplete) else s
      (.assistant, {s with phase := .publishingAssistant})
    | .waiting, .modelFailed status => halt (latch s (some status))
    | .publishingAssistant, .observed => nextCall maximum s
    | .publishingResult, .observed => recorded maximum s
    | .executing, .toolResult result => record s result
    | _, _ => reject maximum s

def advance (maximum : Nat) (s : State) (input : Input)
    (cancelled : Bool) (hostStop : Option Status) : Effect × State :=
  if s.phase = .halted then (.stop, s)
  else dispatch maximum (latch s (if cancelled then some .cancelled else hostStop)) input

def pending (s : State) : Option Call :=
  match s.phase with
  | .executing | .publishingResult => s.reply.calls[s.next]?
  | _ => none

def result (s : State) : Option Check :=
  if s.phase = .publishingResult then s.result else none

def outcome (s : State) : Outcome :=
  match s.stopped with
  | some status => .stopped status
  | none => if s.faulted then .fault else .finish s.reply.content

def models : Effect → Nat
  | .model => 1 | _ => 0

structure Observation where
  input : Input
  cancelled : Bool
  stop : Option Status

def step (maximum : Nat) (s : State) (i : Observation) : Effect × State :=
  advance maximum s i.input i.cancelled i.stop

def run (maximum : Nat) (s : State) : List Observation → State × Nat
  | [] => (s, 0)
  | i :: rest =>
    let next := step maximum s i
    let out := run maximum next.2 rest
    (out.1, models next.1 + out.2)

end MedenSpec.Conversation
