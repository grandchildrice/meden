/-!
# Attestation tiers

Mirrors `tagent_core::attestation::tier_of`.
-/
namespace TAgentSpec

/-- What a verifier concluded. -/
inductive VResult where
  | proved | tested | scanned | failed
  deriving DecidableEq, Repr, Inhabited

/-- The gate's classification. -/
inductive Tier where
  | verified | verifiedWithTcb | notVerified
  deriving DecidableEq, Repr, Inhabited

/-- Components a Lean-only attestation may trust. -/
def leanOnlyTcb : List String := ["lean4", "lake", "mathlib", "aeneas", "charon", "rustc"]

/-- The tier rule. -/
def tierOf (res : VResult) (tcb : List String) (backend : String) (approved : List String) : Tier :=
  if res != .proved then .notVerified
  else if tcb.all (leanOnlyTcb.contains ·) then .verified
  else if approved.contains backend then .verifiedWithTcb
  else .notVerified

/-- Only a proof can verify. -/
theorem tier_not_proved (res : VResult) (tcb : List String) (b : String) (ap : List String)
    (h : res ≠ .proved) : tierOf res tcb b ap = .notVerified := by
  simp [tierOf, h]

/-- A proof with a Lean-only TCB is verified without approval. -/
theorem tier_lean_only (tcb : List String) (b : String) (ap : List String)
    (h : tcb.all (leanOnlyTcb.contains ·) = true) : tierOf .proved tcb b ap = .verified := by
  unfold tierOf
  split
  · rename_i hne; exact absurd hne (by decide)
  · rfl

/-- A proof with a foreign TCB and no approved backend is not verified. -/
theorem tier_foreign_unapproved (tcb : List String) (b : String) (ap : List String)
    (h1 : tcb.all (leanOnlyTcb.contains ·) = false) (h2 : ap.contains b = false) :
    tierOf .proved tcb b ap = .notVerified := by
  unfold tierOf
  split
  · rename_i hne; exact absurd hne (by decide)
  · split
    · rename_i hall; rw [h1] at hall; exact Bool.noConfusion hall
    · split
      · rename_i hin; rw [h2] at hin; exact Bool.noConfusion hin
      · rfl

/-- A proof with a foreign TCB and an approved backend is verified-with-TCB. -/
theorem tier_foreign_approved (tcb : List String) (b : String) (ap : List String)
    (h1 : tcb.all (leanOnlyTcb.contains ·) = false) (h2 : ap.contains b = true) :
    tierOf .proved tcb b ap = .verifiedWithTcb := by
  unfold tierOf
  split
  · rename_i hne; exact absurd hne (by decide)
  · split
    · rename_i hall; rw [h1] at hall; exact Bool.noConfusion hall
    · rfl

/-- Whatever is verified was proved with a Lean-only TCB. -/
theorem verified_implies_lean_only (res : VResult) (tcb : List String) (b : String)
    (ap : List String) (h : tierOf res tcb b ap = .verified) :
    res = .proved ∧ tcb.all (leanOnlyTcb.contains ·) = true := by
  unfold tierOf at h
  split at h
  · exact absurd h (by decide)
  · rename_i hp
    split at h
    · exact ⟨by simpa using hp, by assumption⟩
    · split at h <;> exact absurd h (by decide)

/-- Tests and scans never verify, whatever is approved. -/
theorem tested_never_verifies (tcb : List String) (b : String) (ap : List String) :
    tierOf .tested tcb b ap = .notVerified ∧ tierOf .scanned tcb b ap = .notVerified := by
  exact ⟨tier_not_proved _ _ _ _ (by decide), tier_not_proved _ _ _ _ (by decide)⟩

end TAgentSpec
