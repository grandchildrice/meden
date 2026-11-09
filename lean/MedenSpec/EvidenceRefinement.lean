import MedenSpec.Evidence
import MedenSpec.Generated.Evidence
import MedenSpec.ByteVectors

/-! Refinement of the production evidence kernel. It proves correspondence of
typed state/decisions, not soundness of arbitrary checker adapters or snapshots. -/
namespace MedenSpec.EvidenceRefinement
set_option maxHeartbeats 4000000
open Aeneas Aeneas.Std Result ByteVectors
open MedenEvidenceImpl.evidence (EvidenceKind EvidenceOrigin EvidencePhase Subject Obligation Receipt Gate)
abbrev Kind := EvidenceKind
abbrev Origin := EvidenceOrigin
abbrev Phase := EvidencePhase

def kind : Kind → Evidence.Kind
  | .Empirical => .empirical | .Bounded => .bounded | .Theorem => .theorem | .Refinement => .refinement
def origin : Origin → Evidence.Origin
  | .Model => .model | .Host => .host | .Checker => .checker | .Controller => .controller
def phase : Phase → Evidence.Phase
  | .Draft => .draft | .Reviewed => .reviewed | .Accepted => .accepted
def subject (s : Subject) : Evidence.Subject := ⟨bytes s.session.val, bytes s.contract.val, bytes s.inputs.val⟩
def obligation (r : Obligation) : Evidence.Obligation := ⟨bytes r.statement.val, kind r.kind, bytes r.checker.val⟩
def receipt (r : Receipt) : Evidence.Receipt := ⟨subject r.subject, r.position.val, obligation r.obligation, r.passed, r.active⟩
def gate (g : Gate) : Evidence.Gate := ⟨subject g.subject, g.obligations.val.map obligation, phase g.phase, g.completed.val⟩

private theorem kind_eq (x y : Kind) :
    MedenEvidenceImpl.evidence.EvidenceKind.Insts.CoreCmpPartialEqEvidenceKind.eq x y = ok (decide (kind x = kind y)) := by
  cases x <;> cases y <;> simp [MedenEvidenceImpl.evidence.EvidenceKind.Insts.CoreCmpPartialEqEvidenceKind.eq, EvidenceKind.read_discriminant, kind]
private theorem phase_eq (x y : Phase) :
    MedenEvidenceImpl.evidence.EvidencePhase.Insts.CoreCmpPartialEqEvidencePhase.eq x y = ok (decide (phase x = phase y)) := by
  cases x <;> cases y <;> simp [MedenEvidenceImpl.evidence.EvidencePhase.Insts.CoreCmpPartialEqEvidencePhase.eq, EvidencePhase.read_discriminant, phase]
theorem origin_eq (x y : Origin) :
    MedenEvidenceImpl.evidence.EvidenceOrigin.Insts.CoreCmpPartialEqEvidenceOrigin.eq x y = ok (decide (origin x = origin y)) := by
  cases x <;> cases y <;> simp [MedenEvidenceImpl.evidence.EvidenceOrigin.Insts.CoreCmpPartialEqEvidenceOrigin.eq, EvidenceOrigin.read_discriminant, origin]
theorem subject_eq (x y : Subject) :
    MedenEvidenceImpl.evidence.Subject.Insts.CoreCmpPartialEqSubject.eq x y = ok (decide (subject x = subject y)) := by
  unfold MedenEvidenceImpl.evidence.Subject.Insts.CoreCmpPartialEqSubject.eq
  simp only [vec_eq]
  by_cases hs : x.session.val = y.session.val <;> by_cases hc : x.contract.val = y.contract.val <;>
    by_cases hi : x.inputs.val = y.inputs.val <;>
    simp [hs, hc, hi, subject, Evidence.Subject.mk.injEq, bytes_eq]
theorem obligation_eq (x y : Obligation) :
    MedenEvidenceImpl.evidence.Obligation.Insts.CoreCmpPartialEqObligation.eq x y = ok (decide (obligation x = obligation y)) := by
  unfold MedenEvidenceImpl.evidence.Obligation.Insts.CoreCmpPartialEqObligation.eq
  simp only [vec_eq, kind_eq]
  by_cases hs : x.statement.val = y.statement.val <;> by_cases hk : kind x.kind = kind y.kind <;>
    by_cases hc : x.checker.val = y.checker.val <;>
    simp [hs, hk, hc, obligation, Evidence.Obligation.mk.injEq, bytes_eq]

theorem propose_refines (s : Subject) (requirements : alloc.vec.Vec Obligation) :
    (do let out ← MedenEvidenceImpl.evidence.Gate.propose s requirements; ok (gate out)) =
      ok (Evidence.propose (subject s) (requirements.val.map obligation)) := by
  simp [MedenEvidenceImpl.evidence.Gate.propose, gate, phase, Evidence.propose]

theorem review_refines (g : Gate) (o : Origin) :
    (do let out ← MedenEvidenceImpl.evidence.Gate.review g o; ok (out.1, gate out.2)) =
      ok (Evidence.review (gate g) (origin o)) := by
  unfold MedenEvidenceImpl.evidence.Gate.review
  simp only [core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq, phase_eq]
  cases o <;> cases h : g.phase <;> simp [origin, phase, gate, Evidence.review, h]

theorem accept_refines (g : Gate) (o : Origin) :
    (do let out ← MedenEvidenceImpl.evidence.Gate.accept g o; ok (out.1, gate out.2)) =
      ok (Evidence.accept (gate g) (origin o)) := by
  unfold MedenEvidenceImpl.evidence.Gate.accept
  simp only [core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq, phase_eq]
  cases o <;> cases h : g.phase <;> by_cases he : g.obligations.val = [] <;>
    simp [origin, phase, gate, Evidence.accept, h, he, alloc.vec.Vec.is_empty]

theorem eligible_refines (g : Gate) (s : Subject) :
    MedenEvidenceImpl.evidence.Gate.eligible g s = ok (Evidence.eligible (gate g) (subject s)) := by
  unfold MedenEvidenceImpl.evidence.Gate.eligible
  simp only [phase_eq, subject_eq]
  cases h : g.phase <;>
    by_cases he : g.obligations.val = [] <;>
    by_cases hl : g.completed.val = g.obligations.val.length <;>
    simp [phase, gate, Evidence.eligible, h, he, hl, alloc.vec.Vec.is_empty, alloc.vec.Vec.len]

theorem is_accepted_refines (g : Gate) :
    MedenEvidenceImpl.evidence.Gate.is_accepted g = ok (decide ((gate g).phase = .accepted)) := by
  unfold MedenEvidenceImpl.evidence.Gate.is_accepted
  cases h : g.phase <;> simp [phase_eq, gate, phase, h]

theorem refresh_refines (g : Gate) (o : Origin) (s : Subject) :
    (do let out ← MedenEvidenceImpl.evidence.Gate.refresh g o s; ok (out.1, gate out.2)) =
      ok (Evidence.refresh (gate g) (origin o) (subject s)) := by
  unfold MedenEvidenceImpl.evidence.Gate.refresh
  simp only [core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq, subject_eq, vec_ne]
  by_cases hs : g.subject.session.val = s.session.val <;>
    by_cases hc : g.subject.contract.val = s.contract.val <;>
    by_cases he : subject g.subject = subject s <;>
    cases o <;>
    simp_all [origin, gate, Evidence.refresh, subject, bytes_eq]

private theorem get_obligation (v : alloc.vec.Vec Obligation) (i : Usize) :
    core.slice.Slice.get (core.slice.index.SliceIndexUsizeSlice Obligation)
      (alloc.vec.Vec.deref v) i = ok (v.val[i.val]?) := by
  simp [core.slice.Slice.get, alloc.vec.Vec.deref]

private theorem next_fits (g : Gate) (expected : Obligation)
    (hg : g.obligations.val[g.completed.val]? = some expected) :
    ∃ n, Usize.checked_add g.completed 1#usize = some n ∧ n.val = g.completed.val + 1 := by
  have hi := (List.getElem?_eq_some_iff.mp hg).choose
  have hl : g.obligations.val.length ≤ Usize.max := by simp
  have hs := Usize.checked_add_bv_spec g.completed 1#usize
  cases ha : Usize.checked_add g.completed 1#usize with
  | none => simp only [ha] at hs; simp at hs; omega
  | some n =>
    simp only [ha] at hs
    exact ⟨n, rfl, by simpa using hs.2.1⟩

theorem record_refines (g : Gate) (o : Origin) (s : Subject) (r : Receipt) :
    (do let out ← MedenEvidenceImpl.evidence.Gate.record g o s r; ok (out.1, gate out.2)) =
      ok (Evidence.record (gate g) (origin o) (subject s) (receipt r)) := by
  cases hactive : r.active with
  | false => simp [MedenEvidenceImpl.evidence.Gate.record, hactive, Evidence.record, Evidence.corresponds, receipt]
  | true =>
      cases hg : g.obligations.val[g.completed.val]? with
      | none =>
        unfold MedenEvidenceImpl.evidence.Gate.record
        simp only [core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq, phase_eq, subject_eq, get_obligation]
        cases o <;> cases hp : g.phase <;>
          by_cases hs : subject g.subject = subject s <;>
          by_cases hr : subject r.subject = subject s <;>
          by_cases hi : r.position.val = g.completed.val <;> cases hb : r.passed <;>
          simp [origin, phase, gate, receipt, hp, hs, hr, hi, hb, hg, hactive, Evidence.record, Evidence.corresponds, List.getElem?_map, ← UScalar.eq_equiv]
      | some expected =>
        obtain ⟨n, ha, hn⟩ := next_fits g expected hg
        unfold MedenEvidenceImpl.evidence.Gate.record
        simp only [core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default, origin_eq, phase_eq, subject_eq, get_obligation, obligation_eq]
        cases o <;> cases hp : g.phase <;>
          by_cases hs : subject g.subject = subject s <;>
          by_cases hr : subject r.subject = subject s <;>
          by_cases hi : r.position.val = g.completed.val <;> cases hb : r.passed <;>
          by_cases he : obligation expected = obligation r.obligation <;>
          simp [origin, phase, gate, receipt, hp, hs, hr, hi, hb, hg, he, ha, hn, hactive, Evidence.record, Evidence.corresponds, List.getElem?_map, ← UScalar.eq_equiv, lift]


end MedenSpec.EvidenceRefinement
