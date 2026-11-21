import type { Workspace } from "@meden/workspace-schema";
export type ReviewScope = "human" | "proof-only" | "unknown";
/** Atlas meaning traversal: only known theorem-body edges may be excluded. */
export function reviewScope(
  workspace: Pick<Workspace, "declarations" | "edges">,
  roots: string[],
): Map<string, ReviewScope> {
  const kinds = new Map(workspace.declarations.map((d) => [d.id, d.kind]));
  const validRoots = roots.filter((id) => kinds.has(id));
  const adjacency = new Map<string, Workspace["edges"]>();
  for (const edge of workspace.edges) {
    const edges = adjacency.get(edge.source) || [];
    edges.push(edge);
    adjacency.set(edge.source, edges);
  }
  const walk = (mode: "all" | "possible" | "known") => {
    const visited = new Set<string>(),
      pending = [...validRoots];
    while (pending.length) {
      const id = pending.pop()!;
      if (visited.has(id)) continue;
      visited.add(id);
      for (const edge of adjacency.get(id) || []) {
        if (
          mode !== "all" &&
          kinds.get(id) === "theorem" &&
          edge.site === "value"
        )
          continue;
        if (mode === "known" && edge.site === "unknown") continue;
        pending.push(edge.target);
      }
    }
    return visited;
  };
  const all = walk("all"),
    possible = walk("possible"),
    known = walk("known");
  return new Map(
    workspace.declarations.map((d) => [
      d.id,
      known.has(d.id)
        ? "human"
        : all.has(d.id) && !possible.has(d.id)
          ? "proof-only"
          : "unknown",
    ]),
  );
}
