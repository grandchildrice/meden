import MedenSpec.CheckJobs
import MedenSpec.EvidenceRefinement

namespace MedenSpec.CheckJobsRefinement
set_option maxHeartbeats 8000000
open Aeneas Aeneas.Std Result ByteVectors EvidenceRefinement
open MedenImpl.evidence (CheckJobs CheckTicket Gate Subject EvidenceOrigin)

def ticket (t : CheckTicket) : MedenSpec.CheckJobs.Ticket := ⟨t.id.val, subject t.subject⟩
def jobs (j : CheckJobs) : MedenSpec.CheckJobs.State :=
  ⟨bytes j.session.val, j.sequence.val, j.active.map ticket⟩

private theorem subject_clone (s : Subject) :
    MedenImpl.evidence.Subject.Insts.CoreCloneClone.clone s = ok s := by
  simp [MedenImpl.evidence.Subject.Insts.CoreCloneClone.clone, vec_clone]

private theorem ticket_clone (t : CheckTicket) :
    MedenImpl.evidence.CheckTicket.Insts.CoreCloneClone.clone t = ok t := by
  simp [MedenImpl.evidence.CheckTicket.Insts.CoreCloneClone.clone, subject_clone,
    core.clone.impls.CloneU64.clone, Aeneas.Std.lift]

private theorem ticket_eq (x y : CheckTicket) :
    MedenImpl.evidence.CheckTicket.Insts.CoreCmpPartialEqCheckTicket.eq x y =
      ok (decide (ticket x = ticket y)) := by
  unfold MedenImpl.evidence.CheckTicket.Insts.CoreCmpPartialEqCheckTicket.eq
  simp only [subject_eq]
  by_cases h : x.id.val = y.id.val <;>
    simp [← UScalar.eq_equiv, h, ticket, MedenSpec.CheckJobs.Ticket.mk.injEq]

private theorem matches_refines (j : CheckJobs) (t : CheckTicket) :
    MedenImpl.evidence.CheckJobs.matches j t = ok (decide ((jobs j).active = some (ticket t))) := by
  cases h : j.active <;> simp [MedenImpl.evidence.CheckJobs.matches, h, ticket_eq, jobs]

theorem new_refines (scope : alloc.vec.Vec U8) :
    (do let j ← MedenImpl.evidence.CheckJobs.new scope; ok (jobs j)) =
      ok (MedenSpec.CheckJobs.initial (bytes scope.val)) := by
  simp [MedenImpl.evidence.CheckJobs.new, jobs, MedenSpec.CheckJobs.initial]

theorem start_refines (j : CheckJobs) (o : EvidenceOrigin) (g : Gate) (s : Subject) :
    (do let out ← MedenImpl.evidence.CheckJobs.start j o g s;
        ok (out.1.map ticket, jobs out.2)) =
      ok (MedenSpec.CheckJobs.start (jobs j) (origin o) (gate g) (subject s)) := by
  have bound := U64.checked_add_bv_spec j.sequence 1#u64
  cases hc : U64.checked_add j.sequence 1#u64 with
  | none =>
    have hb : ¬ j.sequence.val < 18446744073709551615 := by
      simp [hc, U64.max_eq] at bound
      omega
    cases o <;> cases hp : g.phase <;>
      by_cases h1 : j.session.val = s.session.val <;>
      by_cases h2 : g.subject.session.val = s.session.val <;>
      by_cases h3 : g.subject.contract.val = s.contract.val <;>
      simp [MedenImpl.evidence.CheckJobs.start, core.cmp.PartialEq.ne.trait_default,
        core.cmp.PartialEq.ne.default, origin_eq, is_accepted_refines, vec_ne,
        hc, Aeneas.Std.lift, hp, h1, h2, h3, hb, subject_clone, ticket_clone,
        jobs, gate, subject, phase, origin, bytes_eq,
        MedenSpec.CheckJobs.start, MedenSpec.CheckJobs.permitted]
  | some n =>
    simp only [hc] at bound
    have hn : n.val = j.sequence.val + 1 := by simpa using bound.2.1
    have hb : j.sequence.val < 18446744073709551615 := by
      have h := bound
      simp [U64.max_eq] at h
      omega
    cases o <;> cases hp : g.phase <;>
      by_cases h1 : j.session.val = s.session.val <;>
      by_cases h2 : g.subject.session.val = s.session.val <;>
      by_cases h3 : g.subject.contract.val = s.contract.val <;>
      simp [MedenImpl.evidence.CheckJobs.start, core.cmp.PartialEq.ne.trait_default,
        core.cmp.PartialEq.ne.default, origin_eq, is_accepted_refines, vec_ne,
        hc, Aeneas.Std.lift, hp, h1, h2, h3, hb, hn, subject_clone, ticket_clone,
        jobs, ticket, gate, subject, phase, origin, bytes_eq,
        MedenSpec.CheckJobs.start, MedenSpec.CheckJobs.permitted]

theorem invalidate_refines (j : CheckJobs) (o : EvidenceOrigin) :
    (do let out ← MedenImpl.evidence.CheckJobs.invalidate j o; ok (out.1, jobs out.2)) =
      ok (MedenSpec.CheckJobs.invalidate (jobs j) (origin o)) := by
  cases o <;> simp [MedenImpl.evidence.CheckJobs.invalidate, core.cmp.PartialEq.ne.trait_default,
    core.cmp.PartialEq.ne.default, origin_eq, jobs, origin, MedenSpec.CheckJobs.invalidate]

theorem abort_refines (j : CheckJobs) (o : EvidenceOrigin) (t : CheckTicket) :
    (do let out ← MedenImpl.evidence.CheckJobs.abort j o t; ok (out.1, jobs out.2)) =
      ok (MedenSpec.CheckJobs.abort (jobs j) (origin o) (ticket t)) := by
  unfold MedenImpl.evidence.CheckJobs.abort
  rw [matches_refines]
  by_cases h : (jobs j).active = some (ticket t) <;>
    simp [h, invalidate_refines, MedenSpec.CheckJobs.abort]

theorem finish_refines (j : CheckJobs) (o : EvidenceOrigin) (g : Gate) (t : CheckTicket) (s : Subject) :
    (do let out ← MedenImpl.evidence.CheckJobs.finish j o g t s; ok (out.1, jobs out.2)) =
      ok (MedenSpec.CheckJobs.finish (jobs j) (origin o) (gate g) (ticket t) (subject s)) := by
  cases o <;> cases hp : g.phase <;>
    by_cases ha : (jobs j).active = some (ticket t) <;>
    by_cases ht : subject t.subject = subject s <;>
    by_cases h1 : j.session.val = s.session.val <;>
    by_cases h2 : g.subject.session.val = s.session.val <;>
    by_cases h3 : g.subject.contract.val = s.contract.val <;>
    simp [MedenImpl.evidence.CheckJobs.finish, core.cmp.PartialEq.ne.trait_default,
      core.cmp.PartialEq.ne.default, origin_eq, is_accepted_refines, matches_refines,
      subject_eq, vec_ne, hp, h1, h2, h3, jobs, ticket, gate, subject,
      phase, origin, bytes_eq, MedenSpec.CheckJobs.finish, MedenSpec.CheckJobs.permitted]
  all_goals split_ifs <;> simp_all [jobs, ticket, subject, bytes_eq]
  all_goals aesop

theorem is_active_refines (j : CheckJobs) :
    MedenImpl.evidence.CheckJobs.is_active j = ok (jobs j).active.isSome := by
  cases h : j.active <;> simp [MedenImpl.evidence.CheckJobs.is_active, h, jobs,
    core.option.Option.is_some]

end MedenSpec.CheckJobsRefinement
