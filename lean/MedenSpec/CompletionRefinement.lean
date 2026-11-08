import MedenSpec.Completion
import MedenSpec.CheckJobsRefinement

namespace MedenSpec.CompletionRefinement
open Aeneas Aeneas.Std Result EvidenceRefinement CheckJobsRefinement
open MedenImpl.evidence (CheckJobs CheckTicket CheckResult Gate Subject EvidenceOrigin)

def output (r : CheckResult) : Completion.Output :=
  ⟨subject r.subject, r.position.val, obligation r.obligation, r.passed⟩

theorem complete_refines (j : CheckJobs) (g : Gate) (o : EvidenceOrigin)
    (t : CheckTicket) (s : Subject) (r : CheckResult) :
    (do let out ← MedenImpl.evidence.CheckJobs.complete j g o t s r
        ok (out.1, jobs out.2.1, gate out.2.2)) =
      ok (Completion.complete (jobs j) (gate g) (origin o) (ticket t) (subject s) (output r)) := by
  have hf := finish_refines j o g t s
  cases hfinish : MedenImpl.evidence.CheckJobs.finish j o g t s with
  | ret finished =>
    rcases finished with ⟨active, next⟩
    simp only [hfinish, bind_ok, Result.ok.injEq] at hf
    cases ha : active with
    | false =>
      simp [MedenImpl.evidence.CheckJobs.complete, hfinish, ha,
        Completion.complete, ← hf]
    | true =>
      have hr := refresh_refines g .Controller r.subject
      cases hrefresh : MedenImpl.evidence.Gate.refresh g .Controller r.subject with
      | ret refreshed =>
        rcases refreshed with ⟨refreshed, nextGate⟩
        simp only [hrefresh, bind_ok, Result.ok.injEq] at hr
        change (refreshed, gate nextGate) =
          Evidence.refresh (gate g) .controller (subject r.subject) at hr
        cases hb : refreshed with
        | false =>
          simp [MedenImpl.evidence.CheckJobs.complete, hfinish, ha, subject_clone,
            hrefresh, hb, Completion.complete, ← hf, ← hr, output]
        | true =>
          let native := MedenImpl.evidence.Receipt.mk r.subject r.position r.obligation r.passed true
          have hm := record_refines nextGate o r.subject native
          cases hrecord : MedenImpl.evidence.Gate.record nextGate o r.subject native with
          | ret recorded =>
            rcases recorded with ⟨recorded, finalGate⟩
            simp only [hrecord, bind_ok, Result.ok.injEq] at hm
            change (recorded, gate finalGate) = Evidence.record (gate nextGate)
              (origin o) (subject r.subject) (Completion.receipt (output r)) at hm
            have hbmap := congrArg Prod.fst hm
            have hgmap := congrArg Prod.snd hm
            simp only [output] at hbmap hgmap
            simp [MedenImpl.evidence.CheckJobs.complete, hfinish, ha, subject_clone,
              hrefresh, hb, hrecord, native, Completion.complete, ← hf, ← hr, output,
              ← hbmap, ← hgmap]
          | vis effect continuation => simp [hrecord] at hm
          | div => simp [hrecord] at hm
      | vis effect continuation => simp [hrefresh] at hr
      | div => simp [hrefresh] at hr
  | vis effect continuation => simp [hfinish] at hf
  | div => simp [hfinish] at hf

theorem success_requires_current_job (j final : CheckJobs) (g finalGate : Gate)
    (o : EvidenceOrigin) (t : CheckTicket) (s : Subject) (r : CheckResult)
    (h : MedenImpl.evidence.CheckJobs.complete j g o t s r = ok (true, final, finalGate)) :
    origin o = .checker ∧ (jobs j).active = some (ticket t) ∧
    subject t.subject = subject s ∧ r.passed = true ∧
    (jobs final).active = none ∧ subject finalGate.subject = subject r.subject := by
  have hm := complete_refines j g o t s r
  simp only [h, bind_ok, Result.ok.injEq] at hm
  have hs := Completion.success_corresponds (jobs j) (gate g) (origin o)
    (ticket t) (subject s) (output r) (congrArg Prod.fst hm).symm
  rw [← hm] at hs
  exact hs

theorem model_cannot_complete (j : CheckJobs) (g : Gate)
    (t : CheckTicket) (s : Subject) (r : CheckResult) :
    MedenImpl.evidence.CheckJobs.complete j g .Model t s r = ok (false, j, g) := by
  simp [MedenImpl.evidence.CheckJobs.complete, MedenImpl.evidence.CheckJobs.finish,
    core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
    EvidenceOrigin.Insts.CoreCmpPartialEqEvidenceOrigin.eq, EvidenceOrigin.read_discriminant]

theorem complete_jobs_refines (j : CheckJobs) (g : Gate) (o : EvidenceOrigin)
    (t : CheckTicket) (s : Subject) (r : CheckResult) :
    (do let out ← MedenImpl.evidence.CheckJobs.complete j g o t s r
        ok (jobs out.2.1)) =
      ok (MedenSpec.CheckJobs.finish (jobs j) (origin o) (gate g) (ticket t) (subject s)).2 := by
  have hm := complete_refines j g o t s r
  cases hc : MedenImpl.evidence.CheckJobs.complete j g o t s r with
  | ret out =>
    simp only [hc, bind_ok, Result.ok.injEq] at hm
    simp only [bind_ok]
    have hj := congrArg (fun x : Bool × MedenSpec.CheckJobs.State × Evidence.Gate => x.2.1) hm
    dsimp only at hj
    rw [hj]
    simp only [Completion.complete]
    split_ifs <;> rfl
  | vis effect continuation => simp [hc] at hm
  | div => simp [hc] at hm

end MedenSpec.CompletionRefinement
