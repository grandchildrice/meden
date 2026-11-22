import Lean
import MedenPlausible.Testable
open Lean Meta
namespace MedenSearch
private def field (key value : String) := (key, Json.str value)
def sampleResult (p : Prop) [MedenPlausible.Testable p] : IO String := do
  let result ← MedenPlausible.Testable.checkIO p {
    numInst := 64, maxSize := 32, numRetries := 10, randomSeed := some 0, quiet := true }
  let (status, candidates) := match result with
    | .failure _ values _ => ("counterexample", values)
    | .success _ => ("not_found", [])
    | .gaveUp _ => ("gave_up", [])
  return (Json.mkObj [field "engine" "Plausible", field "status" status,
    ("seed", toJson (0 : Nat)), ("numInst", toJson (64 : Nat)), ("numRetries", toJson (10 : Nat)),
    ("maxSize", toJson (32 : Nat)), ("candidates", toJson candidates)]).compress

/-- Delta-expand only observed project predicates, preserving all binders. -/
def expandPredicates : Nat → Array Name → Expr → MetaM Expr
  | 0, _, _ => throwError "Predicate expansion exceeds its depth bound"
  | fuel + 1, modules, input => do
    let e := input.consumeMData
    let fn := e.getAppFn
    if let .const name levels := fn then
      let env ← getEnv
      let selected := (env.getModuleIdxFor? name).any fun idx =>
        modules.contains env.header.moduleNames[idx.toNat]!
      if selected && (← isProp e) then
        if let some (.defnInfo info) := env.find? name then
          return ← expandPredicates fuel modules ((info.value.instantiateLevelParams info.levelParams levels).beta e.getAppArgs)
    match e with
    | .forallE name ty body bi =>
      let ty ← expandPredicates fuel modules ty
      withLocalDecl name bi ty fun x => do
        let body ← expandPredicates fuel modules (body.instantiate1 x)
        return .forallE name ty (body.abstract #[x]) bi
    | .lam name ty body bi =>
      let ty ← expandPredicates fuel modules ty
      withLocalDecl name bi ty fun x => do
        let body ← expandPredicates fuel modules (body.instantiate1 x)
        return .lam name ty (body.abstract #[x]) bi
    | .app fn arg => return .app (← expandPredicates fuel modules fn) (← expandPredicates fuel modules arg)
    | _ => return e

private unsafe def checkCore (type : Expr) (modules : Array Name) : MetaM Json := do
  if !(← isProp type) then
    return Json.mkObj [field "engine" "Plausible", field "status" "not_proposition",
      ("candidates", toJson ([] : List String))]
  try withCurrHeartbeats do
    let normalized ← expandPredicates 64 modules type
    unless ← isDefEq type normalized do throwError "Predicate expansion changed the proposition"
    let decorated ← MedenPlausible.Decorations.addDecorations normalized
    let testable ← synthInstance (mkApp (mkConst ``MedenPlausible.Testable) decorated)
    let expression := mkApp2 (mkConst ``sampleResult) decorated testable
    let action ← evalExpr (IO String) (mkApp (mkConst ``IO) (mkConst ``String)) expression
    let output ← action
    match Json.parse output with
    | .ok result => return result
    | .error _ => throwError "Invalid Plausible observation"
  catch error =>
    let exhausted := error.isMaxHeartbeat || error.isMaxRecDepth
    let detail ← if exhausted then pure "Plausible preparation exceeded the bounded Lean computation budget." else
      try withCurrHeartbeats error.toMessageData.toString
      catch _ => pure "Plausible could not prepare this proposition."
    let detail := if detail.utf8ByteSize <= 65536 then detail else "Plausible preparation failed; its diagnostic exceeds 64 KiB."
    return Json.mkObj [field "engine" "Plausible", field "status" (if exhausted then "budget_exhausted" else "unsupported"),
      field "diagnostic" detail,
      ("candidates", toJson ([] : List String))]

/-- Compiler tasks may surface an IO failure instead of a Meta exception. Keep
their state and failure inside a separate context, preserving the explanation. -/
private unsafe def observe (ctx : PPContext) (type : Expr) (modules : Array Name) : IO Json := do
  try ctx.runMetaM <| checkCore type modules
  catch error =>
    let detail := error.toString
    return Json.mkObj [field "engine" "Plausible", field "status" "unavailable",
      field "diagnostic" (if detail.utf8ByteSize <= 65536 then detail else "Plausible evaluation failed; its diagnostic exceeds 64 KiB."),
      ("candidates", toJson ([] : List String))]

unsafe def check (type : Expr) (modules : Array Name) : MetaM Json := do
  observe (← getPPContext) type modules


unsafe def analyze (ci : ConstantInfo) (modules : Array Name) : MetaM Json := do
  if ← isProp ci.type then check ci.type modules else
    forallTelescope ci.type fun args result => do
      if result.isSort && result.sortLevel!.isZero then
        let value := match ci with
          | .defnInfo info => info.value.beta args
          | _ => mkAppN (mkConst ci.name (ci.levelParams.map Level.param)) args
        let proposition ← mkForallFVars args value
        let observation ← check proposition modules
        let observation := observation.setObjVal! "mode" (.str "predicate")
        if (observation.getObjValAs? String "status").toOption == some "counterexample" then
          pure <| observation.setObjVal! "status" (.str "predicate_false_example")
        else pure observation
      else check ci.type modules
end MedenSearch
