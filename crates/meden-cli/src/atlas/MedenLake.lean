import Lake.Load.Package
import Lake.Config.LeanLib
import Lake.Config.LeanExe

open Lean Lake System

private def dependencyJson (dep : Dependency) : Json :=
  let (kind, path, subDir) := match dep.src? with
    | some (.path path copy) => (if copy then "copy" else "path", path.toString, "")
    | some (.git _ _ subDir) => ("git", "", subDir.map FilePath.toString |>.getD "")
    | none => ("registry", "", "")
  Json.mkObj [("name", .str (dep.name.toString (escape := false))), ("kind", .str kind),
    ("path", .str path), ("subdir", .str subDir),
    ("url", .str (match dep.src? with | some (.git url _ _) => url | _ => "")),
    ("revision", (match dep.src? with | some (.git _ rev _) => toJson (rev.map toString) | _ => Json.null)),
    ("options", Json.mkObj (dep.opts.toList.map fun (key, value) => (key.toString, .str value)))]

private def globJson (directory : FilePath) (glob : Lake.Glob) : Json :=
  let (kind, name) := match glob with
    | .one name => ("one", name)
    | .submodules name => ("submodules", name)
    | .andSubmodules name => ("andSubmodules", name)
  Json.mkObj [("directory", .str directory.toString), ("kind", .str kind),
    ("name", .str (if name.isAnonymous then "" else name.toString))]

/-- Load one frozen package only. Never materialize/update dependencies or run build hooks. -/
def main (args : List String) : IO UInt32 := do
  if let ["manifest", path] := args then
    let text ← IO.FS.readFile (FilePath.mk "/workspace" / path)
    let .ok json := Json.parse text | throw <| IO.userError "Invalid Lake manifest JSON"
    let .ok manifest := fromJson? (α := Lake.Manifest) json
      | throw <| IO.userError "Invalid or unsupported Lake manifest"
    IO.println (toJson manifest).compress
    return 0
  let ["package", directory, config, options] := args | throw <| IO.userError "Expected package/config/options"
  let lean ← LeanInstall.get "/analysis/lean"
  let lakeEnv : Lake.Env := {
    lean := lean
    lake := LakeInstall.ofLean lean
    elan? := none
    reservoirApiUrl := ""
    githashOverride := ""
    pkgUrlMap := {}
    noCache := true
    enableArtifactCache? := some false
    restoreAllArtifacts? := some false
    noSystemCache := true
    lakeConfig? := none
    cacheKey? := none
    cacheArtifactEndpoint? := none
    cacheRevisionEndpoint? := none
    cacheService? := none
    initLeanPath := []
    initLeanSrcPath := []
    initSharedLibPath := []
    initPath := []
    toolchain := Lean.toolchain
  }
  let .ok optionsJson := Json.parse options | throw <| IO.userError "Invalid options JSON"
  let .ok fields := optionsJson.getObj? | throw <| IO.userError "Expected options object"
  let mut lakeOpts : NameMap String := {}
  for (key, value) in fields.toList do
    let .ok value := value.getStr? | throw <| IO.userError "Expected string option"
    lakeOpts := lakeOpts.insert key.toName value
  let pkgDir := FilePath.mk "/workspace" / directory
  let some pkg ← (loadPackage {
      lakeEnv := lakeEnv
      wsDir := "/workspace"
      pkgDir := pkgDir
      relPkgDir := directory
      relConfigFile := config
      reconfigure := true
      updateDeps := false
      updateToolchain := false
      lakeOpts := lakeOpts
    }).toBaseIO
    | throw <| IO.userError "Offline Lake package configuration failed"
  let globs := (pkg.leanLibs.flatMap fun lib => lib.config.globs.map (globJson lib.config.srcDir)) ++
    pkg.leanExes.map (fun exe => globJson exe.config.srcDir (.one exe.config.root))
  let librarySources := pkg.leanLibs.map fun lib => Json.mkObj [
    ("directory", .str lib.config.srcDir.toString),
    ("selectors", toJson ((lib.config.roots.map Lake.Glob.andSubmodules ++ lib.config.globs).map (globJson lib.config.srcDir)))]
  let result := Json.mkObj [("name", .str (pkg.baseName.toString (escape := false))),
    ("src_dir", .str pkg.config.srcDir.toString),
    ("library_sources", toJson librarySources),
    ("executable_dirs", toJson (pkg.leanExes.map fun exe => exe.config.srcDir.toString)),
    ("globs", toJson globs), ("dependencies", toJson (pkg.depConfigs.map dependencyJson)),
    ("packages_dir", .str pkg.config.packagesDir.toString),
    ("manifest", .str pkg.relManifestFile.toString)]
  IO.println result.compress
  return 0
