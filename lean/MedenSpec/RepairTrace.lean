import MedenSpec.RepairRefinement

/-! Finite compositions of the actual outer-turn controller. One Model effect
invokes the ordinary conversation loop, which may make multiple LLM requests.
The interpreter, history, flags and physical I/O are separate obligations. -/
namespace MedenSpec.RepairTrace
open Aeneas Aeneas.Std Result MedenRepairImpl RepairRefinement
open verification.turn
open verification.failure (FailureKind)

structure Input where
  signal : Signal
  cancelled : Bool
  failure : Option FailureKind
  review : Bool

def input (i : Input) : Repair.Input :=
  ⟨RepairRefinement.signal i.signal, i.cancelled, i.failure.map RepairRefinement.failure, i.review⟩

def runExtracted (t : Turn) : List Input → Result (Turn × Nat)
  | [] => ok (t, 0)
  | i :: rest => do
    let out ← Turn.advance t i.signal i.cancelled i.failure i.review
    let final ← runExtracted out.2 rest
    ok (final.1, Repair.conversations (effect out.1) + final.2)

private theorem call_success (t : Turn) (i : Input) :
    ∃ out, Turn.advance t i.signal i.cancelled i.failure i.review = ok out ∧
      (effect out.1, state out.2) = Repair.step (state t) (input i) := by
  have hm := advance_refines t i.signal i.cancelled i.failure i.review
  cases hc : Turn.advance t i.signal i.cancelled i.failure i.review with
  | ret out => exact ⟨out, by simp_all, by simpa [hc, input, Repair.step] using hm⟩
  | vis effect continuation => simp [hc] at hm
  | div => simp [hc] at hm

theorem trace_refines (t : Turn) (inputs : List Input) :
    (do let out ← runExtracted t inputs; ok (state out.1, out.2)) =
      ok (Repair.run (state t) (inputs.map input)) := by
  induction inputs generalizing t with
  | nil => simp [runExtracted, Repair.run]
  | cons i rest ih =>
    obtain ⟨out, hc, hm⟩ := call_success t i
    have he := congrArg Prod.fst hm
    have hs := congrArg Prod.snd hm
    dsimp only at he hs
    simp only [runExtracted, hc, bind_ok]
    have hr := ih out.2
    cases hn : runExtracted out.2 rest with
    | ret final =>
      simp only [hn, bind_ok, Result.ok.injEq] at hr
      simp [Repair.run, ← hr, ← hs, ← he]
    | vis effect continuation => simp [hn] at hr
    | div => simp [hn] at hr

theorem extracted_conversation_budget (initial final : Turn) (count : Nat) (inputs : List Input)
    (hi : Turn.begin = ok initial)
    (hr : runExtracted initial inputs = ok (final, count)) : count ≤ 4 := by
  have hn := begin_refines
  simp only [hi, bind_ok, Result.ok.injEq] at hn
  have ht := trace_refines initial inputs
  simp only [hr, bind_ok, Result.ok.injEq] at ht
  have hb := Repair.trace_budget (state initial) (inputs.map input)
  rw [← ht, hn] at hb
  simp only [Repair.initial] at hb
  omega

theorem extracted_stopped (t : Turn) (reason : Terminal) (inputs : List Input)
    (hs : t.stage = .Stopped reason) : runExtracted t inputs = ok (t, 0) := by
  induction inputs with
  | nil => rfl
  | cons i rest ih => simp [runExtracted, Turn.advance, hs, ih, effect, Repair.conversations]

end MedenSpec.RepairTrace
