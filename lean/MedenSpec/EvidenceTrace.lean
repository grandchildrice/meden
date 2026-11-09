import MedenSpec.EvidenceRefinement

/-! Finite compositions of the actual extracted Rust transitions. This driver
is a Lean composition, not an extraction of the release I/O loop or GitHub API. -/
namespace MedenSpec.EvidenceTrace
open Aeneas Aeneas.Std Result EvidenceRefinement
open MedenEvidenceImpl.evidence (EvidenceOrigin Subject Receipt Gate Obligation)

inductive Action where
  | review (origin : Origin)
  | accept (origin : Origin)
  | refresh (origin : Origin) (subject : Subject)
  | record (origin : Origin) (subject : Subject) (receipt : Receipt)

def event : Action → Evidence.Event
  | .review o => .review (origin o)
  | .accept o => .accept (origin o)
  | .refresh o s => .refresh (origin o) (subject s)
  | .record o s r => .record (origin o) (subject s) (receipt r)

def call (g : Gate) : Action → Result (Bool × Gate)
  | .review o => MedenEvidenceImpl.evidence.Gate.review g o
  | .accept o => MedenEvidenceImpl.evidence.Gate.accept g o
  | .refresh o s => MedenEvidenceImpl.evidence.Gate.refresh g o s
  | .record o s r => MedenEvidenceImpl.evidence.Gate.record g o s r

private def specCall (g : Evidence.Gate) : Action → Bool × Evidence.Gate
  | .review o => Evidence.review g (origin o)
  | .accept o => Evidence.accept g (origin o)
  | .refresh o s => Evidence.refresh g (origin o) (subject s)
  | .record o s r => Evidence.record g (origin o) (subject s) (receipt r)

private theorem call_refines (g : Gate) (a : Action) :
    (do let out ← call g a; ok (out.1, gate out.2)) = ok (specCall (gate g) a) := by
  cases a with
  | review o => exact review_refines g o
  | accept o => exact accept_refines g o
  | refresh o s => exact refresh_refines g o s
  | record o s r => exact record_refines g o s r

private theorem call_success (g : Gate) (a : Action) :
    ∃ out, call g a = ok out ∧ (out.1, gate out.2) = specCall (gate g) a := by
  have hm := call_refines g a
  cases hr : call g a with
  | ret out => exact ⟨out, by simp_all, by simpa [hr] using hm⟩
  | vis effect continuation => simp [hr] at hm
  | div => simp [hr] at hm

private def log (history : List Evidence.Receipt) (a : Action) (accepted : Bool) : List Evidence.Receipt :=
  match a with
  | .record _ _ r => if accepted then receipt r :: history else history
  | _ => history

def runExtracted (g : Gate) (history : List Evidence.Receipt) : List Action → Result (Gate × List Evidence.Receipt)
  | [] => ok (g, history)
  | action :: rest => do
    let out ← call g action
    runExtracted out.2 (log history action out.1) rest

theorem trace_refines (g : Gate) (history : List Evidence.Receipt) (actions : List Action) :
    (do let out ← runExtracted g history actions; ok (gate out.1, out.2)) =
      ok (Evidence.run (gate g) history (actions.map event)) := by
  induction actions generalizing g history with
  | nil => simp [runExtracted, Evidence.run]
  | cons action rest ih =>
    obtain ⟨out, hc, hm⟩ := call_success g action
    have hb := congrArg Prod.fst hm
    have hg := congrArg Prod.snd hm
    dsimp only at hb hg
    simp only [runExtracted, hc, bind_ok]
    rw [ih]
    cases action <;> simp_all [Evidence.run, Evidence.step, specCall, event, log]

/-- If an extracted composition is eligible, its log contains current matching
successful evidence for every obligation. The checker/OS attribution remains
a separate trust boundary; this theorem does not execute backend checkers. -/
theorem extracted_requires_all_evidence (s : Subject) (requirements : alloc.vec.Vec Obligation)
    (actions : List Action) (initial final : Gate) (history : List Evidence.Receipt) (current : Subject)
    (hi : MedenEvidenceImpl.evidence.Gate.propose s requirements = ok initial)
    (hr : runExtracted initial [] actions = ok (final, history))
    (he : MedenEvidenceImpl.evidence.Gate.eligible final current = ok true) :
    ∀ i, i < final.obligations.val.length → ∃ r ∈ history,
      r.subject = subject current ∧ r.position = i ∧ r.passed = true ∧ r.active = true ∧
        (final.obligations.val.map obligation)[i]? = some r.obligation := by
  have hp := propose_refines s requirements
  have ht := trace_refines initial [] actions
  have hel := eligible_refines final current
  simp only [hi, bind_ok, Result.ok.injEq] at hp
  simp only [hr, bind_ok, Result.ok.injEq] at ht
  have eligible : Evidence.eligible (gate final) (subject current) = true := by
    rw [he] at hel
    exact (Result.ok_injective hel).symm
  rw [hp] at ht
  have hs : Evidence.run (Evidence.propose (subject s) (requirements.val.map obligation)) [] (actions.map event) = (gate final, history) := ht.symm
  have goal := Evidence.eligible_requires_all_evidence (subject s) (subject current)
    (requirements.val.map obligation) (actions.map event) (by simpa [hs] using eligible)
  simpa [hs, gate] using goal

end MedenSpec.EvidenceTrace
