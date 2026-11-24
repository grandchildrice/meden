import {
  longestPathLevels,
  layoutCone,
} from "../vendor/lean4semanticbench-layout.js";
import type { Workspace } from "@meden/workspace-schema";

/** Fixed L4SB method and numeric configuration; no locale, clock or font probes.
 * Edge order is input order. Object key enumeration and UTF-16 lexical ties
 * retain the upstream order, including numeric-like declaration names.
 */
export const coneLayoutDefinition = Object.freeze({
  version: "l4sb.cone.v1",
  upstream:
    "0xfab0131/Lean4SemanticBench@97e7c29df8d3c358c395bd1a4f2fb6973527b33d",
  expandedHops: 1,
  rootWidth: 186,
  parameters: Object.freeze({
    r0: 196,
    dr: 200,
    slotMax: 200,
    slotMin: 94,
    slotPref: 202,
    rSoft: 1180,
    maxPerLayer: 20000,
  }),
  cones: Object.freeze({
    dep: Object.freeze({ th0: -Math.PI + 0.2, th1: -0.2 }),
    ref: Object.freeze({ th0: 0.2, th1: Math.PI - 0.2 }),
  }),
  scale: Object.freeze({ coordinates: 2.2, width: 1.6 }),
});
export type ConePosition = {
  x: number;
  y: number;
  depth: number;
  width: number;
  side: "dep" | "ref" | "root";
};

/** Actual MIT-licensed L4SB layout, adapted to Meden's project-reference schema. */
export function coneLayout(
  workspace: Pick<Workspace, "declarations" | "edges">,
  root: string,
): Map<string, ConePosition> {
  const result = new Map<string, ConePosition>();
  if (!workspace.declarations.some((item) => item.id === root)) return result;
  result.set(root, {
    x: 0,
    y: 0,
    depth: 0,
    width: coneLayoutDefinition.rootWidth,
    side: "root",
  });
  const byId = new Map(workspace.declarations.map((item) => [item.id, item]));
  const adj: Record<string, string[]> = Object.create(null),
    rev: Record<string, string[]> = Object.create(null);
  for (const edge of workspace.edges) {
    (adj[edge.source] ??= []).push(edge.target);
    (rev[edge.target] ??= []).push(edge.source);
  }
  for (const [side, index] of [
    ["dep", adj],
    ["ref", rev],
  ] as const) {
    const hop: Record<string, number> = Object.create(null);
    hop[root] = 0;
    const pending = [root];
    for (let i = 0; i < pending.length; i++)
      if (hop[pending[i]!] < coneLayoutDefinition.expandedHops)
        for (const id of index[pending[i]] || [])
          if (hop[id] === undefined) {
            hop[id] = hop[pending[i]] + 1;
            pending.push(id);
          }
    const { layers } = longestPathLevels(root, index, hop);
    const geometry = layoutCone({
      layers,
      adj: index,
      ...coneLayoutDefinition.parameters,
      ...coneLayoutDefinition.cones[side],
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
    point.x *= coneLayoutDefinition.scale.coordinates;
    point.y *= coneLayoutDefinition.scale.coordinates;
    point.width *= coneLayoutDefinition.scale.width;
  }
  return result;
}
