import { describe, expect, it } from "vitest";
import { meaningDependencies, importAtlas } from "./index";
import { sampleWorkspace } from "../../../apps/web/src/fixtures/ledger";

describe("Atlas boundary and dependency traversal", () => {
  it("imports complete/no-sorry metadata without fabricating proof evidence", () => {
    const graph = importAtlas({
      metadata: { projectName: "Project" },
      nodes: [
        {
          id: "A",
          shortName: "A",
          kind: "theorem",
          filePath: "A.lean",
          hasSorry: false,
          meta: {
            proofProgress: "complete",
            summary: "An explanatory summary",
          },
        },
      ],
      edges: [],
    });
    expect(graph.declarations[0].proofStatus).toBe("unknown");
    expect(graph.declarations[0].evidenceRefs).toEqual([]);
    expect(graph.declarations[0].statement).toBe("");
    expect(graph.declarations[0].summary).toBe("An explanatory summary");
  });
  it("rejects missing declaration targets and duplicate nodes", () => {
    const node = {
      id: "A",
      shortName: "A",
      kind: "theorem",
      filePath: "A.lean",
      hasSorry: false,
      meta: {},
    };
    expect(() =>
      importAtlas({
        metadata: { projectName: "P" },
        nodes: [node],
        edges: [{ source: "A", target: "B" }],
      }),
    ).toThrow();
    expect(() =>
      importAtlas({
        metadata: { projectName: "P" },
        nodes: [node, node],
        edges: [],
      }),
    ).toThrow();
  });
  it("keeps future edge kinds in meaning scope even when they resemble a known value edge", () => {
    const graph = importAtlas({
      metadata: { projectName: "future" },
      nodes: [
        {
          id: "A",
          shortName: "A",
          kind: "theorem",
          filePath: "A.lean",
          hasSorry: false,
        },
        {
          id: "B",
          shortName: "B",
          kind: "definition",
          filePath: "A.lean",
          hasSorry: false,
        },
      ],
      edges: [
        {
          source: "A",
          target: "B",
          kind: "future_theorem_value_to_definition",
        },
      ],
    });
    expect(graph.edges[0]?.site).toBe("unknown");
    expect(meaningDependencies(graph, "A").has("B")).toBe(true);
  });
  it("prunes proof-value edges while retaining value dependencies from definitions", () => {
    const workspace = sampleWorkspace();
    const meaning = meaningDependencies(workspace, "Ledger.accepted_transfer");
    expect(meaning.has("Nat.add_sub_cancel")).toBe(false);
    expect(meaning.has("Ledger.u64Max")).toBe(true);
  });
  it("retains unclassified dependencies conservatively and terminates on cycles", () => {
    const workspace = sampleWorkspace();
    workspace.edges.push({
      source: "Ledger.State",
      target: "Ledger.accepted_transfer",
      site: "unknown",
    });
    expect(
      meaningDependencies(workspace, "Ledger.accepted_transfer").has(
        "Ledger.State",
      ),
    ).toBe(true);
  });
});

it("places dependencies above and users below with the actual L4SB cone algorithm", async () => {
  const { coneLayout } = await import("./index");
  const workspace = sampleWorkspace();
  const positions = coneLayout(workspace, "Ledger.accepted_transfer");
  expect(positions.get("Ledger.State")!.y).toBeLessThan(0);
  workspace.edges.push({
    source: "Ledger.retry_unchanged",
    target: "Ledger.accepted_transfer",
    site: "value",
  });
  const related = coneLayout(workspace, "Ledger.accepted_transfer");
  // A cycle is permitted in displayed source references; layout still terminates.
  expect(
    [...related.values()].every(
      (p) => Number.isFinite(p.x) && Number.isFinite(p.y),
    ),
  ).toBe(true);
  expect(related.size).toBeLessThanOrEqual(workspace.declarations.length);
});

it("separates human review, known proof-only dependencies and unknown paths without granting proof status", async () => {
  const { reviewScope } = await import("./index");
  const w = sampleWorkspace();
  const root = "Ledger.accepted_transfer";
  let scope = reviewScope(w, [root]);
  expect(scope.get(root)).toBe("human");
  expect(scope.get("Nat.add_sub_cancel")).toBe("proof-only");
  expect(scope.get("Ledger.u64Max")).toBe("human");
  w.edges.push({ source: root, target: "Nat.add_sub_cancel", site: "unknown" });
  scope = reviewScope(w, [root]);
  expect(scope.get("Nat.add_sub_cancel")).toBe("unknown");
  expect([...reviewScope(w, []).values()].every((v) => v === "unknown")).toBe(
    true,
  );
  expect(w.declarations.find((d) => d.id === root)?.proofStatus).toBe("proved");
});

it("keeps dense L4SB layers disjoint at their actual rendered widths and heights", async () => {
  const { coneLayout } = await import("./index");
  const example = sampleWorkspace().declarations[0]!;
  const declarations = Array.from({ length: 81 }, (_, i) => ({
    ...example,
    id: "node" + i,
    name: "node" + i,
  }));
  const edges = declarations
    .slice(1)
    .map((d) => ({ source: "node0", target: d.id, site: "type" as const }));
  const positions = [...coneLayout({ declarations, edges }, "node0").values()];
  expect(positions).toHaveLength(81);
  for (let i = 0; i < positions.length; i++)
    for (let j = i + 1; j < positions.length; j++) {
      const a = positions[i]!,
        b = positions[j]!;
      expect(
        Math.abs(a.x - b.x) >= (a.width + b.width) / 2 ||
          Math.abs(a.y - b.y) >= 74,
      ).toBe(true);
    }
});
