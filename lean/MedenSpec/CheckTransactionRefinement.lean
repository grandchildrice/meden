import MedenSpec.CheckTransaction
import MedenSpec.ContractsTrace

namespace MedenSpec.CheckTransactionRefinement
open Aeneas Aeneas.Std Result ByteVectors MedenEvidenceImpl
set_option maxHeartbeats 8000000
set_option linter.unusedSimpArgs false

def preparation {T : Type} (p : check.Preparation T) : CheckTransaction.Preparation T :=
  ⟨ContractsRefinement.view p.agreement, p.data⟩

def running {T : Type} (r : check.Running T) : CheckTransaction.Running T :=
  ⟨ContractsRefinement.view r.agreement, CheckJobsRefinement.ticket r.ticket, r.data⟩

private theorem mapped_success {α β : Type} (m : Result α) (f : α → β) (target : β)
    (h : (do let value ← m; ok (f value)) = ok target) :
    ∃ value, m = ok value ∧ f value = target := by
  cases hm : m with
  | ret value => exact ⟨value, rfl, by simpa [hm] using h⟩
  | vis effect continuation => simp [hm] at h
  | div => simp [hm] at h

private theorem accepted_success (current : evidence.Contracts) :
    ∃ view, evidence.Contracts.impl.accepted current = ok view ∧
      view.map ContractsRefinement.view = Contracts.accepted (ContractsRefinement.state current) :=
  mapped_success _ _ _ (ContractsRefinement.accepted_refines current)

theorem preparation_data {T : Type} (p : check.Preparation T) :
    check.Preparation.impl.data p = ok (preparation p).data := rfl

theorem running_data {T : Type} (r : check.Running T) :
    check.Running.impl.data r = ok (running r).data := rfl

theorem running_ticket {T : Type} (r : check.Running T) :
    (do let ticket ← check.Running.impl.ticket r; ok (CheckJobsRefinement.ticket ticket)) =
      ok (running r).ticket := by simp [check.Running.impl.ticket, running]

theorem begin_refines {T : Type} (current : evidence.Contracts)
    (agreement : evidence.ContractView) (data : T) :
    (do let out ← check.Preparation.begin current agreement data
        ok (out.1.map preparation, ContractsRefinement.state out.2)) =
      ok (CheckTransaction.begin (ContractsRefinement.state current)
        (ContractsRefinement.view agreement) data) := by
  obtain ⟨accepted, ha, mapped⟩ := accepted_success current
  obtain ⟨edited, he, editMap⟩ := mapped_success _ _ _
    (ContractsRefinement.edit_refines current evidence.EvidenceOrigin.Controller)
  have size : (alloc.vec.Vec.len agreement.obligations = 1#usize) ↔
      (ContractsRefinement.view agreement).obligations.length = 1 := by
    simp [UScalar.eq_equiv, alloc.vec.Vec.len_val, ContractsRefinement.view]
  unfold check.Preparation.begin
  simp only [ContractsRefinement.is_accepted_refines, bind_ok, bne_iff_ne]
  by_cases ready : Contracts.isAccepted (ContractsRefinement.state current) = true
  · by_cases length : (ContractsRefinement.view agreement).obligations.length = 1
    · have lengthNative := size.mpr length
      simp only [ready, ↓reduceIte, lengthNative, ne_self_iff_false, ha, bind_ok]
      cases accepted with
      | none => simp [CheckTransaction.begin, ready, length, ← mapped]
      | some accepted =>
        simp only [ContractsRefinement.view_eq, bind_ok]
        by_cases same : ContractsRefinement.view accepted = ContractsRefinement.view agreement
        · simp only [same, ↓reduceIte, he, bind_ok]
          simp only [CheckTransaction.begin, ready, length, ← mapped, Option.some.injEq,
            same, and_self, ↓reduceIte]
          have stateMap := congrArg Prod.snd editMap
          simp only [EvidenceRefinement.origin] at stateMap
          rcases edited with ⟨allowed, next⟩
          simpa [preparation, CheckTransaction.begin, ready, length, ← mapped, same] using
            congrArg (fun state => ok (some (CheckTransaction.Preparation.mk
              (ContractsRefinement.view agreement) data), state)) stateMap
        · simp [same, CheckTransaction.begin, ready, length, ← mapped]
    · have noLength : alloc.vec.Vec.len agreement.obligations ≠ 1#usize :=
        fun h => length (size.mp h)
      simp [noLength, CheckTransaction.begin, ready, length]
  · simp [ready, CheckTransaction.begin]

private theorem normalized_identity (left right : Contracts.View) :
    {left with subject := {left.subject with inputs := right.subject.inputs}} = right ↔
      CheckTransaction.identity left = CheckTransaction.identity right := by
  rcases left with ⟨revision, subject, obligations⟩
  rcases right with ⟨revision', subject', obligations'⟩
  rcases subject with ⟨session, contract, inputs⟩
  rcases subject' with ⟨session', contract', inputs'⟩
  simp [CheckTransaction.identity, and_assoc]

theorem issue_refines {T : Type} (prepared : check.Preparation T)
    (current : evidence.Contracts) (inputs : alloc.vec.Vec U8) :
    (do let out ← check.Preparation.issue prepared current inputs
        ok (out.1.map running, ContractsRefinement.state out.2)) =
      ok (CheckTransaction.issue (preparation prepared) (ContractsRefinement.state current)
        (bytes inputs.val)) := by
  obtain ⟨accepted, ha, mapped⟩ := accepted_success current
  obtain ⟨issued, hs, startMap⟩ := mapped_success _ _ _
    (ContractsRefinement.start_refines current evidence.EvidenceOrigin.Controller
      prepared.agreement.revision inputs)
  unfold check.Preparation.issue
  simp only [ha, bind_ok]
  cases accepted with
  | none => simp [CheckTransaction.issue, ← mapped]
  | some view =>
    simp only [vec_clone, bind_ok, core.cmp.PartialEq.ne.trait_default,
      core.cmp.PartialEq.ne.default, ContractsRefinement.view_eq, bind_ok,
      ContractsRefinement.is_accepted_refines]
    have normalization :
        ContractsRefinement.view {view with subject :=
          {view.subject with inputs := prepared.agreement.subject.inputs}} =
        {ContractsRefinement.view view with subject :=
          {(ContractsRefinement.view view).subject with inputs :=
            (ContractsRefinement.view prepared.agreement).subject.inputs}} := rfl
    simp only [normalization, normalized_identity]
    by_cases same : CheckTransaction.identity (ContractsRefinement.view view) =
        CheckTransaction.identity (ContractsRefinement.view prepared.agreement)
    · by_cases ready : Contracts.isAccepted (ContractsRefinement.state current) = true
      · rcases issued with ⟨ticket, next⟩
        cases ticket <;>
          simp [same, ready, hs, preparation, running, CheckTransaction.issue,
            ← mapped, EvidenceRefinement.origin, ← startMap]
        all_goals exact ⟨(congrArg Prod.fst startMap).symm, congrArg Prod.snd startMap⟩
      · simp [same, ready, preparation, CheckTransaction.issue]
    · simp [same, preparation, CheckTransaction.issue, ← mapped]

private theorem obligation_clone (required : evidence.Obligation) :
    evidence.Obligation.Insts.CoreCloneClone.clone required = ok required := by
  simp [evidence.Obligation.Insts.CoreCloneClone.clone, vec_clone,
    evidence.EvidenceKind.Insts.CoreCloneClone.clone]

private theorem first_obligation (values : alloc.vec.Vec evidence.Obligation) :
    core.slice.Slice.get (core.slice.index.SliceIndexUsizeSlice evidence.Obligation)
      (alloc.vec.Vec.deref values) 0#usize = ok (values.val[0]?) := by
  simp [core.slice.Slice.get, alloc.vec.Vec.deref]

private theorem refuse_refines {T U : Type} (r : check.Running T) (current : evidence.Contracts) :
    (do let out ← (do
          let (_, next) ← evidence.Contracts.abort current evidence.EvidenceOrigin.Controller r.ticket
          ok ((none : Option U), next))
        ok (out.1, ContractsRefinement.state out.2)) =
      ok (CheckTransaction.refuse (running r) (ContractsRefinement.state current)) := by
  obtain ⟨out, ha, mapped⟩ := mapped_success _ _ _
    (ContractsRefinement.abort_refines current evidence.EvidenceOrigin.Controller r.ticket)
  have stateMap := congrArg Prod.snd mapped
  simp only [EvidenceRefinement.origin] at stateMap
  rcases out with ⟨allowed, next⟩
  simpa [ha, CheckTransaction.refuse, running] using
    congrArg (fun state => ok ((none : Option U), state)) stateMap

theorem complete_refines {T U : Type} (r : check.Running T) (current : evidence.Contracts)
    (inputs outputInputs checker : alloc.vec.Vec U8) (output : U) :
    (do let out ← check.Running.complete r current inputs outputInputs checker output
        ok (out.1, ContractsRefinement.state out.2)) =
      ok (CheckTransaction.complete (running r) (ContractsRefinement.state current)
        (bytes inputs.val) (bytes outputInputs.val) (bytes checker.val) output) := by
  unfold check.Running.complete
  simp only [first_obligation, bind_ok]
  cases obligations : r.agreement.obligations.val with
  | nil =>
    simpa [obligations, CheckTransaction.complete, running, ContractsRefinement.view]
      using (refuse_refines (U := U) r current)
  | cons required rest =>
    simp only [obligations, List.getElem?_cons_zero, vec_ne, bind_ok]
    by_cases same : bytes checker.val = bytes required.checker.val
    · have sameNative := (bytes_eq _ _).mp same
      simp [sameNative, vec_clone, obligation_clone]
      let result : evidence.CheckResult :=
        ⟨{r.agreement.subject with inputs := outputInputs}, 0#usize, required, true⟩
      obtain ⟨completed, hc, completeMap⟩ := mapped_success _ _ _
        (ContractsRefinement.complete_refines current evidence.EvidenceOrigin.Checker
          r.agreement.revision r.ticket inputs result)
      rcases completed with ⟨allowed, next⟩
      have mappedResult : CompletionRefinement.output result =
          Completion.Output.mk {(ContractsRefinement.view r.agreement).subject with
            inputs := bytes outputInputs.val} 0 (EvidenceRefinement.obligation required) true := rfl
      simp only [mappedResult, EvidenceRefinement.origin] at completeMap
      change evidence.Contracts.complete current evidence.EvidenceOrigin.Checker
        r.agreement.revision r.ticket inputs result = ok (allowed, next) at hc
      dsimp only [result] at hc
      simp only [hc, bind_ok]
      simp only [ContractsRefinement.view, EvidenceRefinement.obligation] at completeMap
      cases allowed with
      | false =>
        simpa [CheckTransaction.complete, running, ContractsRefinement.view, obligations,
          same, EvidenceRefinement.obligation, ← completeMap, result] using
          (refuse_refines (U := U) r next)
      | true =>
        simp only [ContractsRefinement.eligible_refines, bind_ok]
        by_cases eligible : Contracts.eligible (ContractsRefinement.state next)
            r.agreement.revision.val (bytes outputInputs.val) = true
        · simp [eligible, CheckTransaction.complete, running, ContractsRefinement.view,
            obligations, same, EvidenceRefinement.obligation, ← completeMap, result]
        · simpa [eligible, CheckTransaction.complete, running, ContractsRefinement.view,
            obligations, same, EvidenceRefinement.obligation, ← completeMap, result] using
            (refuse_refines (U := U) r next)
    · have differentNative : checker.val ≠ required.checker.val :=
        fun eq => same (congrArg bytes eq)
      simpa [same, differentNative, CheckTransaction.complete, running, ContractsRefinement.view,
        obligations, EvidenceRefinement.obligation] using (refuse_refines (U := U) r current)

end MedenSpec.CheckTransactionRefinement
