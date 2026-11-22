import LeanAtlas.GraphData.Core
import LeanAtlas.GraphData.Json
import MedenSemantic
import MedenSearch
import MedenInterchange

open Lean LeanAtlas.Config LeanAtlas.GraphData.Core

private def pretty (env : Environment) (expr : Expr) : IO String := do
  let ctx : PPContext := { env := env, opts := ({} : Options).setBool `pp.fullNames true }
  let value ← ctx.runMetaM <| Lean.Meta.ppExpr expr
  let text := value.pretty 100
  if text.utf8ByteSize > 65536 then throw <| IO.userError "declaration exceeds 64 KiB"
  return text

unsafe def main (args : List String) : IO UInt32 := do
  Lean.initSearchPath (← Lean.findSysroot)
  Lean.enableInitializersExecution
  let modules := args.toArray.map fun arg => stringToName (arg.replace "/" ".")
  let env ← Lean.importModules (#[{module := `MedenSemantic}, {module := `MedenSearch}] ++ modules.map fun name => { module := name }) {} (trustLevel := 0) (loadExts := true)
  let scope : ProjectScope := { projectNamespace := .anonymous }
  let mut nodes : Array NodeData := #[]
  -- Read the selected modules' declaration inventories directly. Scanning every
  -- imported Mathlib constant adds work unrelated to the requested graph.
  for i in [:env.header.moduleData.size] do
    if modules.contains env.header.moduleNames[i]! then
      let data := env.header.moduleData[i]!
      for name in data.constNames ++ data.extraConstNames do
        if (getModuleFor? env name).any modules.contains && LeanAtlas.Deps.Core.isRelevantConstant scope env name then
          if nodes.size >= 5000 then throw <| IO.userError "Atlas exceeds 5000 declarations"
          nodes := nodes.push (buildNodeData scope env name)
  nodes := nodes.qsort (·.id.toString < ·.id.toString)
  let names := nodes.foldl (fun set node => set.insert node.id) ({} : NameSet)
  let mut external : NameSet := {}
  let mut rows : Array Json := #[]
  let mut localNodes : Array NodeData := #[]
  for node in nodes do
    let some ci := env.find? node.id | throw <| IO.userError "declaration disappeared"
    let statement ← pretty env ci.type
    let value ← match ci with
      | .defnInfo definition => do
          let expression ← pretty env definition.value
          pure (Json.str expression)
      | _ => pure Json.null
    let (axioms, standardAxioms) := collectAxiomsPartition env node.id
    let row := LeanAtlas.GraphData.Json.nodeDataToJson node
    let row := row.setObjVal! "statement" (.str statement)
    let row := row.setObjVal! "value" value
    let ctx : PPContext := { env := env, opts := {} }
    let checker ← ctx.runMetaM <| MedenSearch.analyze ci modules
    let semantic ← ctx.runMetaM <| MedenSemantic.describe ci checker
    let intermediate ← try
      let text ← ctx.runMetaM <| MedenInterchange.declaration ci
      pure <| if text.utf8ByteSize <= 60000 then Json.str text else Json.null
      catch _ => pure Json.null
    let semantic := if intermediate == Json.null then semantic else semantic.setObjVal! "intermediate" intermediate
    let row := row.setObjVal! "semantic" semantic
    let row := row.setObjVal! "axioms" (toJson (axioms.map Name.toString))
    let row := row.setObjVal! "standardAxioms" (toJson (standardAxioms.map Name.toString))
    let row := row.setObjVal! "externalDependencies" (toJson ((node.dependencies.filter (fun name => !names.contains name)).map Name.toString))
    rows := rows.push row
    for dependency in node.dependenciesWithKind do
      if !names.contains dependency.name then external := external.insert dependency.name
    localNodes := localNodes.push { node with
      dependencies := node.dependencies.filter names.contains
      dependenciesWithKind := node.dependenciesWithKind.filter (fun dependency => names.contains dependency.name) }
  let edges := buildEdges localNodes
  let result := Json.mkObj [
    ("metadata", Json.mkObj [("projectName", .str "Selected Lean modules"), ("leanVersion", .str Lean.versionString)]),
    ("nodes", .arr rows),
    ("edges", .arr (edges.map LeanAtlas.GraphData.Json.edgeDataToJson)),
    ("externalReferences", toJson (external.toList.map Name.toString))]
  IO.println result.compress
  return 0
