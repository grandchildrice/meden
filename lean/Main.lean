import Lean.Data.Json
import MedenSpec

/-! Executable policy, glob and processor lifecycle conformance oracle. -/

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

structure ProcessorBinding where
  is_local : Bool
  engine : List Nat
  weights : List Nat
  provider : List Nat
  endpoint : List Nat
  model : List Nat
  credential : Option (List Nat)
  deriving FromJson, ToJson

def ProcessorBinding.toBinding (b : ProcessorBinding) : Processor.Binding :=
  if b.is_local then .local ⟨b.engine, b.weights⟩
  else .http ⟨b.provider, b.endpoint, b.model, b.credential⟩

def ProcessorBinding.ofBinding : Processor.Binding → ProcessorBinding
  | .local p => ⟨true, p.engine, p.weights, [], [], [], none⟩
  | .http p => ⟨false, [], [], p.provider, p.endpoint, p.model, p.credential⟩

structure ProcessorAction where
  kind : String
  host : Bool
  now : Nat
  session : List Nat
  deriving FromJson

def ProcessorAction.toAction (a : ProcessorAction) : Processor.Action :=
  let o := if a.host then Processor.Origin.host else .model
  if a.kind == "approve" then .approve o a.now
  else if a.kind == "revoke" then .revoke o
  else .dispatch a.session a.now

structure ProcessorCase where
  session : List Nat
  binding : ProcessorBinding
  actions : List ProcessorAction
  deriving FromJson

structure ProcessorObservation where
  accepted : Bool
  effect : Option ProcessorBinding
  deriving ToJson

structure ProcessorResult where
  steps : List ProcessorObservation
  binding : ProcessorBinding
  phase : String
  expires : Nat
  deriving ToJson

structure Input where
  policy_cases : List PolicyCase := []
  glob_cases : List GlobCase := []
  processor_cases : List ProcessorCase := []
  deriving FromJson

def JPolicy.toPolicy (p : JPolicy) : Policy :=
  { name := p.name
    grants := p.grants.map fun g => { cap := g.capability, scope := g.scope }
    denies := p.denies }

def runPolicy (c : PolicyCase) : Bool :=
  c.policy.toPolicy.allowed (c.requirements.map fun r => { cap := r.capability, scope := r.scope })

def runGlob (c : GlobCase) : Bool :=
  globMatch c.pattern.toList c.text.toList

def runProcessor (c : ProcessorCase) : ProcessorResult :=
  let r := Processor.run (Processor.initial c.session c.binding.toBinding)
    (c.actions.map ProcessorAction.toAction)
  ⟨r.1.map (fun o => ⟨o.accepted, o.effect.map ProcessorBinding.ofBinding⟩),
    ProcessorBinding.ofBinding r.2.binding,
    match r.2.phase with
      | .pending => "pending" | .granted => "granted" | .revoked => "revoked",
    r.2.expires⟩

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
      , ("processor", toJson (input.processor_cases.map runProcessor)) ]
    IO.println out.compress
    return 0
