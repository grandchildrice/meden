import Lean
import MedenSearch

open Lean

unsafe def main (args : List String) : IO UInt32 := do
  let settings :: target :: roots := args | throw <| IO.userError "Missing observation target"
  let configuration ← IO.ofExcept (Json.parse settings)
  let seed ← IO.ofExcept (configuration.getObjValAs? Nat "seed")
  Lean.initSearchPath (← Lean.findSysroot)
  Lean.enableInitializersExecution
  let modules := roots.toArray.map fun arg =>
    (arg.replace "/" ".").splitOn "." |>.foldl (fun n part => Name.str n part) .anonymous
  let env ← Lean.importModules (#[{module := `MedenSearch}] ++ modules.map fun name => {module := name}) {}
    (trustLevel := 0) (loadExts := true)
  for i in [:env.header.moduleData.size] do
    if modules.contains env.header.moduleNames[i]! then
      for name in env.header.moduleData[i]!.constNames do
        if name.toString == target then
          let some ci := env.find? name | throw <| IO.userError "Observation target disappeared"
          let context : PPContext := {env, opts := {}}
          let result ← context.runMetaM <| MedenSearch.analyze ci modules seed
          IO.println result.compress
          return 0
  throw <| IO.userError "Observation target is outside selected modules"
