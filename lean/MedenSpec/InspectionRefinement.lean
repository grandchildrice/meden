import MedenSpec.Inspection
import MedenSpec.ContractsTrace

namespace MedenSpec.InspectionRefinement
open Aeneas Aeneas.Std Result ByteVectors MedenEvidenceImpl
set_option maxHeartbeats 8000000

def snapshot (s : evidence.Inspection) : Inspection.Snapshot :=
  ⟨bytes s.session.val, s.sequence.val, s.accepted.map ContractsRefinement.view,
    s.displayed.map ContractsRefinement.view, s.ready⟩

private theorem mapped_success {α β : Type} (m : Result α) (f : α → β) (target : β)
    (h : (do let value ← m; ok (f value)) = ok target) :
    ∃ value, m = ok value ∧ f value = target := by
  cases hm : m with
  | ret value => exact ⟨value, rfl, by simpa [hm] using h⟩
  | vis effect continuation => simp [hm] at h
  | div => simp [hm] at h

private theorem same_view_refines (left right : Option evidence.ContractView) :
    evidence.same_view left right =
      ok (decide (left.map ContractsRefinement.view = right.map ContractsRefinement.view)) := by
  cases left <;> cases right <;> simp [evidence.same_view, ContractsRefinement.view_eq]

theorem capture_refines (s : evidence.Contracts) :
    (do let out ← evidence.Contracts.inspect s; ok (snapshot out)) =
      ok (Inspection.capture (ContractsRefinement.state s)) := by
  obtain ⟨accepted, ha, hma⟩ := mapped_success _ _ _ (ContractsRefinement.accepted_refines s)
  obtain ⟨displayed, hd, hmd⟩ := mapped_success _ _ _ (ContractsRefinement.displayed_refines s)
  simp [evidence.Contracts.inspect, vec_clone, ha, hd, ContractsRefinement.is_accepted_refines,
    snapshot, Inspection.capture, ContractsRefinement.state, hma, hmd]

theorem accepted_refines (s : evidence.Inspection) :
    (do let out ← evidence.Inspection.impl.accepted s; ok (out.map ContractsRefinement.view)) =
      ok (snapshot s).accepted := by
  cases h : s.accepted <;> simp [evidence.Inspection.impl.accepted, snapshot, h]

theorem matches_refines (s : evidence.Inspection) (current : evidence.Contracts) :
    evidence.Inspection.matches s current =
      ok (decide (snapshot s = Inspection.capture (ContractsRefinement.state current))) := by
  obtain ⟨accepted, ha, hma⟩ := mapped_success _ _ _ (ContractsRefinement.accepted_refines current)
  obtain ⟨displayed, hd, hmd⟩ := mapped_success _ _ _ (ContractsRefinement.displayed_refines current)
  simp only [evidence.Inspection.matches, vec_eq, ← bytes_eq,
    ContractsRefinement.is_accepted_refines, ha, hd, same_view_refines, bind_ok]
  simp [snapshot, Inspection.capture, ContractsRefinement.state, Inspection.Snapshot.mk.injEq,
    hma, hmd]
  split_ifs <;> simp_all

theorem consume_refines (s : evidence.Inspection) (current : evidence.Contracts) :
    (do let out ← evidence.Inspection.consume s current
        ok (out.map (Option.map ContractsRefinement.view))) =
      ok (Inspection.consume (snapshot s) (ContractsRefinement.state current)) := by
  simp only [evidence.Inspection.consume, matches_refines, bind_ok]
  by_cases same : snapshot s = Inspection.capture (ContractsRefinement.state current)
  · simp [Inspection.consume, same]
    exact congrArg Inspection.Snapshot.accepted same
  · simp [Inspection.consume, same]

theorem extracted_original (s : evidence.Inspection) (current : evidence.Contracts)
    (view : Option evidence.ContractView)
    (h : evidence.Inspection.consume s current = ok (some view)) :
    view.map ContractsRefinement.view = (snapshot s).accepted ∧
      snapshot s = Inspection.capture (ContractsRefinement.state current) := by
  have hr := consume_refines s current
  simp only [h, bind_ok, Option.map_some, Result.ok.injEq] at hr
  have hs := (Inspection.consume_iff _ _ _).mp hr.symm
  exact ⟨hs.2, hs.1⟩

theorem extracted_stale (s : evidence.Inspection) (current : evidence.Contracts)
    (changed : snapshot s ≠ Inspection.capture (ContractsRefinement.state current)) :
    evidence.Inspection.consume s current = ok none := by
  simp [evidence.Inspection.consume, matches_refines, changed]

theorem extracted_generation (s : evidence.Inspection) (current : evidence.Contracts)
    (advanced : s.sequence.val < current.sequence.val) :
    evidence.Inspection.consume s current = ok none := by
  apply extracted_stale
  intro same
  have h := congrArg Inspection.Snapshot.sequence same
  simp only [snapshot, Inspection.capture, ContractsRefinement.state] at h
  omega

theorem extracted_stays_stale (initial advanced final : evidence.Contracts)
    (frozen : evidence.Inspection) (actions : List ContractsTrace.Action)
    (hi : evidence.Contracts.inspect initial = ok frozen)
    (changed : initial.sequence.val < advanced.sequence.val)
    (hr : ContractsTrace.runExtracted advanced actions = ok final) :
    evidence.Inspection.consume frozen final = ok none := by
  have hf := capture_refines initial
  simp only [hi, bind_ok, Result.ok.injEq] at hf
  have hs := congrArg Inspection.Snapshot.sequence hf
  simp only [snapshot, Inspection.capture, ContractsRefinement.state] at hs
  have hm := ContractsTrace.extracted_sequence_monotone advanced final actions hr
  apply extracted_generation
  omega

theorem extracted_empty (s : evidence.Inspection) (current : evidence.Contracts)
    (view : Option evidence.ContractView) (empty : s.accepted = none)
    (h : evidence.Inspection.consume s current = ok (some view)) : view = none := by
  have original := (extracted_original s current view h).1
  simp [snapshot, empty] at original
  exact original

end MedenSpec.InspectionRefinement
