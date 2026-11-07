import MedenSpec.Capability

/-!
# Information flow

The model of `crates/meden-agent`: labels, their two lattices, and the two
flow rules the gate applies to a tool call.

A label records where data came from (`sources`, joined by union) and who may
read it (`readers`, joined by intersection). The two join in opposite
directions, which is the point: combining values makes the result *less*
trusted and *less* widely readable.

The theorems here are the ones the safety argument rests on:

* `join_untrusted_left` / `join_untrusted_right`: trust cannot be laundered by
  combining untrusted data with trusted data.
* `untrusted_control_refused`: if any argument that steers a call carries an
  untrusted source, the call is refused. This is the anti-prompt-injection
  property, and it mentions no model.
* `unreadable_not_disclosed`: if any argument may not be read by the
  principal a call discloses to, the call is refused.
-/
namespace MedenSpec

/-- Where a piece of data came from. -/
inductive Source where
  /-- The runtime itself. -/
  | runtime
  /-- The user's own instruction. -/
  | user
  /-- The planning model's output. -/
  | assistant
  /-- The output of a named tool. -/
  | tool (name : String)
  deriving DecidableEq, Repr, Inhabited

/-- Who may read a value. -/
inductive Readers where
  /-- Anyone. -/
  | anyone
  /-- Only these principals. -/
  | only (principals : List String)
  deriving DecidableEq, Repr, Inhabited

namespace Readers

/-- May this principal read? -/
def allows : Readers → String → Bool
  | .anyone, _ => true
  | .only s, p => s.contains p

/-- The meet: who may read data derived from both. -/
def meet : Readers → Readers → Readers
  | .anyone, r => r
  | r, .anyone => r
  | .only a, .only b => .only (a.filter (fun x => b.contains x))

/-- A principal may read the meet exactly when it may read both sides.
`anyone` is the identity, so this also pins down its behaviour. -/
theorem meet_allows (a b : Readers) (p : String) :
    (a.meet b).allows p = (a.allows p && b.allows p) := by
  cases a with
  | anyone => cases b <;> simp [meet, allows]
  | only as =>
    cases b with
    | anyone => simp [meet, allows]
    | only bs =>
      simp only [meet, allows, List.contains]
      simp [List.mem_filter]

/-- `anyone` is the left identity of the meet. -/
theorem anyone_meet (r : Readers) : Readers.meet .anyone r = r := by
  cases r <;> rfl

end Readers

/-- The provenance and confidentiality of one value. -/
structure Label where
  /-- Every source that contributed. -/
  sources : List Source
  /-- Who may read it. -/
  readers : Readers
  deriving Repr, Inhabited

namespace Label

/-- The label of data derived from both: sources unite, readers intersect. -/
def join (a b : Label) : Label :=
  { sources := a.sources ++ b.sources, readers := a.readers.meet b.readers }

/-- May this value steer an action? Only if every source is trusted. -/
def trustedForControl (l : Label) (trusted : List Source) : Bool :=
  l.sources.all (fun s => trusted.contains s)

/-- May this principal read this value? -/
def allowsReader (l : Label) (p : String) : Bool :=
  l.readers.allows p

/-- Joining never drops a source on the left. -/
theorem mem_join_left {s : Source} {a b : Label} (h : s ∈ a.sources) :
    s ∈ (a.join b).sources := by
  simp [join, List.mem_append]; exact Or.inl h

/-- Joining never drops a source on the right. -/
theorem mem_join_right {s : Source} {a b : Label} (h : s ∈ b.sources) :
    s ∈ (a.join b).sources := by
  simp [join, List.mem_append]; exact Or.inr h

/-- The join may steer exactly when both parts may. -/
theorem join_trusted (a b : Label) (trusted : List Source) :
    (a.join b).trustedForControl trusted
      = (a.trustedForControl trusted && b.trustedForControl trusted) := by
  simp [join, trustedForControl, List.all_append]

/-- Trust cannot be laundered: mixing untrusted data into trusted data does
not make the result able to steer. -/
theorem join_untrusted_left (a b : Label) (trusted : List Source)
    (h : a.trustedForControl trusted = false) :
    (a.join b).trustedForControl trusted = false := by
  rw [join_trusted, h, Bool.false_and]

/-- The same on the other side. -/
theorem join_untrusted_right (a b : Label) (trusted : List Source)
    (h : b.trustedForControl trusted = false) :
    (a.join b).trustedForControl trusted = false := by
  rw [join_trusted, h, Bool.and_false]

/-- A principal may read the join exactly when it may read both parts, so
joining never widens who may read. -/
theorem join_allows (a b : Label) (p : String) :
    (a.join b).allowsReader p = (a.allowsReader p && b.allowsReader p) := by
  simp [join, allowsReader, Readers.meet_allows]

end Label

/-! ## The gate's flow rules -/

/-- One argument of a call, as the flow rules see it. -/
structure Arg where
  /-- Does this argument steer what the call does? -/
  control : Bool
  /-- Its label. -/
  label : Label
  deriving Repr, Inhabited

/-- The integrity rule: nothing untrusted may steer. -/
def integrityOk (trusted : List Source) (args : List Arg) : Bool :=
  args.all (fun a => !a.control || a.label.trustedForControl trusted)

/-- The confidentiality rule: every argument must permit the reader this call
discloses to. -/
def disclosureOk (principal : String) (args : List Arg) : Bool :=
  args.all (fun a => a.label.allowsReader principal)

/-- **The anti-injection property.** If any argument that steers a call
carries a source outside the trusted set, the call is refused.

Nothing here mentions a model, a prompt or a classifier: the conclusion
follows from the label alone, so it holds however persuasive the injected
text is. -/
theorem untrusted_control_refused (trusted : List Source) (args : List Arg) (a : Arg)
    (hmem : a ∈ args) (hc : a.control = true)
    (hu : a.label.trustedForControl trusted = false) :
    integrityOk trusted args = false := by
  cases h : integrityOk trusted args with
  | false => rfl
  | true =>
    have := (List.all_eq_true.mp h) a hmem
    rw [hc, hu] at this
    simp at this

/-- **The anti-exfiltration property.** If any argument may not be read by the
principal a call discloses to, the call is refused. -/
theorem unreadable_not_disclosed (principal : String) (args : List Arg) (a : Arg)
    (hmem : a ∈ args) (hu : a.label.allowsReader principal = false) :
    disclosureOk principal args = false := by
  cases h : disclosureOk principal args with
  | false => rfl
  | true =>
    have := (List.all_eq_true.mp h) a hmem
    rw [hu] at this
    simp at this

/-- Adding an argument never turns a refusal into an acceptance. -/
theorem integrity_cons (trusted : List Source) (a : Arg) (args : List Arg)
    (h : integrityOk trusted args = false) :
    integrityOk trusted (a :: args) = false := by
  simp [integrityOk, List.all_cons] at h ⊢
  simp [h]

end MedenSpec
