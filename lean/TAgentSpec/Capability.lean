/-!
# Capabilities and requirements

Mirrors `tagent_core::capability`.
-/
namespace TAgentSpec

/-- One kind of effect an action can have on the world. -/
inductive Capability where
  | fsRead | fsWrite | netEgress | netListen | shellExec | envRead
  | secretRead | processSpawn | packageInstall | clipboard | uiControl
  deriving DecidableEq, Repr, Inhabited

namespace Capability

/-- Every capability, in the order used by the Rust `Capability::ALL`. -/
def all : List Capability :=
  [fsRead, fsWrite, netEgress, netListen, shellExec, envRead,
   secretRead, processSpawn, packageInstall, clipboard, uiControl]

/-- Canonical name. -/
def toString : Capability → String
  | fsRead => "fs.read"
  | fsWrite => "fs.write"
  | netEgress => "net.egress"
  | netListen => "net.listen"
  | shellExec => "shell.exec"
  | envRead => "env.read"
  | secretRead => "secret.read"
  | processSpawn => "process.spawn"
  | packageInstall => "package.install"
  | clipboard => "clipboard"
  | uiControl => "ui.control"

/-- Parse a canonical name. -/
def ofString? : String → Option Capability
  | "fs.read" => some fsRead
  | "fs.write" => some fsWrite
  | "net.egress" => some netEgress
  | "net.listen" => some netListen
  | "shell.exec" => some shellExec
  | "env.read" => some envRead
  | "secret.read" => some secretRead
  | "process.spawn" => some processSpawn
  | "package.install" => some packageInstall
  | "clipboard" => some clipboard
  | "ui.control" => some uiControl
  | _ => none

instance : ToString Capability := ⟨toString⟩

theorem ofString_toString (c : Capability) : ofString? (toString c) = some c := by
  cases c <;> rfl

theorem toString_injective {a b : Capability} (h : toString a = toString b) : a = b := by
  have := congrArg ofString? h
  rw [ofString_toString, ofString_toString] at this
  exact Option.some.inj this

theorem all_complete (c : Capability) : c ∈ all := by
  cases c <;> simp [all]

end Capability

/-- One capability a skill or tool call needs, with an optional scope. -/
structure Requirement where
  cap : Capability
  scope : Option String
  deriving DecidableEq, Repr, Inhabited

end TAgentSpec
