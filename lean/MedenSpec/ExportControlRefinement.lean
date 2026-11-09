import MedenSpec.ExportControl
import MedenSpec.Generated.Evidence

/-! Exact transition and finite-trace refinement of the extracted production
Rust review kernel. The trace driver composes extracted steps; it is not an
extraction/proof of the application's I/O dispatcher, mutex or UI/parser. -/
namespace MedenSpec.ExportControlRefinement
open Aeneas Aeneas.Std Result
open MedenEvidenceImpl.export_control (Phase Origin ExportEvent Decision)

def phase : Phase → ExportControl.Phase
  | .Pending => .pending | .Reviewed => .reviewed | .Rejected => .rejected
  | .Attempted => .attempted | .ResponseReceived => .responseReceived
  | .DeliveryUncertain => .deliveryUncertain

def origin : Origin → ExportControl.Origin
  | .Model => .model | .Host => .host | .Transport => .transport

def event : ExportEvent → ExportControl.Event
  | .Propose => .propose | .Review => .review | .Reject => .reject | .Approve => .approve
  | .ResponseReceived => .responseReceived | .DeliveryUncertain => .deliveryUncertain

def decision (d : Decision) : ExportControl.Decision := ⟨phase d.phase, d.accepted, d.dispatch⟩

private theorem step_success (p : Phase) (o : Origin) (e : ExportEvent) :
    ∃ d, MedenEvidenceImpl.export_control.step p o e = ok d ∧ decision d = ExportControl.step (phase p) (origin o) (event e) := by
  cases p <;> cases o <;> cases e <;> exact ⟨_, rfl, rfl⟩

/-- All state, acceptance and dispatch observations of production Rust match. -/
theorem step_refines (p : Phase) (o : Origin) (e : ExportEvent) :
    (do let d ← MedenEvidenceImpl.export_control.step p o e; ok (decision d)) =
      ok (ExportControl.step (phase p) (origin o) (event e)) := by
  obtain ⟨d, hs, hd⟩ := step_success p o e
  simp [hs, hd]

/-- Invoke the actual extracted step repeatedly, abstracting only the final
state. Authority decisions are never supplied by a handwritten Rust model. -/
def runExtracted (p : Phase) : List (Origin × ExportEvent) → Result (ExportControl.Phase × Nat)
  | [] => ok (phase p, 0)
  | (o, e) :: rest => do
    let d ← MedenEvidenceImpl.export_control.step p o e
    let tail ← runExtracted d.phase rest
    ok (tail.1, (if d.dispatch then 1 else 0) + tail.2)

/-- Arbitrary finite compositions of the extracted step refine the spec trace. -/
theorem trace_refines (p : Phase) (events : List (Origin × ExportEvent)) :
    runExtracted p events = ok (ExportControl.run (phase p) (events.map (fun a => (origin a.1, event a.2)))) := by
  induction events generalizing p with
  | nil => rfl
  | cons action rest ih =>
    rcases action with ⟨o, e⟩
    obtain ⟨d, hs, hd⟩ := step_success p o e
    have hp := congrArg ExportControl.Decision.phase hd
    have hb := congrArg ExportControl.Decision.dispatch hd
    simp only [decision] at hp hb
    simp only [runExtracted, hs, bind_ok]
    rw [ih]
    simp [ExportControl.run, hp, hb]

/-- The actual extracted Rust kernel cannot authorize a second dispatch in
any finite stream of model, host and transport events. -/
theorem extracted_at_most_one (p : Phase) (events : List (Origin × ExportEvent)) :
    ∃ result, runExtracted p events = ok result ∧ result.2 ≤ 1 := by
  exact ⟨_, trace_refines p events, ExportControl.at_most_one_dispatch _ _⟩

end MedenSpec.ExportControlRefinement
