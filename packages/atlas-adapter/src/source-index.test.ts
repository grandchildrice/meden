import { describe, expect, it } from "vitest";
import { maskSource, sourceIndex } from "./source-index";

const snapshot = (files: Record<string, string>) => ({
  revision: "sha256:" + "a".repeat(64),
  files,
});
describe("live source reference index", () => {
  it("finds exact namespace references across selected files, never proof evidence from source text", () => {
    const graph = sourceIndex(
      snapshot({
        "Spec.lean":
          "namespace Machine\ndef step (s : Nat) := s - 1\nend Machine\n",
        "Check.lean":
          "namespace Proof\n-- audit: proved\ntheorem bound (s : Nat) : Machine.step s ≤ s := by\n  sorry\nend Proof\n",
      }),
    );
    const bound = graph.declarations.find(
      (item) => item.name === "Proof.bound",
    )!;
    const step = graph.declarations.find(
      (item) => item.name === "Machine.step",
    )!;
    expect(bound.source).toEqual({ file: "Check.lean", line: 3 });
    expect(graph.edges).toEqual([
      { source: bound.id, target: step.id, site: "unknown" },
    ]);
    for (const declaration of graph.declarations) {
      expect(declaration.proofStatus).toBe("unknown");
      expect(declaration.evidenceRefs).toEqual([]);
    }
  });
  it("ignores fake declarations/references in nested comments and string literals, preserving Unicode line positions", () => {
    const source =
      'namespace N\n/- 外側\n/- theorem fake : True := real -/\n-/\ndef real := 1\ndef text := "real theorem hidden"\n-- def ghost := real\ntheorem α : real = 1 := by rfl\nend N\n';
    const graph = sourceIndex(snapshot({ "Main.lean": source }));
    expect(graph.declarations.map((item) => item.name)).toEqual([
      "N.real",
      "N.text",
      "N.α",
    ]);
    expect(graph.declarations[2]!.source!.line).toBe(8);
    expect(graph.edges).toHaveLength(1);
    const masked = maskSource(source, "lean");
    expect(masked.length).toBe(source.length);
    expect(masked.split("\n").length).toBe(source.split("\n").length);
  });
  it("keeps nested namespaces and private definitions local; refuses ambiguous cross-file guesses", () => {
    const graph = sourceIndex(
      snapshot({
        "A.lean":
          "namespace A\ndef x := 1\nnamespace B\nprivate def x := 2\ndef chosen := x\nend B\ndef outer := x\nend A\n",
        "B.lean": "def x := 3\ndef unknown := chosen\n",
        "C.lean": "def x := 4\n",
        "D.lean": "def ambiguous := x\n",
      }),
    );
    const named = (name: string) =>
      graph.declarations.find((item) => item.name === name)!;
    expect(graph.edges).toContainEqual({
      source: named("A.B.chosen").id,
      target: named("A.B.x").id,
      site: "unknown",
    });
    expect(graph.edges).toContainEqual({
      source: named("A.outer").id,
      target: named("A.x").id,
      site: "unknown",
    });
    expect(
      graph.edges.some((edge) => edge.source === named("unknown").id),
    ).toBe(false);
    expect(
      graph.edges.some((edge) => edge.source === named("ambiguous").id),
    ).toBe(false);
  });
  it("indexes Rust functions and types, excluding comments/raw-string references", () => {
    const graph = sourceIndex(
      snapshot({
        "src/kernel.rs":
          'pub struct State { pub stock: u64 }\npub fn step(s: State) -> State { s }\npub fn label() { let _ = r##"State step\nfn fake() {}"##; }\n',
      }),
    );
    expect(graph.declarations.map((item) => item.name)).toEqual([
      "State",
      "step",
      "label",
    ]);
    expect(graph.edges).toEqual([
      {
        source: graph.declarations[1]!.id,
        target: graph.declarations[0]!.id,
        site: "unknown",
      },
    ]);
  });
});
