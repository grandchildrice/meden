import MedenSpec.Repair
import MedenSpec.Generated.Repair

namespace MedenSpec.RepairRefinement
set_option maxHeartbeats 4000000
open Aeneas Aeneas.Std Result MedenRepairImpl
open verification.turn
open verification.failure (FailureKind)

def terminal : Terminal → Repair.Terminal
  | .Finish => .finish | .Cancelled => .cancelled | .Failed => .failed
  | .Review => .review | .Exhausted => .exhausted
def effect : Effect → Repair.Effect
  | .Model => .model | .Inspect => .inspect | .Check => .check | .Repair => .repair
  | .Stop r => .stop (terminal r)
def stage : Stage → Repair.Stage
  | .Ready => .ready | .Conversation => .conversation | .Inspect fresh => .inspect fresh
  | .Checking => .checking | .Stopped r => .stopped (terminal r)
def state (t : Turn) : Repair.State := ⟨t.remaining.val, stage t.stage⟩
def outcome : meden_agent.conversation.Status → Repair.Outcome
  | .Done _ => .done | .Failed _ => .failed | .Incomplete => .budget
  | .Cancelled => .cancelled | .NeedsReview => .review
def signal : Signal → Repair.Signal
  | .Start => .start | .Conversation status accepted changed => .conversation (outcome status) accepted changed
  | .Current verified => .current verified | .Checked verified => .checked verified
def failure : FailureKind → Repair.Failure
  | .Unverified => .unverified | .Unavailable => .unavailable | .Inconclusive => .inconclusive

private theorem stops_refines (f : Option FailureKind) :
    verification.failure.verification_stops f = ok (Repair.operational (f.map failure)) := by
  cases f with
  | none => rfl
  | some f => cases f <;> simp [verification.failure.verification_stops,
      FailureKind.repairable, FailureKind.Insts.CoreCmpPartialEqFailureKind.eq,
      FailureKind.read_discriminant, failure, Repair.operational]

private theorem zero_eq (v : U8) : v = 0#u8 ↔ v.val = 0 := by
  simpa using UScalar.eq_equiv v 0#u8

attribute [local simp] Turn.stop Turn.repair zero_eq effect state stage signal terminal
  Repair.advance Repair.stop Repair.repair outcome failure Repair.operational Aeneas.Std.lift

theorem begin_refines :
    (do let out ← Turn.begin; ok (state out)) = ok Repair.initial := by
  simp [Turn.begin, state, stage, Repair.initial]

theorem advance_refines (t : Turn) (s : Signal) (cancelled : Bool)
    (f : Option FailureKind) (review : Bool) :
    (do let out ← Turn.advance t s cancelled f review; ok (effect out.1, state out.2)) =
      ok (Repair.advance (state t) (signal s) cancelled (f.map failure) review) := by
  have bound := U8.checked_sub_bv_spec t.remaining 1#u8
  cases hc : U8.checked_sub t.remaining 1#u8 with
  | none =>
    have hz : t.remaining.val = 0 := by simp [hc] at bound; omega
    cases hs : t.stage <;> cases s <;>
      simp only [Turn.advance, hs, stops_refines]
    all_goals repeat' (first
      | simp_all
      | split)
  | some n =>
    simp only [hc] at bound
    have hp : 0 < t.remaining.val := by have h := bound.1; simp at h; omega
    have hn : n.val = t.remaining.val - 1 := by simpa using bound.2.1
    cases hs : t.stage <;> cases s <;>
      simp only [Turn.advance, hs, stops_refines]
    all_goals repeat' (first
      | simp_all
      | split)

theorem inspection_requires_acceptance (t final : Turn) (s : Signal) (cancelled : Bool)
    (f : Option FailureKind) (review : Bool)
    (h : Turn.advance t s cancelled f review = ok (.Inspect, final)) :
    ∃ status, signal s = .conversation status true true ∧ (status = .done ∨ status = .budget) := by
  have hm := advance_refines t s cancelled f review
  simp only [h, bind_ok, Result.ok.injEq] at hm
  exact Repair.inspecting_requires_acceptance (state t) (signal s) cancelled (f.map failure) review
    (congrArg Prod.fst hm).symm

theorem budget_gets_fresh_check (t : Turn) (hs : t.stage = .Conversation) :
    (do let out ← Turn.advance t (.Conversation .Incomplete true true) false (some .Unverified) false
        let next ← Turn.advance out.2 (.Current false) false (some .Unverified) false
        ok next.1) = ok .Check := by
  simp [Turn.advance, hs, stops_refines]

end MedenSpec.RepairRefinement
