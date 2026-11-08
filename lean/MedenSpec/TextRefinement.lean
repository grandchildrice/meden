import MedenSpec.Generated.Text
import MedenSpec.Text

/-! The actual production numeric transformation and finite compositions.
This does not extract String iteration, character conversion or rendering. -/
namespace MedenSpec.TextRefinement
open Aeneas Aeneas.Std Result
set_option maxHeartbeats 4000000

private theorem literal_val (n : Nat) :
    (UScalar.mk (BitVec.ofNat 32 n) : U32).val = n % 2^32 := rfl

theorem scalar_refines (code : U32) :
    (do let out ← MedenTextImpl.text.visible_codepoint code; ok out.val) =
      ok (Text.render code.val) := by
  simp [MedenTextImpl.text.visible_codepoint]
  split <;> simp_all [Text.render, Text.directional, Text.control,
    literal_val, UScalar.ofNatCore_val_eq]
  all_goals
    repeat' (first
      | (split <;> simp_all [UScalar.ofNatCore_val_eq])
      | omega)
  all_goals (try split_ifs) <;> try omega

private theorem success (code : U32) :
    ∃ out, MedenTextImpl.text.visible_codepoint code = ok out ∧ out.val = Text.render code.val := by
  have h := scalar_refines code
  cases hr : MedenTextImpl.text.visible_codepoint code with
  | ret out => exact ⟨out, by simp_all, by simpa [hr] using h⟩
  | vis effect continuation => simp [hr] at h
  | div => simp [hr] at h

theorem extracted_safe (code : U32) :
    ∃ out, MedenTextImpl.text.visible_codepoint code = ok out ∧ ¬Text.forbidden out.val := by
  obtain ⟨out, hr, hv⟩ := success code
  exact ⟨out, hr, hv ▸ Text.safe code.val⟩

theorem extracted_scalar (code : U32) (h : Text.scalar code.val) :
    ∃ out, MedenTextImpl.text.visible_codepoint code = ok out ∧ Text.scalar out.val := by
  obtain ⟨out, hr, hv⟩ := success code
  exact ⟨out, hr, hv ▸ Text.valid_output code.val h⟩

/-- A finite driver calling the real extracted scalar operation, not an
extraction or model of the Rust String iterator/collector. -/
def run : List U32 → Result (List Nat)
  | [] => ok []
  | code :: rest => do
    let out ← MedenTextImpl.text.visible_codepoint code
    let tail ← run rest
    ok (out.val :: tail)

theorem stream_refines (codes : List U32) :
    run codes = ok (codes.map fun code => Text.render code.val) := by
  induction codes with
  | nil => rfl
  | cons code rest ih =>
    obtain ⟨out, hr, hv⟩ := success code
    simp [run, hr, hv, ih]

theorem stream_safe (codes : List U32) :
    ∃ out, run codes = ok out ∧ out.length = codes.length ∧ ∀ code ∈ out, ¬Text.forbidden code := by
  refine ⟨codes.map (fun code => Text.render code.val), stream_refines codes, by simp, ?_⟩
  intro code h
  obtain ⟨input, _, rfl⟩ := List.mem_map.mp h
  exact Text.safe input.val

end MedenSpec.TextRefinement
