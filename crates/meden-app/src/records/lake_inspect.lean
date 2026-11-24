import Lean
import Lean.Util.CollectAxioms

open Lean

/- This driver elaborates the selected source with Lake's original module setup.
   It never imports the selected module's previously compiled artifact. Executable
   project source, plugins and imported artifacts remain adapter trust boundaries. -/
unsafe def main (arguments : List String) : IO UInt32 := do
  let source :: setupFile :: names := arguments
    | throw <| IO.userError "Expected selected source, module setup and theorem names"
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let setup ← ModuleSetup.load setupFile
  let input ← IO.FS.readFile source
  let some environment ← Elab.runFrontend input {} source setup.name (setup? := some setup)
    | throw <| IO.userError "Current selected source did not elaborate successfully"
  let environment := environment.setExporting false
  let context : Core.Context := { fileName := source, fileMap := FileMap.ofString input }
  let state : Core.State := { env := environment }
  for value in names do
    let name := value.toName
    let some declaration := environment.find? name
      | throw <| IO.userError s!"Missing declaration: {name}"
    -- Module exports erase theorem bodies to axiom-shaped declarations. Use the
    -- original kind recorded by Lean's addDecl, as Lean's own #print does.
    unless wasOriginallyTheorem environment name do
      throw <| IO.userError s!"Not a theorem: {name}"
    let axioms ← (collectAxioms name : CoreM (Array Name)).toIO' context state
    for axiomName in axioms do
      unless #[`propext, `Classical.choice, `Quot.sound].contains axiomName do
        throw <| IO.userError s!"Unexpected axiom in {name}: {axiomName}"
    let type ← (Meta.ppExpr declaration.type).run' |>.toIO' context state
    IO.println s!"{name} : {type}\nAxioms: {axioms}"
  IO.println "MEDEN_THEOREMS_CHECKED"
  return 0
