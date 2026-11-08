/-!
Exact outbound authorization. `request` is the canonical, immutable wire
description including method, target, audience, headers and body. Equality
is over the complete description, not over a hash or a model's claim.

The theorem assumes only the host can issue permits, and all sends use
`reserve`. OS confinement and the transport's use of the reserved request
are separate implementation obligations, not axioms proved here.
-/
namespace MedenSpec.Transfer

structure Request where
  session : String
  recipient : String
  request : String
  deriving DecidableEq, Repr

structure Permit where
  request : Request
  expires : Nat
  spent : Bool
  deriving DecidableEq, Repr

def allowed (now : Nat) (session : String) (p : Permit) (r : Request) : Bool :=
  decide (r.session = session ∧ p.request = r ∧ now < p.expires ∧ p.spent = false)

def reserve (now : Nat) (session : String) (p : Permit) (r : Request) : Permit × Bool :=
  let accepted := allowed now session p r
  ({ p with spent := p.spent || accepted }, accepted)

theorem accepted_iff (now : Nat) (session : String) (p : Permit) (r : Request) :
    (reserve now session p r).2 = true ↔
      r.session = session ∧ p.request = r ∧ now < p.expires ∧ p.spent = false := by
  simp [reserve, allowed]

theorem exact_request (now : Nat) (session : String) (p : Permit) (r : Request)
    (h : (reserve now session p r).2 = true) : p.request = r := by
  exact ((accepted_iff now session p r).mp h).2.1

theorem accepted_spends (now : Nat) (session : String) (p : Permit) (r : Request)
    (h : (reserve now session p r).2 = true) :
    (reserve now session p r).1.spent = true := by
  simp only [reserve] at h ⊢
  simp [h]

theorem no_replay (now later : Nat) (session otherSession : String)
    (p : Permit) (r other : Request)
    (h : (reserve now session p r).2 = true) :
    (reserve later otherSession (reserve now session p r).1 other).2 = false := by
  have used := accepted_spends now session p r h
  change allowed later otherSession (reserve now session p r).1 other = false
  simp [allowed, used]

theorem rejection_preserves (now : Nat) (session : String) (p : Permit) (r : Request)
    (h : (reserve now session p r).2 = false) :
    (reserve now session p r).1 = p := by
  simp only [reserve] at h ⊢
  cases p with
  | mk request expires spent => simp_all

theorem no_changed_recipient (now : Nat) (session : String) (p : Permit) (r : Request)
    (h : p.request.recipient ≠ r.recipient) :
    (reserve now session p r).2 = false := by
  cases eq : (reserve now session p r).2 with
  | false => rfl
  | true => exact False.elim (h (congrArg Request.recipient (exact_request now session p r eq)))

theorem no_changed_content (now : Nat) (session : String) (p : Permit) (r : Request)
    (h : p.request.request ≠ r.request) :
    (reserve now session p r).2 = false := by
  cases eq : (reserve now session p r).2 with
  | false => rfl
  | true => exact False.elim (h (congrArg Request.request (exact_request now session p r eq)))

end MedenSpec.Transfer
