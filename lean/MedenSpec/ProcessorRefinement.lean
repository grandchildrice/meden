import MedenSpec.Processor
import MedenSpec.Generated.Processor
import MedenSpec.ByteVectors

/-! Refinement of the actual Rust processor lifecycle and frozen parameters.
These theorems do not model HTTP, process execution, locks or clock provenance. -/
namespace MedenSpec.ProcessorRefinement
open Aeneas Aeneas.Std Result ByteVectors
open MedenProcessorImpl
set_option maxHeartbeats 8000000

private theorem literal_val (n : Nat) :
    (UScalar.mk (BitVec.ofNat 64 n) : U64).val = n % 2^64 := rfl

def origin : processor.ProcessorOrigin → Processor.Origin
  | .Host => .host | .Model => .model
def phase : processor.ProcessorPhase → Processor.Phase
  | .Pending => .pending | .Granted => .granted | .Revoked => .revoked
def localParams (p : processor.LocalParameters) : Processor.Local :=
  ⟨bytes p.engine.val, bytes p.weights.val⟩
def http (p : processor.HttpParameters) : Processor.Http :=
  ⟨bytes p.provider.val, bytes p.endpoint.val, bytes p.model.val,
    p.credential.map fun v => bytes v.val⟩
def binding : processor.Binding → Processor.Binding
  | .Local p => .local (localParams p) | .Http p => .http (http p)
def state (s : processor.State) : Processor.State :=
  ⟨bytes s.session.val, binding s.binding, phase s.phase, s.expires.val⟩
def effect : processor.Dispatch → Processor.Binding
  | .Local c => .local (localParams c.parameters)
  | .Http c => .http (http c.parameters)

private theorem origin_eq (x y : processor.ProcessorOrigin) :
    processor.ProcessorOrigin.Insts.CoreCmpPartialEqProcessorOrigin.eq x y =
      ok (decide (origin x = origin y)) := by
  cases x <;> cases y <;> simp [processor.ProcessorOrigin.Insts.CoreCmpPartialEqProcessorOrigin.eq, origin, processor.ProcessorOrigin.read_discriminant]
private theorem phase_eq (x y : processor.ProcessorPhase) :
    processor.ProcessorPhase.Insts.CoreCmpPartialEqProcessorPhase.eq x y =
      ok (decide (phase x = phase y)) := by
  cases x <;> cases y <;> simp [processor.ProcessorPhase.Insts.CoreCmpPartialEqProcessorPhase.eq, phase, processor.ProcessorPhase.read_discriminant]

theorem local_clone (p : processor.LocalParameters) :
    processor.LocalParameters.Insts.CoreCloneClone.clone p = ok p := by
  simp [processor.LocalParameters.Insts.CoreCloneClone.clone, vec_clone]
theorem http_clone (p : processor.HttpParameters) :
    processor.HttpParameters.Insts.CoreCloneClone.clone p = ok p := by
  rcases p with ⟨provider, endpoint, model, credential⟩
  cases credential <;>
    simp [processor.HttpParameters.Insts.CoreCloneClone.clone, vec_clone]

theorem local_new_refines (engine weights : alloc.vec.Vec U8) :
    (do let p ← processor.LocalParameters.new engine weights; ok (localParams p)) =
      ok (Processor.Local.mk (bytes engine.val) (bytes weights.val)) := by
  simp [processor.LocalParameters.new, localParams]
theorem http_new_refines (provider endpoint model : alloc.vec.Vec U8)
    (credential : Option (alloc.vec.Vec U8)) :
    (do let p ← processor.HttpParameters.new provider endpoint model credential; ok (http p)) =
      ok (Processor.Http.mk (bytes provider.val) (bytes endpoint.val) (bytes model.val)
        (credential.map fun v => bytes v.val)) := by
  simp [processor.HttpParameters.new, http]

/-- Observes the actual getters used by the local adapter. -/
def observeLocal (p : processor.LocalParameters) : Result Processor.Local := do
  let engine ← processor.LocalParameters.impl.engine p
  let weights ← processor.LocalParameters.impl.weights p
  ok ⟨bytes engine.val, bytes weights.val⟩

theorem local_getters_refine (p : processor.LocalParameters) :
    observeLocal p = ok (localParams p) := by
  simp [observeLocal, processor.LocalParameters.impl.engine,
    processor.LocalParameters.impl.weights, localParams, alloc.vec.Vec.deref]

/-- Observes the exact consuming path and all HTTP getter values, including
the distinction between an absent credential and an empty credential. -/
def observeHttp (c : processor.HttpCall) : Result Processor.Http := do
  let p ← processor.HttpCall.into_parameters c
  let provider ← processor.HttpParameters.impl.provider p
  let endpoint ← processor.HttpParameters.impl.endpoint p
  let model ← processor.HttpParameters.impl.model p
  let credential ← processor.HttpParameters.impl.credential p
  ok ⟨bytes provider.val, bytes endpoint.val, bytes model.val,
    credential.map fun v => bytes v.val⟩

theorem http_getters_refine (c : processor.HttpCall) :
    observeHttp c = ok (http c.parameters) := by
  cases h : c.parameters.credential <;>
    simp [observeHttp, processor.HttpCall.into_parameters,
      processor.HttpParameters.impl.provider, processor.HttpParameters.impl.endpoint,
      processor.HttpParameters.impl.model, processor.HttpParameters.impl.credential,
      http, h, alloc.vec.Vec.deref]

theorem local_consume_refines (c : processor.LocalCall) (actual : processor.LocalParameters) :
    processor.LocalCall.consume_for c actual = ok (decide (localParams c.parameters = localParams actual)) := by
  unfold processor.LocalCall.consume_for processor.LocalParameters.Insts.CoreCmpPartialEqLocalParameters.eq
  simp only [vec_eq]
  by_cases h1 : c.parameters.engine.val = actual.engine.val <;>
    by_cases h2 : c.parameters.weights.val = actual.weights.val <;>
    simp [h1, h2, localParams, Processor.Local.mk.injEq, bytes_eq]

theorem local_into_refines (c : processor.LocalCall) :
    (do let p ← processor.LocalCall.into_parameters c; observeLocal p) =
      ok (localParams c.parameters) := by
  simp [processor.LocalCall.into_parameters, local_getters_refine]

theorem new_refines (session : alloc.vec.Vec U8) (b : processor.Binding) :
    (do let s ← processor.State.new session b; ok (state s)) =
      ok (Processor.initial (bytes session.val) (binding b)) := by
  simp [processor.State.new, Processor.initial, state, phase]

theorem binding_refines (s : processor.State) :
    (do let b ← processor.State.impl.binding s; ok (binding b)) = ok (state s).binding := by
  simp [processor.State.impl.binding, state]
theorem phase_refines (s : processor.State) :
    (do let p ← processor.State.impl.phase s; ok (phase p)) = ok (state s).phase := by
  simp [processor.State.impl.phase, state]
theorem expires_refines (s : processor.State) :
    (do let e ← processor.State.impl.expires s; ok e.val) = ok (state s).expires := by
  simp [processor.State.impl.expires, state]

theorem approve_refines (s : processor.State) (o : processor.ProcessorOrigin) (now : U64) :
    (do let r ← processor.State.approve s o now; ok (r.1, state r.2)) =
      ok (Processor.approve (state s) (origin o) now.val) := by
  have bound := U64.checked_add_bv_spec now processor.APPROVAL_LIFETIME_MILLIS
  cases hc : U64.checked_add now processor.APPROVAL_LIFETIME_MILLIS with
  | none =>
    simp only [hc] at bound
    have hb : ¬ now.val + Processor.lifetime ≤ Processor.maxTime := by
      exact Nat.not_le_of_lt (by simpa [U64.max_eq, processor.APPROVAL_LIFETIME_MILLIS,
        Processor.lifetime, Processor.maxTime, literal_val] using bound)
    cases o <;> cases hp : s.phase <;>
      simp [processor.State.approve, core.cmp.PartialEq.ne.trait_default,
        core.cmp.PartialEq.ne.default, origin_eq, phase_eq, hp, hc, Aeneas.Std.lift,
        Processor.approve, state, origin, phase, hb]
  | some n =>
    simp only [hc] at bound
    have hn : n.val = now.val + Processor.lifetime := by
      simpa [processor.APPROVAL_LIFETIME_MILLIS, Processor.lifetime, literal_val] using bound.2.1
    have hb : now.val + Processor.lifetime ≤ Processor.maxTime := by
      have h := bound.1
      simpa [U64.max_eq, processor.APPROVAL_LIFETIME_MILLIS, Processor.lifetime,
        Processor.maxTime, literal_val] using h
    cases o <;> cases hp : s.phase <;>
      simp [processor.State.approve, core.cmp.PartialEq.ne.trait_default,
        core.cmp.PartialEq.ne.default, origin_eq, phase_eq, hp, hc, Aeneas.Std.lift,
        Processor.approve, state, origin, phase, hb, hn]

theorem revoke_refines (s : processor.State) (o : processor.ProcessorOrigin) :
    (do let r ← processor.State.revoke s o; ok (r.1, state r.2)) =
      ok (Processor.revoke (state s) (origin o)) := by
  cases o <;> simp [processor.State.revoke, core.cmp.PartialEq.ne.trait_default,
    core.cmp.PartialEq.ne.default, origin_eq, Processor.revoke, origin, state, phase]

theorem authorized_refines (s : processor.State) (session : Slice U8) (now : U64) :
    processor.State.is_authorized s session now =
      ok (Processor.authorized (state s) (bytes session.val) now.val) := by
  cases hp : s.phase <;>
    by_cases hs : s.session.val = session.val <;>
    by_cases ht : now.val < s.expires.val <;>
    simp [processor.State.is_authorized, phase_eq, slice_eq, alloc.vec.Vec.deref,
      hp, hs, ht, state, phase, Processor.authorized, bytes_eq]

theorem dispatch_refines (s : processor.State) (session : Slice U8) (now : U64) :
    (do let r ← processor.State.dispatch s session now; ok (r.1.map effect, state r.2)) =
      ok (Processor.dispatch (state s) (bytes session.val) now.val) := by
  cases hp : s.phase <;> cases hb : s.binding <;>
    by_cases hs : s.session.val = session.val <;>
    by_cases ht : now.val < s.expires.val <;>
    simp [processor.State.dispatch, phase_eq, authorized_refines, local_clone, http_clone,
      hp, hb, hs, ht, show s.expires.val ≤ now.val ↔ ¬ now.val < s.expires.val from Nat.not_lt.symm,
      UScalar.le_equiv, state, phase, effect, binding,
      Processor.dispatch, Processor.authorized, bytes_eq]

end MedenSpec.ProcessorRefinement
