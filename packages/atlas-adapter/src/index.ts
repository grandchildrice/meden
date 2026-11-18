import { z } from "zod";
import type { Declaration, Workspace } from "@meden/workspace-schema";

const atlasSchema = z.object({
  metadata: z.object({
    projectName: z.string(),
    leanVersion: z.string().optional(),
    generatedAt: z.string().optional(),
  }),
  nodes: z
    .array(
      z.object({
        id: z.string().min(1),
        shortName: z.string(),
        kind: z.string(),
        filePath: z.string(),
        lineStart: z.number().int().positive().nullable().optional(),
        hasSorry: z.boolean(),
        meta: z
          .object({
            summary: z.string().nullable().optional(),
            axioms: z.array(z.string()).nullable().optional(),
          })
          .passthrough(),
      }),
    )
    .max(20_000),
  edges: z
    .array(
      z.object({
        source: z.string(),
        target: z.string(),
        kind: z.string().optional(),
      }),
    )
    .max(100_000),
});

/** Structural import. Atlas metadata alone cannot establish a fresh proof receipt. */
export function importAtlas(
  input: unknown,
): Pick<Workspace, "name" | "declarations" | "edges"> {
  const graph = atlasSchema.parse(input);
  const ids = new Set(graph.nodes.map((node) => node.id));
  if (ids.size !== graph.nodes.length)
    throw new Error("宣言のIDが重複しています");
  if (
    graph.edges.some((edge) => !ids.has(edge.source) || !ids.has(edge.target))
  )
    throw new Error("依存先の宣言がありません");
  const declarations: Declaration[] = graph.nodes.map((node) => ({
    id: node.id,
    name: node.id,
    label: node.shortName,
    kind:
      node.kind === "theorem"
        ? "theorem"
        : node.kind === "axiom"
          ? "axiom"
          : "definition",
    statement: "",
    summary: node.meta.summary || undefined,
    source: {
      file: node.filePath,
      ...(node.lineStart ? { line: node.lineStart } : {}),
    },
    proofStatus: node.hasSorry ? "unproved" : "unknown",
    evidenceRefs: [],
    axioms: node.meta.axioms || [],
  }));
  return {
    name: graph.metadata.projectName,
    declarations,
    edges: graph.edges.map((edge) => ({
      source: edge.source,
      target: edge.target,
      site: edge.kind?.includes("_type_to_")
        ? "type"
        : edge.kind?.includes("_value_to_")
          ? "value"
          : "unknown",
    })),
  };
}

export type GraphScope = "dependencies" | "meaning" | "project";
/** Match Atlas' edge distinction; unclassified edges are retained conservatively. */
export function dependencyScope(
  workspace: Pick<Workspace, "declarations" | "edges">,
  root: string,
  scope: GraphScope,
): Set<string> {
  const nodes = new Map(workspace.declarations.map((node) => [node.id, node]));
  const edges = new Map<string, Workspace["edges"]>();
  for (const edge of workspace.edges) {
    if (
      scope === "meaning" &&
      nodes.get(edge.source)?.kind === "theorem" &&
      edge.site === "value"
    )
      continue;
    const outgoing = edges.get(edge.source) || [];
    outgoing.push(edge);
    edges.set(edge.source, outgoing);
    if (scope === "project")
      edges.set(edge.target, [
        ...(edges.get(edge.target) || []),
        { ...edge, target: edge.source },
      ]);
  }
  const seen = new Set<string>();
  const pending = nodes.has(root) ? [root] : [];
  while (pending.length) {
    const id = pending.pop()!;
    if (seen.has(id)) continue;
    seen.add(id);
    for (const edge of edges.get(id) || [])
      if (!seen.has(edge.target)) pending.push(edge.target);
  }
  return seen;
}

/** Breadth-first rings, independent of React and of a future desktop renderer. */
export function radialLayout(
  workspace: Pick<Workspace, "declarations" | "edges">,
  root: string,
  scope: GraphScope = "dependencies",
): Map<string, { x: number; y: number; depth: number }> {
  const visible = dependencyScope(workspace, root, scope);
  const depths = new Map<string, number>();
  if (!visible.has(root)) return new Map();
  const pending = [root];
  depths.set(root, 0);
  const declarations = new Map(
    workspace.declarations.map((node) => [node.id, node]),
  );
  const adjacency = new Map<string, string[]>();
  for (const edge of workspace.edges) {
    if (
      scope === "meaning" &&
      declarations.get(edge.source)?.kind === "theorem" &&
      edge.site === "value"
    )
      continue;
    const from = adjacency.get(edge.source) || [];
    from.push(edge.target);
    adjacency.set(edge.source, from);
    if (scope === "project")
      adjacency.set(edge.target, [
        ...(adjacency.get(edge.target) || []),
        edge.source,
      ]);
  }
  for (let i = 0; i < pending.length; i++)
    for (const target of adjacency.get(pending[i]) || []) {
      if (visible.has(target) && !depths.has(target)) {
        depths.set(target, depths.get(pending[i])! + 1);
        pending.push(target);
      }
    }
  const rings = new Map<number, string[]>();
  for (const [id, depth] of depths) {
    const ring = rings.get(depth) || [];
    ring.push(id);
    rings.set(depth, ring);
  }
  const positions = new Map<string, { x: number; y: number; depth: number }>();
  let previousRadius = 0;
  for (const [depth, ring] of rings) {
    ring.sort();
    const radius =
      depth === 0 ? 0 : Math.max(previousRadius + 180, ring.length * 46);
    previousRadius = radius;
    ring.forEach((id, index) => {
      const angle =
        -Math.PI / 2 +
        (2 * Math.PI * index) / ring.length +
        (depth % 2 ? 0 : 0.25);
      positions.set(id, {
        x: Math.cos(angle) * radius,
        y: Math.sin(angle) * radius,
        depth,
      });
    });
  }
  return positions;
}
