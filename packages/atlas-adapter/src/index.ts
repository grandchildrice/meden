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
    throw new Error("Duplicate declaration IDs");
  if (
    graph.edges.some((edge) => !ids.has(edge.source) || !ids.has(edge.target))
  )
    throw new Error("Dependency target declaration not found");
  const declarations: Declaration[] = graph.nodes.map((node) => ({
    id: node.id,
    name: node.id,
    label: node.shortName,
    isMainTheorem: node.meta?.isMainTheorem,
    kind:
      node.kind === "theorem"
        ? "theorem"
        : node.kind === "axiom"
          ? "axiom"
          : "definition",
    statement: node.statement || "",
    summary: node.meta?.summary || undefined,
    semantic: node.semantic,
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

/** Match Atlas' edge distinction; unclassified edges are retained conservatively. */
export function meaningDependencies(
  workspace: Pick<Workspace, "declarations" | "edges">,
  root: string,
): Set<string> {
  const nodes = new Map(workspace.declarations.map((node) => [node.id, node]));
  const edges = new Map<string, Workspace["edges"]>();
  for (const edge of workspace.edges) {
    if (nodes.get(edge.source)?.kind === "theorem" && edge.site === "value")
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

/** Actual MIT-licensed L4SB layout, adapted to Meden's project-reference schema. */
export function coneLayout(
  workspace: Pick<Workspace, "declarations" | "edges">,
  root: string,
): Map<
  string,
  {
    x: number;
    y: number;
    depth: number;
    width: number;
    side: "dep" | "ref" | "root";
  }
> {
  const result = new Map<
    string,
    {
      x: number;
      y: number;
      depth: number;
      width: number;
      side: "dep" | "ref" | "root";
    }
  >();
  if (!workspace.declarations.some((item) => item.id === root)) return result;
  result.set(root, { x: 0, y: 0, depth: 0, width: 186, side: "root" });
  const byId = new Map(workspace.declarations.map((item) => [item.id, item]));
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
      if (hop[pending[i]!] < 1)
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
      r0: 196,
      dr: 200,
      slotMax: 200,
      slotMin: 94,
      slotPref: 202,
      rSoft: 1180,
      maxPerLayer: 20000,
      groupOf: (name) => byId.get(name)?.source?.file || "",
    });
    for (const [id, pos] of Object.entries(geometry.pos))
      if (id !== root && !result.has(id))
        result.set(id, {
          x: pos.x,
          y: pos.y,
          depth: pos.level,
          width: pos.w,
          side,
        });
  }
  // Uniform enlargement preserves L4SB angles and gives English cards room.
  for (const point of result.values()) {
    point.x *= 2.2;
    point.y *= 2.2;
    point.width *= 1.6;
  }
  return result;
}

export { reviewScope, type ReviewScope } from "./review-scope";
