import Lean.Data.Json
import MedenSpec

/-! Executable policy and glob oracle for Rust conformance tests. -/

open Lean MedenSpec

instance : FromJson Capability where
  fromJson? j := do
    let s ← j.getStr?
    match Capability.ofString? s with
    | some c => pure c
    | none => throw s!"unknown capability {s}"

structure JGrant where
  capability : Capability
  scope : Option String := none
  deriving FromJson

structure JRequirement where
  capability : Capability
  scope : Option String := none
  deriving FromJson

structure JPolicy where
  name : String := ""
  grants : List JGrant := []
  denies : List Capability := []
  deriving FromJson

structure PolicyCase where
  policy : JPolicy
  requirements : List JRequirement
  deriving FromJson

structure GlobCase where
  pattern : String
  text : String
  deriving FromJson

deriving instance FromJson, ToJson for Transfer.Request
deriving instance FromJson, ToJson for Transfer.Permit

structure TransferCase where
  now : Nat
  session : List Nat
  permit : Transfer.Permit
  request : Transfer.Request
  deriving FromJson

structure Input where
  policy_cases : List PolicyCase := []
  glob_cases : List GlobCase := []
  transfer_cases : List TransferCase := []
  deriving FromJson

def JPolicy.toPolicy (p : JPolicy) : Policy :=
  { name := p.name
    grants := p.grants.map fun g => { cap := g.capability, scope := g.scope }
    denies := p.denies }

def runPolicy (c : PolicyCase) : Bool :=
  c.policy.toPolicy.allowed (c.requirements.map fun r => { cap := r.capability, scope := r.scope })

def runGlob (c : GlobCase) : Bool :=
  globMatch c.pattern.toList c.text.toList

def runTransfer (c : TransferCase) : Bool :=
  Transfer.allowed c.now c.session c.permit c.request

partial def readAll (s : IO.FS.Stream) (acc : String) : IO String := do
  let line ← s.getLine
  if line.isEmpty then pure acc else readAll s (acc ++ line)

def main : IO UInt32 := do
  let stdin ← IO.getStdin
  let text ← readAll stdin ""
  match Json.parse text >>= fromJson? (α := Input) with
  | .error e =>
    IO.eprintln s!"meden-spec: {e}"
    return 1
  | .ok input =>
    let out : Json := Json.mkObj
      [ ("policy", toJson (input.policy_cases.map runPolicy))
      , ("glob", toJson (input.glob_cases.map runGlob))
      , ("transfer", toJson (input.transfer_cases.map runTransfer)) ]
    IO.println out.compress
    return 0
