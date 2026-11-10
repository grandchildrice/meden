import MedenSpec.Contracts
import MedenSpec.CompletionRefinement

namespace MedenSpec.ContractsRefinement
set_option maxHeartbeats 8000000
open Aeneas Aeneas.Std Result ByteVectors EvidenceRefinement CheckJobsRefinement
open MedenEvidenceImpl.evidence (ContractView VersionedGate Contracts Gate Obligation Subject EvidenceOrigin)

def versioned (v : VersionedGate) : MedenSpec.Contracts.Versioned := ⟨v.revision.val, gate v.gate⟩
def view (v : ContractView) : MedenSpec.Contracts.View :=
  ⟨v.revision.val, subject v.subject, v.obligations.val.map obligation⟩
def state (s : Contracts) : MedenSpec.Contracts.State :=
  ⟨bytes s.session.val, s.sequence.val, s.accepted.map versioned, s.draft.map versioned, jobs s.jobs⟩

private theorem bytes_empty (xs : List U8) : bytes xs = [] ↔ xs = [] := by simp [bytes]
private theorem bytes_nil : bytes [] = [] := rfl

private theorem obligation_clone (o : Obligation) :
    MedenEvidenceImpl.evidence.Obligation.Insts.CoreCloneClone.clone o = ok o := by
  simp [MedenEvidenceImpl.evidence.Obligation.Insts.CoreCloneClone.clone, vec_clone,
    MedenEvidenceImpl.evidence.EvidenceKind.Insts.CoreCloneClone.clone]

private theorem obligations_clone (v : alloc.vec.Vec Obligation) :
    alloc.vec.CloneVec.clone MedenEvidenceImpl.evidence.Obligation.Insts.CoreCloneClone v = ok v := by
  have h := Slice.clone_spec (s := v.slice)
    (clone := MedenEvidenceImpl.evidence.Obligation.Insts.CoreCloneClone.clone) (by intros; apply obligation_clone)
  unfold alloc.vec.CloneVec.clone
  cases hc : Slice.clone MedenEvidenceImpl.evidence.Obligation.Insts.CoreCloneClone.clone v.slice with
  | ret out =>
    have he : v.slice = out := by simpa [hc] using h
    simp [← he]
  | vis effect continuation => simp [hc] at h
  | div => simp [hc] at h

private theorem mapped_allM {α β : Type} [DecidableEq β] (f : α → β)
    (xs ys : List α) (hlen : xs.length = ys.length) :
    List.allM (fun p : α × α => ok (decide (f p.1 = f p.2))) (List.zip xs ys) =
      ok (decide (xs.map f = ys.map f)) := by
  induction xs generalizing ys with
  | nil => cases ys <;> simp_all [pure]
  | cons x xs ih =>
    cases ys with
    | nil => simp_all
    | cons y ys =>
      have hl : xs.length = ys.length := by simpa using hlen
      by_cases h : f x = f y <;> simp [h, ih ys hl, pure]

private theorem obligations_eq (x y : alloc.vec.Vec Obligation) :
    alloc.vec.partial_eq.PartialEqVec.eq MedenEvidenceImpl.evidence.Obligation.Insts.CoreCmpPartialEqObligation x y =
      ok (decide (x.val.map obligation = y.val.map obligation)) := by
  unfold alloc.vec.partial_eq.PartialEqVec.eq
  simp only [obligation_eq]
  by_cases h : x.val.length = y.val.length
  · simp only [h, ↓reduceIte]
    exact mapped_allM obligation x.val y.val h
  · have hn : x.val.map obligation ≠ y.val.map obligation := by
      intro he
      have := congrArg List.length he
      simp only [List.length_map] at this
      exact h this
    simp [h, hn]

theorem view_eq (a b : ContractView) :
    MedenEvidenceImpl.evidence.ContractView.Insts.CoreCmpPartialEqContractView.eq a b =
      ok (decide (view a = view b)) := by
  simp [MedenEvidenceImpl.evidence.ContractView.Insts.CoreCmpPartialEqContractView.eq,
    subject_eq, obligations_eq, view, ← UScalar.eq_equiv]
  split_ifs <;> simp_all

private theorem gate_clone (g : Gate) :
    MedenEvidenceImpl.evidence.Gate.Insts.CoreCloneClone.clone g = ok g := by
  simp [MedenEvidenceImpl.evidence.Gate.Insts.CoreCloneClone.clone, subject_clone, obligations_clone,
    MedenEvidenceImpl.evidence.EvidencePhase.Insts.CoreCloneClone.clone,
    core.clone.impls.CloneUsize.clone, Aeneas.Std.lift]

theorem new_refines (session : alloc.vec.Vec U8) :
    (do let s ← MedenEvidenceImpl.evidence.Contracts.new session; ok (state s)) =
      ok (MedenSpec.Contracts.initial (bytes session.val)) := by
  simp [MedenEvidenceImpl.evidence.Contracts.new, vec_clone, MedenEvidenceImpl.evidence.CheckJobs.new,
    state, jobs, MedenSpec.Contracts.initial, MedenSpec.CheckJobs.initial]

theorem accepted_refines (s : Contracts) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.impl.accepted s; ok (out.map view)) =
      ok (MedenSpec.Contracts.accepted (state s)) := by
  cases h : s.accepted <;>
    simp [MedenEvidenceImpl.evidence.Contracts.impl.accepted, h, MedenEvidenceImpl.evidence.VersionedGate.view,
      subject_clone, obligations_clone, view, versioned, state, gate, MedenSpec.Contracts.accepted,
      MedenSpec.Contracts.view]

theorem displayed_refines (s : Contracts) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.displayed s; ok (out.map view)) =
      ok (MedenSpec.Contracts.displayed (state s)) := by
  cases hd : s.draft <;> cases ha : s.accepted <;>
    simp [MedenEvidenceImpl.evidence.Contracts.displayed, MedenEvidenceImpl.evidence.Contracts.impl.accepted,
      hd, ha, MedenEvidenceImpl.evidence.VersionedGate.view, subject_clone, obligations_clone,
      view, versioned, state, gate, MedenSpec.Contracts.displayed, MedenSpec.Contracts.selected,
      MedenSpec.Contracts.view]

theorem pending_refines (s : Contracts) :
    MedenEvidenceImpl.evidence.Contracts.pending s = ok (MedenSpec.Contracts.pending (state s)) := by
  cases h : s.draft <;> simp [MedenEvidenceImpl.evidence.Contracts.pending, h, state, MedenSpec.Contracts.pending]

theorem is_accepted_refines (s : Contracts) :
    MedenEvidenceImpl.evidence.Contracts.is_accepted s = ok (MedenSpec.Contracts.isAccepted (state s)) := by
  cases hd : s.draft <;> cases ha : s.accepted <;>
    simp [MedenEvidenceImpl.evidence.Contracts.is_accepted, hd, ha, EvidenceRefinement.is_accepted_refines,
      state, versioned, MedenSpec.Contracts.isAccepted]
  rename_i a
  cases hp : a.gate.phase <;> simp [gate, phase, hp]

private theorem session_ne (a : alloc.vec.Vec U8) (b : Slice U8) :
    core.cmp.impls.PartialEqShared.ne (MedenEvidenceImpl.Slice.Insts.CoreCmpPartialEqSlice
      core.cmp.PartialEqU8) (alloc.vec.Vec.deref a) b =
      ok (decide (bytes a.val ≠ bytes b.val)) := by
  simp [core.cmp.impls.PartialEqShared.ne, core.cmp.PartialEq.ne.trait_default,
    core.cmp.PartialEq.ne.default, MedenEvidenceImpl.Slice.Insts.CoreCmpPartialEqSlice,
    slice_eq, alloc.vec.Vec.deref, bytes_eq]

theorem review_refines (s : Contracts) (o : EvidenceOrigin) (session : Slice U8) (rev : U64) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.review s o session rev; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.review (state s) (origin o) (bytes session.val) rev.val) := by
  unfold MedenEvidenceImpl.evidence.Contracts.review
  dsimp only
  rw [session_ne]
  simp only [bind_ok]
  by_cases hs : bytes s.session.val = bytes session.val
  · cases hd : s.draft with
    | none =>
      cases ha : s.accepted with
      | none => simp [MedenEvidenceImpl.evidence.Contracts.review, session_ne, hs, hd, ha,
          MedenSpec.Contracts.review, state]
      | some a =>
        cases o <;> cases hp : a.gate.phase <;> by_cases hr : a.revision.val = rev.val <;>
          simp [MedenEvidenceImpl.evidence.Contracts.review, session_ne, hs, hd, ha, hp,
            UScalar.eq_equiv, hr, MedenEvidenceImpl.evidence.Gate.review,
            core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
            origin_eq, MedenEvidenceImpl.evidence.EvidencePhase.Insts.CoreCmpPartialEqEvidencePhase.eq,
            MedenEvidenceImpl.evidence.EvidencePhase.read_discriminant,
            MedenSpec.Contracts.review, MedenSpec.Contracts.reviewOne, Evidence.review,
            state, versioned, gate, phase, origin]
    | some d =>
      cases o <;> cases hp : d.gate.phase <;> by_cases hr : d.revision.val = rev.val <;>
        simp [MedenEvidenceImpl.evidence.Contracts.review, session_ne, hs, hd, hp,
          UScalar.eq_equiv, hr, MedenEvidenceImpl.evidence.Gate.review,
          core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
          origin_eq, MedenEvidenceImpl.evidence.EvidencePhase.Insts.CoreCmpPartialEqEvidencePhase.eq,
          MedenEvidenceImpl.evidence.EvidencePhase.read_discriminant,
          MedenSpec.Contracts.review, MedenSpec.Contracts.reviewOne, Evidence.review,
          state, versioned, gate, phase, origin]
  · simp [MedenEvidenceImpl.evidence.Contracts.review, session_ne, hs, MedenSpec.Contracts.review, state]

theorem reject_refines (s : Contracts) (o : EvidenceOrigin) (session : Slice U8) (rev : U64) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.reject s o session rev; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.reject (state s) (origin o) (bytes session.val) rev.val) := by
  unfold MedenEvidenceImpl.evidence.Contracts.reject
  dsimp only
  rw [session_ne]
  cases o <;> cases hd : s.draft <;>
    simp [MedenEvidenceImpl.evidence.Contracts.reject, session_ne, hd,
      core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq,
      MedenSpec.Contracts.reject, state, origin, versioned, UScalar.eq_equiv]
  all_goals split_ifs <;> simp_all [state, versioned]

theorem invalidate_refines (s : Contracts) (o : EvidenceOrigin) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.invalidate s o; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.invalidate (state s) (origin o)) := by
  cases o <;> simp [MedenEvidenceImpl.evidence.Contracts.invalidate,
    MedenEvidenceImpl.evidence.CheckJobs.invalidate, core.cmp.PartialEq.ne.trait_default,
    core.cmp.PartialEq.ne.default, origin_eq, MedenSpec.Contracts.invalidate,
    MedenSpec.CheckJobs.invalidate, state, jobs, origin]

theorem edit_refines (s : Contracts) (o : EvidenceOrigin) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.edit s o; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.edit (state s) (origin o)) := by
  cases o <;> cases ha : s.accepted <;>
    simp [MedenEvidenceImpl.evidence.Contracts.edit, MedenEvidenceImpl.evidence.CheckJobs.invalidate,
      core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq,
      MedenSpec.Contracts.edit, MedenSpec.CheckJobs.invalidate, ha,
      state, jobs, origin, versioned, gate]

theorem abort_refines (s : Contracts) (o : EvidenceOrigin) (t : MedenEvidenceImpl.evidence.CheckTicket) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.abort s o t; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.abort (state s) (origin o) (ticket t)) := by
  have hm := CheckJobsRefinement.abort_refines s.jobs o t
  cases h : MedenEvidenceImpl.evidence.CheckJobs.abort s.jobs o t with
  | ret out =>
    rcases out with ⟨allowed, next⟩
    simp only [h, bind_ok, Result.ok.injEq] at hm
    simp [MedenEvidenceImpl.evidence.Contracts.abort, h, MedenSpec.Contracts.abort, state, ← hm]
  | vis effect continuation => simp [h] at hm
  | div => simp [h] at hm

theorem is_active_refines (s : Contracts) :
    MedenEvidenceImpl.evidence.Contracts.is_active s = ok (state s).jobs.active.isSome := by
  exact CheckJobsRefinement.is_active_refines s.jobs

theorem refresh_refines (s : Contracts) (o : EvidenceOrigin) (rev : U64) (inputs : alloc.vec.Vec U8) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.refresh s o rev inputs; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.refresh (state s) (origin o) rev.val (bytes inputs.val)) := by
  cases ha : s.accepted with
  | none => simp [MedenEvidenceImpl.evidence.Contracts.refresh, ha, MedenSpec.Contracts.refresh, state]
  | some a =>
    by_cases hr : a.revision.val = rev.val
    · have hm := EvidenceRefinement.refresh_refines a.gate o {a.gate.subject with inputs}
      cases h : MedenEvidenceImpl.evidence.Gate.refresh a.gate o {a.gate.subject with inputs} with
      | ret out =>
        rcases out with ⟨allowed, next⟩
        simp only [h, bind_ok, Result.ok.injEq] at hm
        change (allowed, gate next) = Evidence.refresh (gate a.gate) (origin o)
          (MedenSpec.Contracts.withInputs (gate a.gate) (bytes inputs.val)) at hm
        have hn : a.revision = rev := UScalar.eq_of_val_eq hr
        simp only [MedenEvidenceImpl.evidence.Contracts.refresh, ha]
        simp [hn, subject_clone, h, MedenSpec.Contracts.refresh, state, ha, versioned, hr, ← hm]
      | vis effect continuation => simp [h] at hm
      | div => simp [h] at hm
    · simp [MedenEvidenceImpl.evidence.Contracts.refresh, ha, ← UScalar.eq_equiv, hr,
        MedenSpec.Contracts.refresh, state, versioned]

theorem eligible_refines (s : Contracts) (rev : U64) (inputs : alloc.vec.Vec U8) :
    MedenEvidenceImpl.evidence.Contracts.eligible s rev inputs =
      ok (MedenSpec.Contracts.eligible (state s) rev.val (bytes inputs.val)) := by
  cases hd : s.draft <;> cases hj : s.jobs.active <;> cases ha : s.accepted <;>
    simp [MedenEvidenceImpl.evidence.Contracts.eligible, core.option.Option.is_some, subject_clone,
      CheckJobsRefinement.is_active_refines, hd, hj, ha, jobs, state, versioned,
      MedenSpec.Contracts.eligible, ← UScalar.eq_equiv, EvidenceRefinement.eligible_refines,
      MedenSpec.Contracts.withInputs, gate, subject]
  -- A faulty implementation may remove a branch; keep its remaining equality.
  all_goals try split_ifs <;> simp_all

theorem accept_refines (s : Contracts) (o : EvidenceOrigin) (session : Slice U8) (rev : U64) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.accept s o session rev; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.accept (state s) (origin o) (bytes session.val) rev.val) := by
  unfold MedenEvidenceImpl.evidence.Contracts.accept
  dsimp only
  rw [session_ne]
  simp only [bind_ok]
  by_cases hs : bytes s.session.val = bytes session.val
  · cases hd : s.draft with
    | none => simp [hs, hd, MedenSpec.Contracts.accept, state]
    | some d =>
      cases o <;> cases hp : d.gate.phase <;>
        by_cases hr : d.revision.val = rev.val <;> by_cases he : d.gate.obligations.val = [] <;>
        simp [hs, hd, hp, hr, he, UScalar.eq_equiv, gate_clone,
          MedenEvidenceImpl.evidence.Gate.accept, alloc.vec.Vec.is_empty,
          core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
          origin_eq, MedenEvidenceImpl.evidence.EvidencePhase.Insts.CoreCmpPartialEqEvidencePhase.eq,
          MedenEvidenceImpl.evidence.EvidencePhase.read_discriminant, MedenEvidenceImpl.evidence.CheckJobs.invalidate,
          MedenSpec.Contracts.accept, Evidence.accept, MedenSpec.CheckJobs.invalidate,
          state, versioned, gate, jobs, phase, origin]
  · simp [hs, MedenSpec.Contracts.accept, state]

theorem complete_refines (s : Contracts) (o : EvidenceOrigin) (rev : U64)
    (t : MedenEvidenceImpl.evidence.CheckTicket) (inputs : alloc.vec.Vec U8)
    (r : MedenEvidenceImpl.evidence.CheckResult) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.complete s o rev t inputs r; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.complete (state s) (origin o) rev.val (ticket t)
        (bytes inputs.val) (CompletionRefinement.output r)) := by
  cases hd : s.draft with
  | some d => simp [MedenEvidenceImpl.evidence.Contracts.complete, hd, core.option.Option.is_some,
      MedenSpec.Contracts.complete, state]
  | none =>
    cases ha : s.accepted with
    | none => simp [MedenEvidenceImpl.evidence.Contracts.complete, hd, ha, core.option.Option.is_some,
        MedenSpec.Contracts.complete, state]
    | some a =>
      by_cases hr : a.revision = rev
      · have hm := CompletionRefinement.complete_refines s.jobs a.gate o t
          {a.gate.subject with inputs} r
        cases h : MedenEvidenceImpl.evidence.CheckJobs.complete s.jobs a.gate o t {a.gate.subject with inputs} r with
        | ret out =>
          rcases out with ⟨allowed, nextJobs, nextGate⟩
          simp only [h, bind_ok, Result.ok.injEq] at hm
          change (allowed, jobs nextJobs, gate nextGate) = Completion.complete (jobs s.jobs)
            (gate a.gate) (origin o) (ticket t)
            (MedenSpec.Contracts.withInputs (gate a.gate) (bytes inputs.val))
            (CompletionRefinement.output r) at hm
          simp [MedenEvidenceImpl.evidence.Contracts.complete, hd, ha, hr, core.option.Option.is_some,
            subject_clone, h, MedenSpec.Contracts.complete, state, hd, ha, versioned, ← hm]
        | vis effect continuation => simp [h] at hm
        | div => simp [h] at hm
      · have hn : a.revision.val ≠ rev.val := fun h => hr (UScalar.eq_of_val_eq h)
        simp [MedenEvidenceImpl.evidence.Contracts.complete, hd, ha, hr, hn, core.option.Option.is_some,
          MedenSpec.Contracts.complete, state, versioned]

theorem start_refines (s : Contracts) (o : EvidenceOrigin) (rev : U64) (inputs : alloc.vec.Vec U8) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.start s o rev inputs;
        ok (out.1.map ticket, state out.2)) =
      ok (MedenSpec.Contracts.start (state s) (origin o) rev.val (bytes inputs.val)) := by
  cases o with
  | Model | Host | Checker =>
    simp [MedenEvidenceImpl.evidence.Contracts.start, core.cmp.PartialEq.ne.trait_default,
      core.cmp.PartialEq.ne.default, origin_eq, origin, MedenSpec.Contracts.start]
  | Controller =>
    cases hd : s.draft with
    | some d => simp [MedenEvidenceImpl.evidence.Contracts.start, hd, core.option.Option.is_some,
        core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
        origin_eq, MedenSpec.Contracts.start, state, origin]
    | none =>
      cases ha : s.accepted with
      | none => simp [MedenEvidenceImpl.evidence.Contracts.start, hd, ha, core.option.Option.is_some,
          core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
          origin_eq, MedenSpec.Contracts.start, state, origin]
      | some a =>
        by_cases hr : a.revision = rev
        · have hf := EvidenceRefinement.refresh_refines a.gate .Controller {a.gate.subject with inputs}
          cases h : MedenEvidenceImpl.evidence.Gate.refresh a.gate .Controller {a.gate.subject with inputs} with
          | ret out =>
            rcases out with ⟨allowed, nextGate⟩
            simp only [h, bind_ok, Result.ok.injEq] at hf
            change (allowed, gate nextGate) = Evidence.refresh (gate a.gate) .controller
              (MedenSpec.Contracts.withInputs (gate a.gate) (bytes inputs.val)) at hf
            let cleared := {s.jobs with active := none}
            cases hb : allowed with
            | false =>
              simp [MedenEvidenceImpl.evidence.Contracts.start, core.cmp.PartialEq.ne.trait_default,
                core.cmp.PartialEq.ne.default, origin_eq, hd, ha, hr, core.option.Option.is_some,
                MedenEvidenceImpl.evidence.CheckJobs.invalidate, subject_clone, h, hb,
                MedenSpec.Contracts.start, state, versioned, origin, ← hf,
                MedenSpec.CheckJobs.invalidate, jobs]
            | true =>
              have hj := CheckJobsRefinement.start_refines cleared .Controller nextGate
                {a.gate.subject with inputs}
              cases hc : MedenEvidenceImpl.evidence.CheckJobs.start cleared .Controller nextGate
                  {a.gate.subject with inputs} with
              | ret out =>
                rcases out with ⟨issued, nextJobs⟩
                simp only [hc, bind_ok, Result.ok.injEq] at hj
                change (issued.map ticket, jobs nextJobs) = MedenSpec.CheckJobs.start
                  (jobs cleared) .controller (gate nextGate)
                  (MedenSpec.Contracts.withInputs (gate a.gate) (bytes inputs.val)) at hj
                simp [MedenEvidenceImpl.evidence.Contracts.start, core.cmp.PartialEq.ne.trait_default,
                  core.cmp.PartialEq.ne.default, origin_eq, hd, ha, hr, core.option.Option.is_some,
                  MedenEvidenceImpl.evidence.CheckJobs.invalidate, subject_clone, h, hb, cleared, hc,
                  MedenSpec.Contracts.start, state, versioned, origin, ← hf, hb,
                  MedenSpec.CheckJobs.invalidate, jobs, cleared]
                simpa only [jobs, cleared, Option.map_none] using
                  And.intro (congrArg Prod.fst hj) (congrArg Prod.snd hj)
              | vis effect continuation => simp [hc] at hj
              | div => simp [hc] at hj
          | vis effect continuation => simp [h] at hf
          | div => simp [h] at hf
        · have hn : a.revision.val ≠ rev.val := fun h => hr (UScalar.eq_of_val_eq h)
          simp [MedenEvidenceImpl.evidence.Contracts.start, core.cmp.PartialEq.ne.trait_default,
            core.cmp.PartialEq.ne.default, origin_eq, hd, ha, hr, hn, core.option.Option.is_some,
            MedenSpec.Contracts.start, state, versioned, origin]

theorem propose_refines (s : Contracts) (o : EvidenceOrigin) (c : alloc.vec.Vec U8)
    (requirements : alloc.vec.Vec Obligation) :
    (do let out ← MedenEvidenceImpl.evidence.Contracts.propose s o c requirements; ok (out.1, state out.2)) =
      ok (MedenSpec.Contracts.propose (state s) (origin o) (bytes c.val)
        (requirements.val.map obligation)) := by
  have bound := U64.checked_add_bv_spec s.sequence 1#u64
  cases hc : U64.checked_add s.sequence 1#u64 with
  | none =>
    have hb : ¬ s.sequence.val < 18446744073709551615 := by
      simp [hc, U64.max_eq] at bound
      omega
    cases o <;> cases hd : s.draft <;> cases ha : s.accepted <;>
      simp [MedenEvidenceImpl.evidence.Contracts.propose,
        core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq,
        alloc.vec.Vec.is_empty, vec_eq, obligations_eq, hc, Aeneas.Std.lift,
        MedenSpec.Contracts.propose, MedenSpec.Contracts.fresh,
        MedenSpec.Contracts.sameProposal, MedenSpec.Contracts.selected,
        state, versioned, gate, origin, bytes_eq, hd, ha, hb]
    all_goals split_ifs <;> simp_all [versioned, gate, subject, phase, bytes_eq, bytes_empty, bytes_nil]
  | some n =>
    simp only [hc] at bound
    have hn : n.val = s.sequence.val + 1 := by simpa using bound.2.1
    have hb : s.sequence.val < 18446744073709551615 := by
      simp [U64.max_eq] at bound
      omega
    cases o <;> cases hd : s.draft <;> cases ha : s.accepted <;>
      simp [MedenEvidenceImpl.evidence.Contracts.propose,
        core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq,
        alloc.vec.Vec.is_empty, vec_eq, obligations_eq, hc, Aeneas.Std.lift,
        vec_clone, MedenEvidenceImpl.evidence.Gate.propose, MedenEvidenceImpl.evidence.CheckJobs.invalidate,
        MedenSpec.Contracts.propose, MedenSpec.Contracts.fresh,
        MedenSpec.Contracts.sameProposal, MedenSpec.Contracts.selected,
        MedenSpec.CheckJobs.invalidate, Evidence.propose,
        state, versioned, gate, jobs, subject, phase, origin, bytes_eq, hd, ha, hb, hn]
    all_goals split_ifs <;> simp_all [versioned, gate, subject, phase, bytes_eq, bytes_empty, bytes_nil, hn]

end MedenSpec.ContractsRefinement
