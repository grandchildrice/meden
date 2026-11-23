import Lean
/-! A bounded presentation-only bridge to Informath's documented Dedukti dialect.
It is not an export for kernel checking: universes/implicit instance arguments
are deliberately absent, as in Informath's own Lean exporter. -/
open Lean Meta
namespace MedenInterchange

private def ident (n : Name) : String :=
  String.join <| n.toString.toList.map fun c =>
    if c.toNat < 128 && (c.isAlphanum || c == '_') then String.singleton c
    else if c == '.' then "__" else "_u" ++ toString c.toNat ++ "_"

private def constant (n : Name) : String :=
  let known := [(``Nat, "Nat"), (``Int, "Int"), (``Bool, "bool"),
    (``True, "true"), (``False, "false"), (``And, "and"), (``Or, "or"),
    (``Not, "not"), (``Iff, "iff"), (``Eq, "Eq"), (``Ne, "Neq"),
    (``LE.le, "Leq"), (``LT.lt, "Lt"), (``GE.ge, "Geq"), (``GT.gt, "Gt"),
    (``HAdd.hAdd, "plus"), (``HSub.hSub, "minus"), (``HMul.hMul, "times"),
    (``HDiv.hDiv, "div"), (``HPow.hPow, "pow"), (``Nat.succ, "succ")]
  (known.find? (fun entry => entry.1 == n)).map Prod.snd |>.getD (ident n)

def expression : Nat → Expr → MetaM String
  | 0, _ => throwError "Informath interchange depth exceeded"
  | fuel + 1, input => do
    let e := input.consumeMData
    match e with
    | .forallE name ty body bi =>
      withLocalDecl name bi ty fun x => do
        if ← isProp ty then
          return "(if " ++ (← expression fuel ty) ++ " " ++ (← expression fuel (body.instantiate1 x)) ++ ")"
        else
          return "(forall " ++ (← expression fuel ty) ++ " (" ++ ident name ++ " => " ++ (← expression fuel (body.instantiate1 x)) ++ "))"
    | .lam name ty body bi =>
      withLocalDecl name bi ty fun x => do
        return "(" ++ ident name ++ " => " ++ (← expression fuel (body.instantiate1 x)) ++ ")"
    | .fvar id => return ident (← id.getDecl).userName
    | .sort l => return if l.isZero then "Prop" else "Set"
    | .lit (.natVal n) => return toString n
    | .const name _ => return constant name
    | _ =>
      let fn := e.getAppFn
      let args := e.getAppArgs
      if fn.isConstOf ``OfNat.ofNat && args.size >= 2 then
        return ← expression fuel args[1]!
      if fn.isConstOf ``Exists && args.size == 2 then
        return "(exists " ++ (← expression fuel args[0]!) ++ " " ++ (← expression fuel args[1]!) ++ ")"
      if fn.isConstOf ``Membership.mem && args.size >= 2 then
        return "(element " ++ (← expression fuel args[args.size - 1]!) ++ " " ++ (← expression fuel args[args.size - 2]!) ++ ")"
      if ← isProp e then
        if let .const name levels := fn then
          if constant name == ident name then
            if let some (.defnInfo info) := (← getEnv).find? name then
              return ← expression fuel ((info.value.instantiateLevelParams info.levelParams levels).beta args)
      let info ← getFunInfoNArgs fn args.size
      let explicit := args.zip info.paramInfo |>.filterMap fun (arg, param) =>
        if param.binderInfo.isExplicit && !param.isProp then some arg else none
      let rendered ← explicit.mapM (expression fuel)
      return "(" ++ (← expression fuel fn) ++
        (if rendered.isEmpty then "" else " " ++ String.intercalate " " rendered.toList) ++ ")"

def declaration (ci : ConstantInfo) : MetaM String := do
  if ← isProp ci.type then
    return "statement : Proof " ++ (← expression 64 ci.type) ++ "."
  else
    match ci with
    | .defnInfo info =>
      forallTelescope ci.type fun args result => do
        let mut binders := ""
        for arg in args do
          let decl ← arg.fvarId!.getDecl
          let ty ← expression 64 decl.type
          binders := binders ++ "(" ++ ident decl.userName ++ " : " ++ (if ← isProp decl.type then "Proof " else "Elem ") ++ ty ++ ") -> "
        let value ← expression 64 info.value
        let ty ← expression 64 result
        return "def " ++ ident ci.name ++ " : " ++ binders ++ (if result.isSort && result.sortLevel!.isZero then "Prop" else "Elem " ++ ty) ++ " := " ++ value ++ "."
    | _ => throwError "No defining expression"
end MedenInterchange
