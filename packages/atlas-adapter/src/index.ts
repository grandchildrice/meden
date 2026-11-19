import {
  longestPathLevels,
  layoutCone,
} from "../vendor/lean4semanticbench-layout.js";
import { atlasGraphSchema } from "@meden/workspace-schema";
import type { Declaration, Workspace } from "@meden/workspace-schema";

/** Structural import. Atlas metadata alone cannot establish a fresh proof receipt. */
export function importAtlas(
  input: unknown,
): Pick<Workspace, "name" | "declarations" | "edges"> {
  const graph = atlasGraphSchema.parse(input);
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
    statement: node.statement || "",
    summary: node.meta?.summary || undefined,
    source: {
      file: node.filePath,
      ...(node.lineStart ? { line: node.lineStart } : {}),
    },
    proofStatus: node.hasSorry ? "unproved" : "unknown",
    evidenceRefs: [],
    axioms: [
      ...(node.axioms || node.meta?.axioms || []),
      ...(node.standardAxioms || []),
    ],
  }));
  return {
    name: graph.metadata.projectName,
    declarations,
    edges: graph.edges.map((edge) => ({
      source: edge.source,
      target: edge.target,
      site: edgeSite(edge.kind),
    })),
  };
}

function edgeSite(kind: string | undefined): "type" | "value" | "unknown" {
  switch (kind) {
    case "theorem_type_to_definition":
    case "theorem_type_to_theorem":
    case "definition_type_to_definition":
    case "definition_type_to_theorem":
      return "type";
    case "theorem_value_to_definition":
    case "theorem_value_to_theorem":
    case "definition_value_to_definition":
    case "definition_value_to_theorem":
      return "value";
    default:
      return "unknown";
  }
}

export type GraphScope = "dependencies" | "meaning" | "project";
/** Match Atlas' edge distinction; unclassified edges are retained conservatively. */
export function dependencyScope(
  workspace: Pick<Workspace, "declarations" | "edges">,
  root: string,
  scope: GraphScope,
): Set<string> {
  const nodes = new Map(workspace.declarations.map((node) => [node.id, node]));
  if (scope === "project") return new Set(nodes.keys());
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
  if (scope === "project") {
    const outerDepth = Math.max(0, ...depths.values()) + 1;
    for (const id of visible) if (!depths.has(id)) depths.set(id, outerDepth);
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

/** Actual MIT-licensed L4SB layout, adapted to Meden's project-reference schema. */
export function coneLayout(
  workspace: Pick<Workspace, "declarations" | "edges">,
  root: string,
): Map<
  string,
  { x: number; y: number; depth: number; side: "dep" | "ref" | "root" }
> {
  const result = new Map<
    string,
    { x: number; y: number; depth: number; side: "dep" | "ref" | "root" }
  >();
  if (!workspace.declarations.some((item) => item.id === root)) return result;
  result.set(root, { x: 0, y: 0, depth: 0, side: "root" });
  const adj: Record<string, string[]> = {},
    rev: Record<string, string[]> = {};
  for (const edge of workspace.edges) {
    (adj[edge.source] ??= []).push(edge.target);
    (rev[edge.target] ??= []).push(edge.source);
  }
  for (const [side, index] of [
    ["dep", adj],
    ["ref", rev],
  ] as const) {
    const hop: Record<string, number> = { [root]: 0 },
      pending = [root];
    for (let i = 0; i < pending.length; i++)
      for (const id of index[pending[i]] || [])
        if (hop[id] === undefined) {
          hop[id] = hop[pending[i]] + 1;
          pending.push(id);
        }
    const { layers } = longestPathLevels(root, index, hop);
    const geometry = layoutCone({
      layers,
      adj: index,
      th0: side === "dep" ? -Math.PI + 0.2 : 0.2,
      th1: side === "dep" ? -0.2 : Math.PI - 0.2,
      r0: 210,
      dr: 210,
      slotMax: 190,
      slotMin: 110,
      slotPref: 210,
      maxPerLayer: 20000,
      groupOf: (name) =>
        workspace.declarations.find((item) => item.id === name)?.source?.file ||
        "",
    });
    for (const [id, pos] of Object.entries(geometry.pos))
      if (id !== root && !result.has(id))
        result.set(id, { x: pos.x, y: pos.y, depth: pos.level, side });
  }
  return result;
}
