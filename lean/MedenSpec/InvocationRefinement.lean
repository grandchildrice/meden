import MedenSpec.Invocation
import MedenSpec.ConversationRefinement

namespace MedenSpec.InvocationRefinement
open Aeneas Aeneas.Std Result MedenConversationImpl
open invocation
universe u
variable {α : Type u}

def rule : invocation.Rule → Invocation.Rule
  | .UnknownTool => .unknownTool | .InvalidArguments => .invalidArguments
  | .MissingArgument => .missingArgument | .Policy => .policy

def refusal (r : invocation.Refusal) : Invocation.Refusal := ⟨rule r.rule, r.reason⟩

def preparation : core.result.Result α invocation.Refusal → Except Invocation.Refusal α
  | .Ok request => .ok request | .Err reason => .error (refusal reason)

def state (s : Attempt α) : Invocation.State α := ⟨s.checked.map preparation, s.announced⟩

def notice : invocation.Notice → Invocation.Notice
  | .Allowed => .allowed | .Refused reason => .refused (refusal reason) | .Invalid => .invalid

def resolution : core.result.Result α conversation.ExecutionError → Except Conversation.Check α
  | .Ok request => .ok request
  | .Err error => .error (ConversationRefinement.check (.Err error))

theorem new_refines (checked : core.result.Result α invocation.Refusal) :
    (do let s ← Attempt.new checked; ok (state s)) = ok (Invocation.initial (preparation checked)) := by
  simp [Attempt.new, state, Invocation.initial]

theorem announce_refines (s : Attempt α) :
    (do let (n, next) ← Attempt.announce s; ok (notice n, state next)) =
      ok (Invocation.announce (state s)) := by
  rcases s with ⟨checked, announced⟩
  cases announced <;> cases checked with
  | none => simp [Attempt.announce, state, notice, Invocation.announce]
  | some result => cases result <;> simp [Attempt.announce, state, notice, preparation, Invocation.announce]

theorem dispatch_refines (s : Attempt α) (cancelled : Bool) (stop : Option conversation.Status) :
    (do let (r, next) ← Attempt.dispatch s cancelled stop; ok (resolution r, state next)) =
      ok (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)) := by
  rcases s with ⟨checked, announced⟩
  cases announced <;> cases checked with
  | none => simp [Attempt.dispatch, core.option.Option.take, state, resolution,
      ConversationRefinement.check, ConversationRefinement.status, Invocation.dispatch]
  | some result => cases result <;> cases cancelled <;> cases stop <;>
      simp [Attempt.dispatch, core.option.Option.take, flow.interrupt, state,
        preparation, resolution, refusal, ConversationRefinement.check, ConversationRefinement.status,
        Invocation.dispatch]

inductive Action where
  | announce | dispatch (cancelled : Bool) (stop : Option conversation.Status)

def action : Action → Invocation.Action
  | .announce => .announce
  | .dispatch cancelled stop => .dispatch cancelled (stop.map ConversationRefinement.status)

def one (s : Attempt α) : Action → Result (Invocation.Event α × Attempt α)
  | .announce => do
    let (n, next) ← Attempt.announce s
    ok (.notice (notice n), next)
  | .dispatch cancelled stop => do
    let (r, next) ← Attempt.dispatch s cancelled stop
    ok (.resolved (resolution r), next)

theorem one_refines (s : Attempt α) (a : Action) :
    (do let (event, next) ← one s a; ok (event, state next)) =
      ok (Invocation.step (state s) (action a)) := by
  cases a with
  | announce =>
    have h := announce_refines s
    cases hr : Attempt.announce s with
    | ret out =>
      rcases out with ⟨n, next⟩
      simp only [hr, bind_ok] at h
      change ok (notice n, state next) = ok (Invocation.announce (state s)) at h
      simp only [Result.ok.injEq] at h
      simp [one, hr, action, Invocation.step, ← h]
    | vis effect continuation => simp [hr] at h
    | div => simp [hr] at h
  | dispatch cancelled stop =>
    have h := dispatch_refines s cancelled stop
    cases hr : Attempt.dispatch s cancelled stop with
    | ret out =>
      rcases out with ⟨r, next⟩
      simp only [hr, bind_ok] at h
      change ok (resolution r, state next) =
        ok (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)) at h
      simp only [Result.ok.injEq] at h
      simp [one, hr, action, Invocation.step, ← h]
    | vis effect continuation => simp [hr] at h
    | div => simp [hr] at h

def runExtracted (s : Attempt α) : List Action → Result (Attempt α × List α × Nat)
  | [] => ok (s, [], 0)
  | a :: rest => do
    let (event, next) ← one s a
    let out ← runExtracted next rest
    ok (out.1, Invocation.payloads event ++ out.2.1, Invocation.notifications event + out.2.2)

private theorem one_success (s : Attempt α) (a : Action) :
    ∃ event next, one s a = ok (event, next) ∧
      (event, state next) = Invocation.step (state s) (action a) := by
  have h := one_refines s a
  cases hr : one s a with
  | ret out =>
    rcases out with ⟨event, next⟩
    exact ⟨event, next, rfl, by simpa [hr] using h⟩
  | vis effect continuation => simp [hr] at h
  | div => simp [hr] at h

theorem trace_refines (s : Attempt α) (actions : List Action) :
    (do let out ← runExtracted s actions;
        ok (Invocation.Trace.mk (state out.1) out.2.1 out.2.2)) =
      ok (Invocation.run (state s) (actions.map action)) := by
  induction actions generalizing s with
  | nil => simp [runExtracted, Invocation.run]
  | cons a rest ih =>
    obtain ⟨event, next, hr, hs⟩ := one_success s a
    have hh := ih next
    cases ho : runExtracted next rest with
    | ret out =>
      simp only [ho, bind_ok, Result.ok.injEq] at hh
      simp [runExtracted, hr, ho, Invocation.run, ← hs, ← hh]
    | vis effect continuation => simp [ho] at hh
    | div => simp [ho] at hh

theorem extracted_safe (checked : core.result.Result α invocation.Refusal) (initial final : Attempt α)
    (actions : List Action) (requests : List α) (count : Nat)
    (hi : Attempt.new checked = ok initial)
    (hr : runExtracted initial actions = ok (final, requests, count)) :
    requests.length ≤ 1 ∧ requests.length ≤ count ∧
      ∀ request ∈ requests, checked = .Ok request := by
  have hn := new_refines checked
  simp only [hi, bind_ok, Result.ok.injEq] at hn
  have ht := trace_refines initial actions
  simp only [hr, bind_ok, Result.ok.injEq] at ht
  have hs := Invocation.trace_safe (preparation checked) (actions.map action)
  dsimp only at hs
  rw [← hn, ← ht] at hs
  refine ⟨hs.1, hs.2.1, ?_⟩
  intro request member
  have hm := hs.2.2 request member
  cases checked <;> simpa [preparation] using hm

private theorem queued_ok_iff (s : Attempt α) (request : α) :
    (state s).queued = some (.ok request) ↔ s.checked = some (.Ok request) := by
  cases h : s.checked with
  | none => simp [state, h]
  | some result => cases result <;> simp [state, h, preparation]

theorem extracted_dispatch_condition (s next : Attempt α) (request : α)
    (cancelled : Bool) (stop : Option conversation.Status)
    (hr : Attempt.dispatch s cancelled stop = ok (.Ok request, next)) :
    s.announced = true ∧ s.checked = some (.Ok request) ∧ cancelled = false ∧ stop = none := by
  have h := dispatch_refines s cancelled stop
  simp only [hr, bind_ok] at h
  change ok (Except.ok request, state next) =
    ok (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)) at h
  simp only [Result.ok.injEq] at h
  have he : (Invocation.dispatch (state s) cancelled (stop.map ConversationRefinement.status)).1 =
      .ok request := (congrArg Prod.fst h).symm
  have hc := (Invocation.dispatch_iff _ _ _ _).mp he
  rw [queued_ok_iff] at hc
  simpa [state] using hc

end MedenSpec.InvocationRefinement
