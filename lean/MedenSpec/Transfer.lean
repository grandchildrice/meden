/-! Exact permission predicate used by processor bindings. Complete canonical
request equality is checked without hashing. Permission issuance, revocation,
transport use and clocks remain host implementation obligations. One-use file
exports use the separate review/dispatch state, not this reusable predicate. -/
namespace MedenSpec.Transfer

structure Request where
  session : List Nat
  recipient : List Nat
  request : List Nat
  deriving DecidableEq, Repr

structure Permit where
  request : Request
  expires : Nat
  spent : Bool
  deriving DecidableEq, Repr

def allowed (now : Nat) (session : List Nat) (p : Permit) (r : Request) : Bool :=
  decide (r.session = session ∧ p.request = r ∧ now < p.expires ∧ p.spent = false)

theorem accepted_iff (now : Nat) (session : List Nat) (p : Permit) (r : Request) :
    allowed now session p r = true ↔
      r.session = session ∧ p.request = r ∧ now < p.expires ∧ p.spent = false := by
  simp [allowed]

theorem exact_request (now : Nat) (session : List Nat) (p : Permit) (r : Request)
    (h : allowed now session p r = true) : p.request = r := by
  exact ((accepted_iff now session p r).mp h).2.1

theorem no_foreign_session (now : Nat) (session : List Nat) (p : Permit) (r : Request)
    (h : r.session ≠ session) : allowed now session p r = false := by
  simp [allowed, h]

theorem no_expired (now : Nat) (session : List Nat) (p : Permit) (r : Request)
    (h : p.expires ≤ now) : allowed now session p r = false := by
  simp [allowed, Nat.not_lt.mpr h]

theorem no_revoked (now : Nat) (session : List Nat) (p : Permit) (r : Request)
    (h : p.spent = true) : allowed now session p r = false := by
  simp [allowed, h]

theorem no_changed_recipient (now : Nat) (session : List Nat) (p : Permit) (r : Request)
    (h : p.request.recipient ≠ r.recipient) :
    allowed now session p r = false := by
  cases eq : allowed now session p r with
  | false => rfl
  | true => exact False.elim (h (congrArg Request.recipient (exact_request now session p r eq)))

theorem no_changed_content (now : Nat) (session : List Nat) (p : Permit) (r : Request)
    (h : p.request.request ≠ r.request) :
    allowed now session p r = false := by
  cases eq : allowed now session p r with
  | false => rfl
  | true => exact False.elim (h (congrArg Request.request (exact_request now session p r eq)))

end MedenSpec.Transfer
