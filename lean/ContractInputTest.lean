import Terms

/-! Native checks of the real parser and JSON entry point. Contract syntax
admission does not itself prove types, output bounds or requested behavior. -/
open Lean MedenTerms
namespace ContractInputTest

private def contract (text : String := "s") (properties : Array Json := #[])
    (fields : Array Json := #[.str "value"]) (parameters : Array Json := #[.str "amount"]) : Json :=
  Json.mkObj [("fields", .arr fields), ("parameters", .arr parameters),
    ("specification", .str text), ("properties", .arr properties)]

private def property (statement : Json) : Json :=
  Json.mkObj [("name", .str "amount"), ("statement", statement)]

private def ok {α : Type} : Except String α → Bool
  | .ok _ => true
  | .error _ => false

private unsafe def tests : IO Unit := do
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let env ← importModules #[{module := `Lean.Elab.Tactic.Omega}] {} (loadExts := true)
  let bounded := String.ofList (List.replicate 8191 ' ') ++ "s"
  let oversized := " " ++ bounded
  let cases : List (String × Json × Bool) := [
    ("identity", contract, true),
    ("record and parameter", contract "{ value := s.value + amount }", true),
    ("property names", contract "s" #[property (.str "out.value >= s.value")], true),
    ("last property checked", contract "s" #[property (.str "True"),
      property (.str "by trivial")], false),
    ("spec has no out", contract "out", false),
    ("spec has no output projection", contract "out.value", false),
    ("spec has no transition", contract "transition s amount", false),
    ("label is not a free variable", contract "value", false),
    ("global function", contract "IO.println", false),
    ("tactic", contract "by exact s", false),
    ("trailing command", contract "s\n#eval IO.println \"bad\"", false),
    ("property statement checked", contract "s" #[property (.str "IO.println")], false),
    ("property statement type", contract "s" #[property .null], false),
    ("property name type", contract "s" #[Json.mkObj [("name", .null),
      ("statement", .str "True")]], false),
    ("all fields decoded", contract (fields := #[.str "value", .null]), false),
    ("all parameters decoded", contract (parameters := #[.str "amount", .null]), false),
    ("no undeclared parameter", contract "amount" (parameters := #[]), false),
    ("exact size bound", contract bounded, true),
    ("oversized text", contract oversized, false),
    ("oversized property", contract "s" #[property (.str oversized)], false),
    ("missing metadata", Json.mkObj [], false),
    ("missing properties", Json.mkObj [("fields", .arr #[.str "value"]),
      ("parameters", .arr #[]), ("specification", .str "s")], false)
  ]
  for (label, json, expected) in cases do
    unless ok (checkContract env json) == expected do
      throw (IO.userError s!"contract input assertion failed: {label}")
  let ordered := contract (fields := #[.str "second", .str "first", .str "second"])
  unless (match stringList ordered "fields" with
    | .ok values => values == ["second", "first", "second"]
    | .error _ => false) do
    throw (IO.userError "contract input assertion failed: exact field order")
  unless !(names ["value"] ["amount"] false).contains `out.value &&
      (names ["value"] ["amount"] true).contains `out.value do
    throw (IO.userError "contract input assertion failed: property-only output name")
  IO.println s!"{cases.length + 2} contract input assertions passed"

#eval tests

end ContractInputTest
