import MedenSpec.CompletionRefinement

/-! Finite compositions of the actual issued-job operations. The I/O loop,
prepared-input construction, OS cancellation and checker attribution remain
adapter obligations, rather than premises supplied by model text. -/
namespace MedenSpec.CheckJobsTrace
open Aeneas Aeneas.Std Result CheckJobsRefinement EvidenceRefinement CompletionRefinement
open MedenEvidenceImpl.evidence (CheckJobs CheckTicket CheckResult EvidenceOrigin Gate Subject)

inductive Action where
  | start (o : EvidenceOrigin) (g : Gate) (s : Subject)
  | invalidate (o : EvidenceOrigin)
  | abort (o : EvidenceOrigin) (t : CheckTicket)
  | finish (o : EvidenceOrigin) (g : Gate) (t : CheckTicket) (s : Subject)
  | complete (o : EvidenceOrigin) (g : Gate) (t : CheckTicket) (s : Subject) (r : CheckResult)

def event : Action → MedenSpec.CheckJobs.Action
  | .start o g s => .start (origin o) (gate g) (subject s)
  | .invalidate o => .invalidate (origin o)
  | .abort o t => .abort (origin o) (ticket t)
  | .finish o g t s => .finish (origin o) (gate g) (ticket t) (subject s)
  | .complete o g t s _ => .finish (origin o) (gate g) (ticket t) (subject s)

def one (j : CheckJobs) (history : List MedenSpec.CheckJobs.Ticket) :
    Action → Result (CheckJobs × List MedenSpec.CheckJobs.Ticket)
  | .start o g s => do
    let out ← MedenEvidenceImpl.evidence.CheckJobs.start j o g s
    ok (out.2, match out.1 with | none => history | some t => ticket t :: history)
  | .invalidate o => do
    let out ← MedenEvidenceImpl.evidence.CheckJobs.invalidate j o
    ok (out.2, history)
  | .abort o t => do
    let out ← MedenEvidenceImpl.evidence.CheckJobs.abort j o t
    ok (out.2, history)
  | .finish o g t s => do
    let out ← MedenEvidenceImpl.evidence.CheckJobs.finish j o g t s
    ok (out.2, history)
  | .complete o g t s r => do
    let out ← MedenEvidenceImpl.evidence.CheckJobs.complete j g o t s r
    ok (out.2.1, history)

private theorem one_refines (j : CheckJobs) (history : List MedenSpec.CheckJobs.Ticket) (a : Action) :
    (do let out ← one j history a; ok (jobs out.1, out.2)) =
      ok (MedenSpec.CheckJobs.step (jobs j) (event a), MedenSpec.CheckJobs.log (jobs j) history (event a)) := by
  cases a with
  | start o g s =>
    have hm := start_refines j o g s
    cases hc : MedenEvidenceImpl.evidence.CheckJobs.start j o g s with
    | ret out =>
      simp only [hc, bind_ok, Result.ok.injEq] at hm
      have hg := congrArg Prod.snd hm
      have ht := congrArg Prod.fst hm
      cases hv : out.1 <;>
        simp_all [one, event, MedenSpec.CheckJobs.step, MedenSpec.CheckJobs.log]
      all_goals rw [← ht]
    | vis effect continuation => simp [hc] at hm
    | div => simp [hc] at hm
  | invalidate o =>
    have hm := invalidate_refines j o
    cases hc : MedenEvidenceImpl.evidence.CheckJobs.invalidate j o with
    | ret out =>
      simp only [hc, bind_ok, Result.ok.injEq] at hm
      have hg := congrArg Prod.snd hm
      simp_all [one, event, MedenSpec.CheckJobs.step, MedenSpec.CheckJobs.log]
    | vis effect continuation => simp [hc] at hm
    | div => simp [hc] at hm
  | abort o t =>
    have hm := abort_refines j o t
    cases hc : MedenEvidenceImpl.evidence.CheckJobs.abort j o t with
    | ret out =>
      simp only [hc, bind_ok, Result.ok.injEq] at hm
      have hg := congrArg Prod.snd hm
      simp_all [one, event, MedenSpec.CheckJobs.step, MedenSpec.CheckJobs.log]
    | vis effect continuation => simp [hc] at hm
    | div => simp [hc] at hm
  | finish o g t s =>
    have hm := finish_refines j o g t s
    cases hc : MedenEvidenceImpl.evidence.CheckJobs.finish j o g t s with
    | ret out =>
      simp only [hc, bind_ok, Result.ok.injEq] at hm
      have hg := congrArg Prod.snd hm
      simp_all [one, event, MedenSpec.CheckJobs.step, MedenSpec.CheckJobs.log]
    | vis effect continuation => simp [hc] at hm
    | div => simp [hc] at hm

  | complete o g t s r =>
    have hm := complete_jobs_refines j g o t s r
    cases hc : MedenEvidenceImpl.evidence.CheckJobs.complete j g o t s r with
    | ret out =>
      simp only [hc, bind_ok, Result.ok.injEq] at hm
      simp_all [one, event, MedenSpec.CheckJobs.step, MedenSpec.CheckJobs.log]
    | vis effect continuation => simp [hc] at hm
    | div => simp [hc] at hm

private theorem one_success (j : CheckJobs) (history : List MedenSpec.CheckJobs.Ticket) (a : Action) :
    ∃ out, one j history a = ok out ∧ (jobs out.1, out.2) =
      (MedenSpec.CheckJobs.step (jobs j) (event a), MedenSpec.CheckJobs.log (jobs j) history (event a)) := by
  have hm := one_refines j history a
  cases hc : one j history a with
  | ret out => exact ⟨out, by simp_all, by simpa [hc] using hm⟩
  | vis effect continuation => simp [hc] at hm
  | div => simp [hc] at hm

def runExtracted (j : CheckJobs) (history : List MedenSpec.CheckJobs.Ticket) :
    List Action → Result (CheckJobs × List MedenSpec.CheckJobs.Ticket)
  | [] => ok (j, history)
  | a :: rest => do
    let out ← one j history a
    runExtracted out.1 out.2 rest

theorem trace_refines (j : CheckJobs) (history : List MedenSpec.CheckJobs.Ticket) (actions : List Action) :
    (do let out ← runExtracted j history actions; ok (jobs out.1, out.2)) =
      ok (MedenSpec.CheckJobs.run (jobs j) history (actions.map event)) := by
  induction actions generalizing j history with
  | nil => simp [runExtracted, MedenSpec.CheckJobs.run]
  | cons a rest ih =>
    obtain ⟨out, hc, hm⟩ := one_success j history a
    have hj := congrArg Prod.fst hm
    have hh := congrArg Prod.snd hm
    dsimp only at hj hh
    simp only [runExtracted, hc, bind_ok]
    rw [ih]
    simp [MedenSpec.CheckJobs.run, hj, hh]

/-- After a job is retired, every older generation stays unusable across any
finite sequence of later jobs, including identical restored source/tool bytes. -/
theorem extracted_no_late_result (initial final finished : CheckJobs)
    (history : List MedenSpec.CheckJobs.Ticket) (actions : List Action)
    (t : CheckTicket) (o : EvidenceOrigin) (g : Gate) (s : Subject) (accepted : Bool)
    (hi : initial.active = none) (ht : t.id.val ≤ initial.sequence.val)
    (hr : runExtracted initial [] actions = ok (final, history))
    (hf : MedenEvidenceImpl.evidence.CheckJobs.finish final o g t s = ok (accepted, finished)) :
    accepted = false := by
  have htrace := trace_refines initial [] actions
  simp only [hr, bind_ok, Result.ok.injEq] at htrace
  have h := MedenSpec.CheckJobs.no_late_result (jobs initial) [] (actions.map event)
    (by simp [jobs, hi]) (ticket t) ht (origin o) (gate g) (subject s)
  rw [← htrace] at h
  have hfinish := finish_refines final o g t s
  simp only [hf, bind_ok, Result.ok.injEq] at hfinish
  exact (congrArg Prod.fst hfinish).trans h

/-- An accepted completion names a ticket in the log of actually issued jobs. -/
theorem extracted_result_was_issued (scope : alloc.vec.Vec U8) (initial final finished : CheckJobs)
    (history : List MedenSpec.CheckJobs.Ticket) (actions : List Action)
    (t : CheckTicket) (o : EvidenceOrigin) (g : Gate) (s : Subject)
    (hi : MedenEvidenceImpl.evidence.CheckJobs.new scope = ok initial)
    (hr : runExtracted initial [] actions = ok (final, history))
    (hf : MedenEvidenceImpl.evidence.CheckJobs.finish final o g t s = ok (true, finished)) :
    ticket t ∈ history := by
  have hnew := new_refines scope
  simp only [hi, bind_ok, Result.ok.injEq] at hnew
  have htrace := trace_refines initial [] actions
  simp only [hr, bind_ok, Result.ok.injEq] at htrace
  have issued := MedenSpec.CheckJobs.trace_issued (jobs initial) [] (actions.map event)
    (by simp [hnew, MedenSpec.CheckJobs.initial, MedenSpec.CheckJobs.Issued])
  rw [← htrace] at issued
  have hfinish := finish_refines final o g t s
  simp only [hf, bind_ok, Result.ok.injEq] at hfinish
  have accepted : (MedenSpec.CheckJobs.finish (jobs final) (origin o) (gate g) (ticket t) (subject s)).1 = true :=
    (congrArg Prod.fst hfinish).symm
  unfold MedenSpec.CheckJobs.finish at accepted
  split at accepted
  · rename_i hm
    exact issued (ticket t) hm.2.2.1
  · simp at accepted

theorem extracted_completion_was_issued (scope : alloc.vec.Vec U8)
    (initial final after : CheckJobs) (g afterGate : Gate)
    (history : List MedenSpec.CheckJobs.Ticket) (actions : List Action)
    (t : CheckTicket) (o : EvidenceOrigin) (s : Subject) (r : CheckResult)
    (hi : MedenEvidenceImpl.evidence.CheckJobs.new scope = ok initial)
    (hr : runExtracted initial [] actions = ok (final, history))
    (hc : MedenEvidenceImpl.evidence.CheckJobs.complete final g o t s r = ok (true, after, afterGate)) :
    ticket t ∈ history := by
  have hm := complete_refines final g o t s r
  simp only [hc, bind_ok, Result.ok.injEq] at hm
  have hs := Completion.success_requires_job (jobs final) (gate g) (origin o)
    (ticket t) (subject s) (output r) (congrArg Prod.fst hm).symm
  have hf := finish_refines final o g t s
  cases hn : MedenEvidenceImpl.evidence.CheckJobs.finish final o g t s with
  | ret out =>
    rcases out with ⟨accepted, finished⟩
    simp only [hn, bind_ok, Result.ok.injEq] at hf
    have hb : accepted = true := (congrArg Prod.fst hf).trans hs
    exact extracted_result_was_issued scope initial final finished history actions t o g s hi hr
      (by simpa [hb] using hn)
  | vis effect continuation => simp [hn] at hf
  | div => simp [hn] at hf

end MedenSpec.CheckJobsTrace
