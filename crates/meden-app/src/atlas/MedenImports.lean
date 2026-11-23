import Lean.Elab.Import

open Lean

private def ordinaryModule (name : String) : Bool :=
  (name.splitOn ".").all fun part =>
    !part.isEmpty && part.toList.zipIdx.all fun (c, i) =>
      c == '_' || ('a' ≤ c && c ≤ 'z') || ('A' ≤ c && c ≤ 'Z') ||
      (i > 0 && '0' ≤ c && c ≤ '9')

/-- Inspect only the import header; never elaborate or evaluate the observed source. -/
def main (args : List String) : IO UInt32 := do
  Lean.initSearchPath (← Lean.findSysroot)
  let mut rows : Array Json := #[]
  for file in args do
    let source ← IO.FS.readFile (System.FilePath.mk "/analysis/sources" / file)
    let (imports, _, messages) ← Lean.Elab.parseImports source (some file)
    if messages.hasErrors then
      throw <| IO.userError s!"Invalid Lean import header in {file}"
    if imports.size > 4096 then throw <| IO.userError "Import header exceeds 4096 modules"
    let mut modules : Array Json := #[]
    for entry in imports do
      let name := entry.module.toString
      if !ordinaryModule name then throw <| IO.userError "Ordinary import module names required"
      let external ← try
        let path ← Lean.findOLean entry.module
        pure (path.toString.startsWith "/analysis/lean/lib/lean/" ||
          path.toString.startsWith "/analysis/library/")
      catch _ => pure false
      modules := modules.push (Json.mkObj [("name", .str name), ("external", toJson external)])
    rows := rows.push (Json.mkObj [("file", .str file), ("modules", .arr modules)])
  IO.println (Json.arr rows).compress
  return 0
