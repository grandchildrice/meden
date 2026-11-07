import Lean.Data.Json
import MedenSpec

/-!
# meden-spec

Executable oracle for the conformance test. Reads one JSON document from
stdin:

```json
{"policy_cases": [{"policy": {"name": "p", "grants": [{"capability": "fs.read", "scope": "/a/*"}],
                               "denies": ["shell.exec"]},
                    "requirements": [{"capability": "fs.read", "scope": "/a/b"}]}],
 "glob_cases": [{"pattern": "a*", "text": "ab"}],
 "tier_cases": [{"result": "proved", "tcb": ["lean4"], "backend": "lean4", "approved": []}]}
```

and prints `{"policy": [bool], "glob": [bool], "tier": [string]}` computed by
the definitions in `MedenSpec`.
-/

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

structure JArg where
  control : Bool
  sources : List String
  readers : Option (List String) := none
  deriving FromJson

structure FlowCase where
  trusted : List String
  principal : String
  args : List JArg
  deriving FromJson

structure TierCase where
  result : String
  tcb : List String
  backend : String
  approved : List String
  deriving FromJson

structure Input where
  policy_cases : List PolicyCase := []
  glob_cases : List GlobCase := []
  tier_cases : List TierCase := []
  flow_cases : List FlowCase := []
  deriving FromJson

def JPolicy.toPolicy (p : JPolicy) : Policy :=
  { name := p.name
    grants := p.grants.map fun g => { cap := g.capability, scope := g.scope }
    denies := p.denies }

def vresultOf? : String → Option VResult
  | "proved" => some .proved
  | "tested" => some .tested
  | "scanned" => some .scanned
  | "failed" => some .failed
  | _ => none

def MedenSpec.Tier.name : Tier → String
  | .verified => "verified"
  | .verifiedWithTcb => "verified-with-tcb"
  | .notVerified => "not-verified"

def runPolicy (c : PolicyCase) : Bool :=
  c.policy.toPolicy.allowed (c.requirements.map fun r => { cap := r.capability, scope := r.scope })

def runGlob (c : GlobCase) : Bool :=
  globMatch c.pattern.toList c.text.toList

def sourceOf (s : String) : Source :=
  match s with
  | "runtime" => .runtime
  | "user" => .user
  | "assistant" => .assistant
  | _ => .tool (if s.startsWith "tool:" then (s.drop 5).toString else s)

def readersOf : Option (List String) → Readers
  | none => .anyone
  | some ps => .only ps

def argOf (a : JArg) : Arg :=
  { control := a.control
    label := { sources := a.sources.map sourceOf, readers := readersOf a.readers } }

def runIntegrity (c : FlowCase) : Bool :=
  integrityOk (c.trusted.map sourceOf) (c.args.map argOf)

def runDisclosure (c : FlowCase) : Bool :=
  disclosureOk c.principal (c.args.map argOf)

def runTier (c : TierCase) : String :=
  match vresultOf? c.result with
  | some r => (tierOf r c.tcb c.backend c.approved).name
  | none => "bad-result"

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
      , ("tier", toJson (input.tier_cases.map runTier))
      , ("integrity", toJson (input.flow_cases.map runIntegrity))
      , ("disclosure", toJson (input.flow_cases.map runDisclosure)) ]
    IO.println out.compress
    return 0
