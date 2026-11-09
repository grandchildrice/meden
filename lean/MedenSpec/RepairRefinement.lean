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
def failure : FailureKind → Repair.Failure
  | .Unverified => .unverified | .Unavailable => .unavailable | .Inconclusive => .inconclusive
def observation : core.result.Result Bool FailureKind → Repair.Observation
  | .Ok verified => .ok verified | .Err kind => .error (failure kind)
def signal : Signal → Repair.Signal
  | .Start => .start
  | .Conversation inspection accepted changed proposalFailed =>
    .conversation inspection accepted changed proposalFailed
  | .Current result => .current (observation result) | .Checked result => .checked (observation result)

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
  Repair.advance Repair.stop Repair.repair failure observation Repair.operational Aeneas.Std.lift

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
    ∃ fresh proposalFailed, signal s = .conversation (some fresh) true true proposalFailed := by
  have hm := advance_refines t s cancelled f review
  simp only [h, bind_ok, Result.ok.injEq] at hm
  exact Repair.inspecting_requires_acceptance (state t) (signal s) cancelled (f.map failure) review
    (congrArg Prod.fst hm).symm

theorem budget_gets_fresh_check (t : Turn) (proposalFailed : Bool)
    (hs : t.stage = .Conversation) :
    (do let out ← Turn.advance t (.Conversation (some true) true true proposalFailed)
          false (some .Unverified) false
        let next ← Turn.advance out.2 (.Current (.Ok false)) false (some .Unverified) false
        ok next.1) = ok .Check := by
  simp [Turn.advance, hs, stops_refines]

theorem proposal_repair (t : Turn) (fresh changed : Bool)
    (f : Option FailureKind) (enabled : Repair.operational (f.map failure) = false)
    (hs : t.stage = .Conversation) (budget : 0 < t.remaining.val) :
    (do let out ← Turn.advance t (.Conversation (some fresh) false changed true) false f false
        ok (effect out.1, state out.2)) =
      ok (.repair, {state t with stage := .ready}) := by
  rw [advance_refines]
  simpa [hs, budget] using enabled

theorem draft_repair_requires_failure (t final : Turn)
    (inspection : Option Bool) (changed proposalFailed cancelled review : Bool)
    (f : Option FailureKind)
    (h : Turn.advance t (.Conversation inspection false changed proposalFailed)
      cancelled f review = ok (.Repair, final)) :
    proposalFailed = true ∧ inspection.isSome = true := by
  have hm := advance_refines t (.Conversation inspection false changed proposalFailed)
    cancelled f review
  simp only [h, bind_ok, Result.ok.injEq] at hm
  exact Repair.unaccepted_repair_requires_proposal_failure (state t) inspection changed
    proposalFailed cancelled review (f.map failure) (congrArg Prod.fst hm).symm

theorem inspection_error (t : Turn) (fresh : Bool)
    (hs : t.stage = .Inspect fresh) (kind : FailureKind) :
    (do let out ← Turn.advance t (.Current (.Err kind)) false (some .Unverified) false
        ok (effect out.1, state out.2)) =
      ok (match failure kind with
        | .unverified => (.check, {state t with stage := .checking})
        | .unavailable | .inconclusive => Repair.stop (state t) .failed) := by
  rw [advance_refines]
  cases kind <;> simp [hs]

end MedenSpec.RepairRefinement
