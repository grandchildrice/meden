import Lean

/-! Deterministic controlled English from elaborated expressions, plus a fixed-seed
Plausible observation. This module never constructs or replaces project proofs. -/
open Lean Meta

namespace MedenSemantic

private def words (name : Name) : String :=
  let text := name.toString
  let chars := text.toList
  let (_, result) := chars.foldl (fun (previous, out) c =>
    let separator := c == '_' || c == '.'
    let capital := c.isUpper && previous.isLower
    (c, out ++ (if separator || capital then " " else "") ++
      (if separator then "" else String.singleton c.toLower))) (' ', "")
  result.trimAscii.toString

private def join (xs : Array String) : String :=
  String.intercalate ", " xs.toList

private def explicitArgs (fn : Expr) (args : Array Expr) : MetaM (Array Expr) := do
  let info ← getFunInfoNArgs fn args.size
  return args.zip info.paramInfo |>.filterMap fun (arg, param) =>
    if param.binderInfo.isExplicit && !param.isProp then some arg else none

private def atom (n : Name) : String :=
  if n == ``Nat then "a natural number"
  else if n == ``Int then "an integer"
  else if n == ``Bool then "a boolean"
  else if n == ``True then "true"
  else if n == ``False then "false"
  else if n == ``Bool.true then "true"
  else if n == ``Bool.false then "false"
  else if n.toString == "Real" then "a real number"
  else if n.toString == "Complex" then "a complex number"
  else "“" ++ words n ++ "”"

/-- Fuel bounds interpretation; unknown applications remain named, never guessed. -/
def english : Nat → Expr → MetaM String
  | 0, _ => pure "an expression beyond the explanation depth limit"
  | fuel + 1, input => do
    let e := input.consumeMData
    match e with
    | .forallE name ty body bi =>
      if ← isProp ty then
        let premise ← english fuel ty
        withLocalDecl name bi ty fun x => do
          return "if " ++ premise ++ ", then " ++ (← english fuel (body.instantiate1 x))
      else
        withLocalDecl name bi ty fun x => do
          let label := if name.isAnonymous then "value" else name.toString
          return "for every " ++ label ++ " that is " ++ (← english fuel ty) ++
            ", " ++ (← english fuel (body.instantiate1 x))
    | .lam name ty body bi =>
      withLocalDecl name bi ty fun x => do
        return "the function taking " ++ name.toString ++ " to " ++ (← english fuel (body.instantiate1 x))
    | .letE name _ty value body _ =>
      return "with " ++ name.toString ++ " equal to " ++ (← english fuel value) ++
        ", " ++ (← english fuel (body.instantiate1 value))
    | .fvar id => return (← id.getDecl).userName.toString
    | .sort level => return if level.isZero then "a proposition" else "a type"
    | .lit (.natVal n) => return toString n
    | .lit (.strVal s) => return "the text “" ++ s ++ "”"
    | .const name levels =>
      if ← isProp e then
        if let some (.defnInfo info) := (← getEnv).find? name then
          return ← english fuel (info.value.instantiateLevelParams info.levelParams levels)
      return atom name
    | .proj _ index value =>
      return "component " ++ toString (index + 1) ++ " of " ++ (← english fuel value)
    | _ =>
      let fn := e.getAppFn
      let args := e.getAppArgs
      let name := fn.constName?
      if name == some ``OfNat.ofNat && args.size >= 2 then
        return ← english fuel args[1]!
      if name == some ``Exists && args.size == 2 then
        let pred := args[1]!
        if let .lam n ty body bi := pred then
          return ← withLocalDecl n bi ty fun x => do
            return "there exists " ++ n.toString ++ " that is " ++ (← english fuel ty) ++
              " such that " ++ (← english fuel (body.instantiate1 x))
      let binary := [(``Eq, "equals"), (``Ne, "differs from"),
        (``LT.lt, "is less than"), (``LE.le, "is at most"),
        (``GT.gt, "is greater than"), (``GE.ge, "is at least"),
        (``And, "and"), (``Or, "or"), (``Iff, "if and only if"),
        (``HAdd.hAdd, "plus"), (``HSub.hSub, "minus"),
        (``HMul.hMul, "times"), (``HDiv.hDiv, "divided by"),
        (``HPow.hPow, "raised to")]
      if name == some ``Membership.mem && args.size >= 2 then
        return "(" ++ (← english fuel args[args.size - 1]!) ++ ") belongs to (" ++
          (← english fuel args[args.size - 2]!) ++ ")"
      if let some (_, verb) := binary.find? (fun (n, _) => name == some n) then
        if args.size >= 2 then
          return "(" ++ (← english fuel args[args.size - 2]!) ++ ") " ++ verb ++
            " (" ++ (← english fuel args[args.size - 1]!) ++ ")"
      if name == some ``Not && args.size == 1 then
        return "it is not the case that " ++ (← english fuel args[0]!)
      if name == some ``Nat.succ && args.size == 1 then
        return "one plus " ++ (← english fuel args[0]!)
      if ← isProp e then
        if let .const n levels := fn then
          if let some (.defnInfo info) := (← getEnv).find? n then
            return ← english fuel ((info.value.instantiateLevelParams info.levelParams levels).beta args)
      let explicit ← explicitArgs fn args
      let rendered ← explicit.mapM (english fuel)
      let head ← english fuel fn
      if ← isProp e then
        return head ++ (if rendered.isEmpty then " holds" else " holds for " ++ join rendered)
      else
        return head ++ (if rendered.isEmpty then "" else " applied to " ++ join rendered)

private def field (key value : String) := (key, Json.str value)

/-- A deterministic falsification condition, distinct from a discovered witness. -/
def falsify : Nat → Expr → MetaM String
  | 0, _ => pure "make the remaining proposition false"
  | fuel + 1, input => do
    let e := input.consumeMData
    if let .forallE name ty body bi := e then
      return ← if ← isProp ty then
        withLocalDecl name bi ty fun x => do
          return "satisfy (" ++ (← english fuel ty) ++ ") and (" ++ (← falsify fuel (body.instantiate1 x)) ++ ")"
      else
        withLocalDecl name bi ty fun x => do
          return "find " ++ name.toString ++ " that is " ++ (← english fuel ty) ++
            " such that (" ++ (← falsify fuel (body.instantiate1 x)) ++ ")"
    let args := e.getAppArgs
    let name := e.getAppFn.constName?
    if (name == some ``And || name == some ``Or) && args.size == 2 then
      return "(" ++ (← falsify fuel args[0]!) ++ ") " ++
        (if name == some ``And then "or" else "and") ++ " (" ++ (← falsify fuel args[1]!) ++ ")"
    -- Negating an order relation must not assume an undeclared total order.
    let comparisons := [(``Eq, "does not equal"), (``Ne, "does not differ from"),
      (``LE.le, "is not at most"), (``LT.lt, "is not less than"),
      (``GE.ge, "is not at least"), (``GT.gt, "is not greater than")]
    if let some (_, verb) := comparisons.find? (fun (n, _) => name == some n) then
      if args.size >= 2 then
        return "(" ++ (← english fuel args[args.size - 2]!) ++ ") " ++ verb ++
          " (" ++ (← english fuel args[args.size - 1]!) ++ ")"
    return "make this false: " ++ (← english fuel e)

def describe (ci : ConstantInfo) : MetaM Json := do
  let isProposition ← isProp ci.type
  let sentence ← if isProposition then english 64 ci.type else
    match ci with
    | .defnInfo info => pure <| "Defined as " ++ (← english 64 info.value)
    | _ => pure <| "An object whose type is " ++ (← english 64 ci.type)
  let sentence := if sentence.utf8ByteSize <= 60000 then sentence else "The elaborated meaning exceeds the explanation size bound."
  let predicate ← if isProposition then pure false else
    forallTelescope ci.type fun _ result => pure (result.isSort && result.sortLevel!.isZero)
  let counterfactual ← if isProposition then falsify 64 ci.type else pure ""
  let counterfactual := if counterfactual.utf8ByteSize <= 60000 then counterfactual else "make the complete proposition false; its explanation exceeds the size bound"
  let refutation := if isProposition then
      "To falsify this statement, " ++ counterfactual ++ "."
    else if predicate then
      "Inputs where the predicate is false illustrate its boundary; they do not refute the definition itself."
    else "A definition specifies an object rather than asserting a universal theorem. Review its inputs, output and defining expression."
  return Json.mkObj [("version", toJson (1 : Nat)), field "origin" "elaborated",
    field "sentence" sentence, field "renderer" "Structural English", field "refutation" refutation,
    ("checker", Json.null)]

end MedenSemantic
