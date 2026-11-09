import MedenSpec.ProcessorRefinement

/-! Arbitrary finite compositions of the extracted production operations.
HTTP effects use the consuming getters used by I/O. Local effects observe
their token snapshot; the adapter's consuming resource comparison is proved
separately by local_consume_refines. Physical I/O is not modeled here. -/
namespace MedenSpec.ProcessorTrace
open Aeneas Aeneas.Std Result ByteVectors ProcessorRefinement MedenProcessorImpl

inductive Action where
  | approve : processor.ProcessorOrigin → U64 → Action
  | revoke : processor.ProcessorOrigin → Action
  | dispatch : Slice U8 → U64 → Action

def action : Action → Processor.Action
  | .approve o t => .approve (origin o) t.val
  | .revoke o => .revoke (origin o)
  | .dispatch session t => .dispatch (bytes session.val) t.val

def observe : processor.Dispatch → Result Processor.Binding
  | .Local c => do
    let p ← processor.LocalCall.into_parameters c
    let observed ← observeLocal p
    ok (.local observed)
  | .Http c => do
    let observed ← observeHttp c
    ok (.http observed)

theorem observe_refines (d : processor.Dispatch) : observe d = ok (effect d) := by
  cases d <;> simp [observe, effect, processor.LocalCall.into_parameters, local_getters_refine, http_getters_refine]

def stepExtracted (s : processor.State) : Action → Result (Processor.Observation × processor.State)
  | .approve o t => do
    let r ← processor.State.approve s o t
    ok (⟨r.1, none⟩, r.2)
  | .revoke o => do
    let r ← processor.State.revoke s o
    ok (⟨r.1, none⟩, r.2)
  | .dispatch session t => do
    let r ← processor.State.dispatch s session t
    let observed ← match r.1 with
      | none => ok none
      | some d => do let b ← observe d; ok (some b)
    ok (⟨observed.isSome, observed⟩, r.2)

theorem step_refines (s : processor.State) (a : Action) :
    (do let r ← stepExtracted s a; ok (r.1, state r.2)) =
      ok (Processor.step (state s) (action a)) := by
  cases a with
  | approve o t =>
    have h := approve_refines s o t
    cases hr : processor.State.approve s o t with
    | ret r =>
      have he : (r.1, state r.2) = Processor.approve (state s) (origin o) t.val := by
        simpa [hr] using h
      simp [stepExtracted, hr, action, Processor.step, ← he]
    | vis i k => simp [hr] at h
    | div => simp [hr] at h
  | revoke o =>
    have h := revoke_refines s o
    cases hr : processor.State.revoke s o with
    | ret r =>
      have he : (r.1, state r.2) = Processor.revoke (state s) (origin o) := by
        simpa [hr] using h
      simp [stepExtracted, hr, action, Processor.step, ← he]
    | vis i k => simp [hr] at h
    | div => simp [hr] at h
  | dispatch session t =>
    have h := dispatch_refines s session t
    cases hr : processor.State.dispatch s session t with
    | ret r =>
      have he : (r.1.map effect, state r.2) =
          Processor.dispatch (state s) (bytes session.val) t.val := by simpa [hr] using h
      simp [stepExtracted, hr, action, Processor.step, ← he]
      cases r.1 <;> simp [observe_refines]
    | vis i k => simp [hr] at h
    | div => simp [hr] at h

private theorem step_success (s : processor.State) (a : Action) :
    ∃ r, stepExtracted s a = ok r ∧
      (r.1, state r.2) = Processor.step (state s) (action a) := by
  have h := step_refines s a
  cases hr : stepExtracted s a with
  | ret r => exact ⟨r, by simp_all, by simpa [hr] using h⟩
  | vis i k => simp [hr] at h
  | div => simp [hr] at h

def runExtracted (s : processor.State) : List Action → Result (List Processor.Observation × processor.State)
  | [] => ok ([], s)
  | a :: rest => do
    let r ← stepExtracted s a
    let tail ← runExtracted r.2 rest
    ok (r.1 :: tail.1, tail.2)

private theorem run_success (s : processor.State) (actions : List Action) :
    ∃ r, runExtracted s actions = ok r ∧
      (r.1, state r.2) = Processor.run (state s) (actions.map action) := by
  induction actions generalizing s with
  | nil => exact ⟨([], s), rfl, rfl⟩
  | cons a rest ih =>
    obtain ⟨r, hr, he⟩ := step_success s a
    obtain ⟨tail, ht, heTail⟩ := ih r.2
    refine ⟨(r.1 :: tail.1, tail.2), ?_, ?_⟩
    · simp [runExtracted, hr, ht]
    · simp only [List.map_cons, Processor.run]
      rw [← he, ← heTail]

theorem trace_refines (s : processor.State) (actions : List Action) :
    (do let r ← runExtracted s actions; ok (r.1, state r.2)) =
      ok (Processor.run (state s) (actions.map action)) := by
  obtain ⟨r, hr, he⟩ := run_success s actions
  simp [hr, he]

/-- All returned effects contain the original transport and every original
parameter. Thus a local connection cannot emit an HTTP effect at any step. -/
theorem extracted_frozen (s : processor.State) (actions : List Action) :
    ∃ r, runExtracted s actions = ok r ∧ (state r.2).binding = (state s).binding ∧
      ∀ obs ∈ r.1, ∀ b, obs.effect = some b → b = (state s).binding := by
  obtain ⟨r, hr, he⟩ := run_success s actions
  have hb := Processor.run_preserves_binding (state s) (actions.map action)
  have hf := Processor.run_effect_exact (state s) (actions.map action)
  rw [← he] at hb hf
  exact ⟨r, hr, hb, fun obs ho b hb => hf b obs ho hb⟩

theorem extracted_revoked (s : processor.State) (h : phase s.phase = .revoked)
    (actions : List Action) :
    ∃ r, runExtracted s actions = ok r ∧ state r.2 = state s ∧
      ∀ obs ∈ r.1, obs.effect = none := by
  obtain ⟨r, hr, he⟩ := run_success s actions
  have hsafe := Processor.revoked_run (state s) h (actions.map action)
  rw [← he] at hsafe
  exact ⟨r, hr, hsafe⟩

end MedenSpec.ProcessorTrace
