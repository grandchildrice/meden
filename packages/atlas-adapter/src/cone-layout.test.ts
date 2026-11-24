import { expect, test } from "vitest";
import fc from "fast-check";
import { coneLayout, importAtlas } from "./index";
import golden from "./fixtures/cone-layout-v1.json";
import { z } from "zod";
import { workspaceSchema } from "@meden/workspace-schema";

const graphSchema = z.object({
  name: workspaceSchema.shape.name,
  declarations: workspaceSchema.shape.declarations,
  edges: workspaceSchema.shape.edges,
});
const inherited = Object.getOwnPropertyNames(Object.prototype);
const names = [
  ...inherited,
  "A",
  "B",
  "C",
  "0",
  "1",
  "10",
  "01",
  "α",
  "🦊",
  "é",
  "é",
];
function graph(
  ids: string[],
  edges: [number, number][],
  groups = ["Source.lean"],
) {
  return importAtlas({
    metadata: { projectName: "Original identifiers" },
    nodes: ids.map((id, i) => ({
      id,
      shortName: id,
      kind: "theorem",
      filePath: groups[i % groups.length],
      hasSorry: false,
    })),
    edges: edges.map(([a, b]) => ({
      source: ids[a % ids.length],
      target: ids[b % ids.length],
    })),
  });
}

test("ordinary cone coordinates retain the pre-change L4SB golden", () => {
  for (const data of golden.cases) {
    const workspace = graphSchema.parse(data.workspace);
    for (const original of data.roots) {
      const positions = coneLayout(workspace, original.root);
      expect([...positions.keys()]).toEqual(
        original.positions.map(([id]) => id),
      );
      for (const [id, position] of original.positions as Array<
        [
          string,
          { x: number; y: number; width: number; depth: number; side: string },
        ]
      >) {
        const next = positions.get(id)!;
        expect(next.depth).toBe(position.depth);
        expect(next.side).toBe(position.side);
        for (const field of ["x", "y", "width"] as const)
          expect(next[field]).toBeCloseTo(position[field], 12);
      }
    }
  }
});

test.each(inherited)(
  "valid declaration %s works as root, dependency, user and cycle",
  (id) => {
    const before = Object.getOwnPropertyDescriptors(Object.prototype);
    const workspace = graph(
      [id, "dependency", "user"],
      [
        [0, 1],
        [2, 0],
        [1, 0],
      ],
    );
    const positions = coneLayout(workspace, id);
    expect(new Set(positions.keys())).toEqual(
      new Set([id, "dependency", "user"]),
    );
    expect(positions.get(id)).toMatchObject({
      x: 0,
      y: 0,
      depth: 0,
      side: "root",
    });
    expect(coneLayout(graph([id], [[0, 0]]), id).size).toBe(1);
    for (const root of ["dependency", "user"])
      expect(coneLayout(workspace, root).has(id)).toBe(true);
    expect(coneLayout(workspace, "missing").size).toBe(0);
    expect(
      workspace.declarations.every((d) => d.proofStatus === "unknown"),
    ).toBe(true);
    expect(Object.getOwnPropertyDescriptors(Object.prototype)).toEqual(before);
  },
);

test("finite identifier and cycle corpora preserve input, scope and repeated coordinates", () => {
  fc.assert(
    fc.property(
      fc.record({
        ids: fc.uniqueArray(fc.constantFrom(...names), {
          minLength: 1,
          maxLength: 14,
        }),
        edges: fc.array(fc.tuple(fc.nat(100), fc.nat(100)), { maxLength: 60 }),
        groups: fc.array(
          fc.constantFrom("A.lean", "B.lean", "__proto__", "λ.lean"),
          { minLength: 1, maxLength: 5 },
        ),
      }),
      (data) => {
        const workspace = graph(data.ids, data.edges, data.groups),
          original = JSON.stringify(workspace);
        for (const root of data.ids) {
          const positions = coneLayout(workspace, root);
          const expected = new Set([root]);
          for (const edge of workspace.edges) {
            if (edge.source === root) expected.add(edge.target);
            if (edge.target === root) expected.add(edge.source);
          }
          expect(positions.has(root)).toBe(true);
          expect([...positions.keys()].every((id) => expected.has(id))).toBe(
            true,
          );
          expect([...coneLayout(workspace, root)]).toEqual([...positions]);
          for (const p of positions.values())
            expect(
              Number.isFinite(p.x) && Number.isFinite(p.y) && p.width > 0,
            ).toBe(true);
        }
        expect(JSON.stringify(workspace)).toBe(original);
      },
    ),
    { numRuns: Number(process.env.MEDEN_FUZZ_RUNS || 200) },
  );
});
