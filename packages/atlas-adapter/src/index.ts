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

export { coneLayout, coneLayoutDefinition } from "./cone-layout";
export type { ConePosition } from "./cone-layout";

export { reviewScope, type ReviewScope } from "./review-scope";
