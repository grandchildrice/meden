import MedenSpec.ContractsRefinement
import MedenSpec.ContractsInvariant

/-! Arbitrary finite compositions of the production contract controller.
Typed origins are inputs here; physical host provenance, JSON decoding,
rendering, locking and input construction remain adapter obligations. -/
namespace MedenSpec.ContractsTrace
open Aeneas Aeneas.Std Result ByteVectors EvidenceRefinement CheckJobsRefinement ContractsRefinement
open MedenEvidenceImpl.evidence (Contracts Obligation EvidenceOrigin CheckTicket CheckResult)

inductive Action where
  | propose (origin : EvidenceOrigin) (contract : alloc.vec.Vec U8) (obligations : alloc.vec.Vec Obligation)
  | review (origin : EvidenceOrigin) (session : Slice U8) (revision : U64)
  | accept (origin : EvidenceOrigin) (session : Slice U8) (revision : U64)
  | reject (origin : EvidenceOrigin) (session : Slice U8) (revision : U64)
  | refresh (origin : EvidenceOrigin) (revision : U64) (inputs : alloc.vec.Vec U8)
  | start (origin : EvidenceOrigin) (revision : U64) (inputs : alloc.vec.Vec U8)
  | invalidate (origin : EvidenceOrigin)
  | edit (origin : EvidenceOrigin)
  | abort (origin : EvidenceOrigin) (ticket : CheckTicket)
  | complete (origin : EvidenceOrigin) (revision : U64) (ticket : CheckTicket)
      (inputs : alloc.vec.Vec U8) (result : CheckResult)

def event : Action → MedenSpec.Contracts.Action
  | .propose o c obs => .propose (origin o) (bytes c.val) (obs.val.map obligation)
  | .review o session rev => .review (origin o) (bytes session.val) rev.val
  | .accept o session rev => .accept (origin o) (bytes session.val) rev.val
  | .reject o session rev => .reject (origin o) (bytes session.val) rev.val
  | .refresh o rev inputs => .refresh (origin o) rev.val (bytes inputs.val)
  | .start o rev inputs => .start (origin o) rev.val (bytes inputs.val)
  | .invalidate o => .invalidate (origin o)
  | .edit o => .edit (origin o)
  | .abort o t => .abort (origin o) (ticket t)
  | .complete o rev t inputs r => .complete (origin o) rev.val (ticket t)
      (bytes inputs.val) (CompletionRefinement.output r)

private def next {α : Type} (operation : Result (α × Contracts)) : Result Contracts := do
  let out ← operation
  ok out.2

def one (s : Contracts) : Action → Result Contracts
  | .propose o c obs => next (MedenEvidenceImpl.evidence.Contracts.propose s o c obs)
  | .review o session rev => next (MedenEvidenceImpl.evidence.Contracts.review s o session rev)
  | .accept o session rev => next (MedenEvidenceImpl.evidence.Contracts.accept s o session rev)
  | .reject o session rev => next (MedenEvidenceImpl.evidence.Contracts.reject s o session rev)
  | .refresh o rev inputs => next (MedenEvidenceImpl.evidence.Contracts.refresh s o rev inputs)
  | .start o rev inputs => next (MedenEvidenceImpl.evidence.Contracts.start s o rev inputs)
  | .invalidate o => next (MedenEvidenceImpl.evidence.Contracts.invalidate s o)
  | .edit o => next (MedenEvidenceImpl.evidence.Contracts.edit s o)
  | .abort o t => next (MedenEvidenceImpl.evidence.Contracts.abort s o t)
  | .complete o rev t inputs r => next (MedenEvidenceImpl.evidence.Contracts.complete s o rev t inputs r)

private theorem next_refines {α β : Type} (operation : Result (α × Contracts))
    (observe : α → β) (expected : β × MedenSpec.Contracts.State)
    (h : (do let out ← operation; ok (observe out.1, state out.2)) = ok expected) :
    (do let out ← next operation; ok (state out)) = ok expected.2 := by
  cases hc : operation with
  | ret out =>
    simp only [hc, bind_ok, Result.ok.injEq] at h
    simpa [next, hc] using congrArg Prod.snd h
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

theorem one_refines (s : Contracts) (a : Action) :
    (do let out ← one s a; ok (state out)) =
      ok (MedenSpec.Contracts.step (state s) (event a)) := by
  cases a with
  | propose o c obs => exact next_refines _ id _ (ContractsRefinement.propose_refines s o c obs)
  | review o session rev => exact next_refines _ id _ (ContractsRefinement.review_refines s o session rev)
  | accept o session rev => exact next_refines _ id _ (ContractsRefinement.accept_refines s o session rev)
  | reject o session rev => exact next_refines _ id _ (ContractsRefinement.reject_refines s o session rev)
  | refresh o rev inputs => exact next_refines _ id _ (ContractsRefinement.refresh_refines s o rev inputs)
  | start o rev inputs => exact next_refines _ (Option.map ticket) _ (ContractsRefinement.start_refines s o rev inputs)
  | invalidate o => exact next_refines _ id _ (ContractsRefinement.invalidate_refines s o)
  | edit o => exact next_refines _ id _ (ContractsRefinement.edit_refines s o)
  | abort o t => exact next_refines _ id _ (ContractsRefinement.abort_refines s o t)
  | complete o rev t inputs r => exact next_refines _ id _ (ContractsRefinement.complete_refines s o rev t inputs r)

def runExtracted (s : Contracts) : List Action → Result Contracts
  | [] => ok s
  | a :: rest => do
    let out ← one s a
    runExtracted out rest

theorem trace_refines (s : Contracts) (actions : List Action) :
    (do let out ← runExtracted s actions; ok (state out)) =
      ok (MedenSpec.Contracts.run (state s) (actions.map event)) := by
  induction actions generalizing s with
  | nil => simp [runExtracted, MedenSpec.Contracts.run]
  | cons a rest ih =>
    have hm := one_refines s a
    cases hc : one s a with
    | ret out =>
      simp only [hc, bind_ok, Result.ok.injEq] at hm
      simp only [runExtracted, hc, bind_ok]
      rw [ih]
      simp [MedenSpec.Contracts.run, hm]
    | vis effect continuation => simp [hc] at hm
    | div => simp [hc] at hm

theorem extracted_sequence_monotone (initial final : Contracts) (actions : List Action)
    (h : runExtracted initial actions = ok final) : initial.sequence.val ≤ final.sequence.val := by
  have hm := trace_refines initial actions
  simp only [h, bind_ok, Result.ok.injEq] at hm
  have hb := MedenSpec.Contracts.run_sequence_monotone (state initial) (actions.map event)
  rw [← hm] at hb
  exact hb

theorem reviewed_acceptance (s final : Contracts) (o : EvidenceOrigin)
    (session : Slice U8) (revision : U64)
    (h : MedenEvidenceImpl.evidence.Contracts.accept s o session revision = ok (true, final)) :
    origin o = .host ∧ bytes session.val = (state s).session ∧
      ∃ d, (state s).draft = some d ∧ d.revision = revision.val ∧
        d.gate.phase = .reviewed ∧ d.gate.obligations ≠ [] ∧
        (state final).accepted = some {d with gate := {d.gate with phase := .accepted}} := by
  have hm := ContractsRefinement.accept_refines s o session revision
  simp only [h, bind_ok, Result.ok.injEq] at hm
  have accepted := MedenSpec.Contracts.acceptance_requires_review (state s) (origin o)
    (bytes session.val) revision.val (congrArg Prod.fst hm).symm
  rw [← hm] at accepted
  exact accepted

theorem extracted_meaning_preserved (initial final : Contracts) (actions : List Action)
    (h : ∀ a ∈ actions, MedenSpec.Contracts.hostAcceptance (event a) = false)
    (hr : runExtracted initial actions = ok final) :
    MedenSpec.Contracts.meaning (state final) = MedenSpec.Contracts.meaning (state initial) := by
  have hm := trace_refines initial actions
  simp only [hr, bind_ok, Result.ok.injEq] at hm
  rw [hm]
  apply MedenSpec.Contracts.run_preserves_meaning
  intro a ha
  obtain ⟨original, ho, he⟩ := List.mem_map.mp ha
  subst a
  exact h original ho

end MedenSpec.ContractsTrace
