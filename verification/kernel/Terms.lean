import Lean.Elab.Tactic.Omega
import Lean

/-! Trusted input adapter, run before compiling contract terms. Candidate
input is parsed as data; it cannot introduce declarations, tactics, imports,
metaprograms or new syntax. This adapter is not an implementation theorem. -/
open Lean

private def allowedKinds : List String := [
  "null", "num", "termIfThenElse", "Lean.Parser.Term.ifThenElse",
  "Lean.Parser.Term.app", "Lean.Parser.Term.paren",
  "Lean.Parser.Term.typeAscription", "Lean.Parser.Term.hygienicLParen",
  "Lean.Parser.Term.structInst", "Lean.Parser.Term.structInstFields",
  "Lean.Parser.Term.structInstField", "Lean.Parser.Term.structInstLVal",
  "Lean.Parser.Term.structInstFieldDef", "Lean.Parser.Term.optEllipsis",
  "Lean.Parser.Term.arrow", "term_+_", "term_-_", "term_=_", "term_≠_",
  "term_<_", "term_≤_", "term_>_", "term_≥_", "term_∧_", "term_∨_",
  "term¬_", "term_&&_", "term_||_", "term_==_", "term_!=_", "term!_"
]

private def hasAtom (token : String) : Nat → Syntax → Bool
  | 0, _ => false
  | fuel + 1, stx =>
    (stx.isAtom && stx.getAtomVal == token) || stx.getArgs.any (hasAtom token fuel)

private def checkSyntax (allowed fields : List Name) : Nat → Syntax → Except String Unit
  | 0, _ => .error "mathematical term nesting exceeds the supported budget"
  | fuel + 1, stx => do
    match stx with
    | .missing => throw "missing mathematical term"
    | .atom _ _ => pure ()
    | .ident _ _ n _ =>
      unless allowed.contains n do throw s!"unsupported mathematical identifier: {n}"
    | .node _ kind args =>
      -- Parenthesis hygiene metadata is created by the trusted parser.
      if kind == `hygieneInfo then return
      -- Labels are not expression variables or global function names. Field
      -- punning would turn a label into a free expression, so require :=.
      if kind == `Lean.Parser.Term.structInstField && !hasAtom ":=" 128 stx then
        throw "record fields must have explicit := values"
      if kind == `Lean.Parser.Term.optEllipsis && hasAtom ".." 128 stx then
        throw "record field defaults must be explicit"
      unless allowedKinds.contains (kind.toString false) do
        throw s!"unsupported mathematical syntax: {kind}; commands and tactics are not contract data"
      if kind == `Lean.Parser.Term.structInstLVal then
        match args.toList with
        | [.ident _ _ n _, .node _ k tail] =>
          unless k == `null && tail.isEmpty do
            throw "record labels cannot contain projections or indices"
          unless fields.contains n do throw s!"unknown state field: {n}"
          return
        | _ => throw "record labels must be single declared state fields"
      for arg in args do checkSyntax allowed fields fuel arg

private def names (fields parameters : List String) (property : Bool) : List Name :=
  let base := [`s, `Nat, `Bool, `True, `False, `true, `false, `Nat.min, `Nat.max, `min, `max]
  let out := if property then [`out, `transition] else []
  base ++ out ++ parameters.map Name.mkSimple ++
    fields.map (Name.str `s) ++ (if property then fields.map (Name.str `out) else [])

private def checkTerm (env : Environment) (allowed fields : List Name) (text : String) : Except String Unit := do
  if text.utf8ByteSize > 8192 then throw "mathematical term exceeds 8 KiB"
  let stx ← Parser.runParserCategory env `term text "contract"
  checkSyntax allowed fields 128 stx

private def stringList (json : Json) (field : String) : Except String (List String) := do
  let values ← (← json.getObjVal? field).getArr?
  values.toList.mapM Json.getStr?

private def checkContract (env : Environment) (json : Json) : Except String Unit := do
  let fields ← stringList json "fields"
  let parameters ← stringList json "parameters"
  checkTerm env (names fields parameters false) (fields.map Name.mkSimple)
    (← json.getObjValAs? String "specification")
  for property in (← (← json.getObjVal? "properties").getArr?) do
    checkTerm env (names fields parameters true) (fields.map Name.mkSimple)
      (← property.getObjValAs? String "statement")

-- Initializer execution is needed only for the fixed imported parser tables.
-- The environment imports no candidate module, extension or native library.
private unsafe def run (args : List String) : IO UInt32 := do
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let env ← importModules #[{module := `Lean.Elab.Tactic.Omega}] {} (loadExts := true)
  let bytes ← IO.FS.readBinFile (args.headD "contract.json")
  if bytes.size > 65536 then
    IO.eprintln "contract exceeds 64 KiB"
    return 65
  match String.fromUTF8? bytes with
  | none => IO.eprintln "contract is not UTF-8"; return 65
  | some text =>
    match Json.parse text >>= checkContract env with
    | .error error => IO.eprintln error; return 65
    | .ok () => return 0

unsafe def main (args : List String) : IO UInt32 := do
  try run args catch e => IO.eprintln s!"contract parser unavailable: {e}"; return 69
