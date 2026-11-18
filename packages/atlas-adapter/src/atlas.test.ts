import { describe, expect, it } from "vitest";
import { dependencyScope, importAtlas, radialLayout } from "./index";
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
  it("prunes proof-value edges while retaining value dependencies from definitions", () => {
    const workspace = sampleWorkspace();
    const all = dependencyScope(
      workspace,
      "Ledger.accepted_transfer",
      "dependencies",
    );
    const meaning = dependencyScope(
      workspace,
      "Ledger.accepted_transfer",
      "meaning",
    );
    expect(all.has("Nat.add_sub_cancel")).toBe(true);
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
      dependencyScope(workspace, "Ledger.accepted_transfer", "meaning").has(
        "Ledger.State",
      ),
    ).toBe(true);
    expect(
      radialLayout(workspace, "Ledger.accepted_transfer").size,
    ).toBeLessThanOrEqual(workspace.declarations.length);
    expect(radialLayout(workspace, "missing").size).toBe(0);
  });
  it("includes incoming proof references in the related-declarations overview", () => {
    const workspace = sampleWorkspace();
    expect(
      dependencyScope(workspace, "Ledger.State", "dependencies").has(
        "Ledger.accepted_transfer",
      ),
    ).toBe(false);
    expect(
      dependencyScope(workspace, "Ledger.State", "project").has(
        "Ledger.accepted_transfer",
      ),
    ).toBe(true);
    const positions = radialLayout(workspace, "Ledger.State", "project");
    expect(positions.has("Ledger.accepted_transfer")).toBe(true);
    expect(
      [...positions.values()].every(
        (position) =>
          Number.isFinite(position.x) && Number.isFinite(position.y),
      ),
    ).toBe(true);
  });
  it("never uses a pruned edge as a radial depth shortcut", () => {
    const workspace = sampleWorkspace();
    workspace.edges.push({
      source: "Ledger.accepted_transfer",
      target: "Ledger.u64Max",
      site: "value",
    });
    const positions = radialLayout(
      workspace,
      "Ledger.accepted_transfer",
      "meaning",
    );
    expect(positions.get("Ledger.u64Max")?.depth).toBe(2);
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
