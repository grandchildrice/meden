import Terms

/-! Exercise the executable admission function, including untrusted trees that
the fixed parser would not produce. These are runtime guards, not proof roots. -/
open Lean MedenTerms
namespace ContractSyntaxTest

private def atom (text : String) := Syntax.atom .none text
private def node (kind : Name) (args : Array Syntax := #[]) := Syntax.node .none kind args
private def accepts (stx : Syntax) (fuel := 128) : Bool :=
  match checkSyntax [`s, `s.value] [`value] fuel stx with
  | .ok () => true
  | .error _ => false

#guard accepts (mkIdent `s)
#guard !accepts (mkIdent `IO.println)
#guard !accepts (node `null #[mkIdent `IO.println])
#guard !accepts (node `Lean.Parser.Term.byTactic)
#guard !accepts (node `Lean.Parser.Term.structInstField #[atom "value"])
#guard accepts (node `Lean.Parser.Term.structInstField #[atom "value", atom ":=", atom "0"])
#guard !accepts (node `Lean.Parser.Term.optEllipsis #[atom ".."])
#guard accepts (node `Lean.Parser.Term.optEllipsis)
#guard accepts (node `Lean.Parser.Term.structInstLVal #[mkIdent `value, node `null])
#guard !accepts (node `Lean.Parser.Term.structInstLVal #[mkIdent `other, node `null])
#guard !accepts (node `Lean.Parser.Term.structInstLVal #[mkIdent `value, node `null #[atom "bad"]])
#guard !accepts (node `Lean.Parser.Term.structInstLVal #[mkIdent `value.bad, node `null])
#guard !accepts .missing
#guard !accepts (atom "0") 0
#guard !accepts (node `null #[atom "0"]) 1
#guard accepts (node `null #[atom "0"]) 2

-- Opaque metadata is an explicit dependency on the trusted parser. It is not
-- recursively inspected or classified as an executable expression subtree.
#guard accepts (node `hygieneInfo #[mkIdent `IO.println])

end ContractSyntaxTest
