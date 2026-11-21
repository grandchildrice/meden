import { hierarchy, treemap, treemapBinary } from "d3-hierarchy";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";

import { declarationRole, type DeclarationRole } from "./declaration-role";

export const MAP_WIDTH = 10000;
export const MAP_HEIGHT = 6600;
type File = WorkspaceOverview["files"][number];
export type Symbol = File["symbols"][number] & {
  id: string;
  file: string;
  x: number;
  y: number;
  cellWidth: number;
  role: DeclarationRole;
};
type Branch = { path: string; file?: File; children?: Branch[] };
export type MapArea = {
  path: string;
  file?: File;
  parent: string;
  x: number;
  y: number;
  width: number;
  height: number;
  depth: number;
  color: number;
  files: number;
  declarations: number;
  symbols: Symbol[];
};
export type MapScene = {
  inventoryOnly: boolean;
  areas: MapArea[];
  byPath: Map<string, MapArea>;
  symbols: Map<string, Symbol>;
  edges: Array<{ source: string; target: string }>;
  edgesByFile: Map<string, Array<{ source: string; target: string }>>;
};

/** Prepared once per native source revision. Navigation never changes these coordinates. */
export function projectMap(
  snapshot: WorkspaceOverview,
  tracked: ReadonlySet<string> = new Set(),
): MapScene {
  const root: Branch = { path: "", children: [] };
  const branches = new Map([["", root]]);
  for (const file of snapshot.files) {
    let parent = root;
    const parts = file.path.split("/");
    for (let i = 0; i < parts.length; i++) {
      const path = parts.slice(0, i + 1).join("/");
      let item = branches.get(path);
      if (!item) {
        item = i === parts.length - 1 ? { path, file } : { path, children: [] };
        branches.set(path, item);
        parent.children!.push(item);
      }
      parent = item;
    }
  }
  // Show the dominant namespace's categories without giving them new coordinates on zoom.
  const count = (branch: Branch): number =>
    branch.file ? 1 : (branch.children || []).reduce((n, b) => n + count(b), 0);
  const dominant = root.children?.find(
    (b) => b.children && count(b) > snapshot.files.length * 0.8,
  );
  if (dominant)
    root.children = root.children!.flatMap((b) =>
      b === dominant ? b.children! : [b],
    );
  const tree = hierarchy(root)
    .sum((b) =>
      b.file
        ? snapshot.inventory_only
          ? 1
          : Math.max(1, b.file.declarations)
        : 0,
    )
    .sort(
      (a, b) => b.value! - a.value! || a.data.path.localeCompare(b.data.path),
    );
  const layout = treemap<Branch>()
    .size([MAP_WIDTH, MAP_HEIGHT])
    .tile(treemapBinary)
    .paddingInner((d) =>
      d.depth ? Math.min(d.x1 - d.x0, d.y1 - d.y0) * 0.012 : 22,
    )
    .paddingOuter((d) =>
      d.depth ? Math.min(d.x1 - d.x0, d.y1 - d.y0) * 0.016 : 0,
    )
    .paddingTop((d) =>
      d.depth ? Math.min(d.x1 - d.x0, d.y1 - d.y0) * 0.09 : 0,
    )(tree);
  const colors = new Map(layout.children?.map((b, i) => [b.data.path, i % 5]));
  const areas: MapArea[] = layout
    .descendants()
    .filter((d) => d.depth)
    .map((d) => {
      let top = d;
      while (top.parent?.depth) top = top.parent;
      const width = d.x1 - d.x0,
        height = d.y1 - d.y0;
      const columns = Math.max(
        1,
        Math.ceil(
          Math.sqrt(
            ((d.data.file?.symbols.length || 1) * width) / Math.max(1, height),
          ),
        ),
      );
      const rows = Math.ceil((d.data.file?.symbols.length || 1) / columns);
      const symbols = (d.data.file?.symbols || []).map((s, i) => ({
        ...s,
        name: s.name.split(/[^\p{L}\p{N}_'.«»]/u)[0] || s.name,
        role: declarationRole(
          s.declaration,
          tracked.has(s.name.split(/[^\p{L}\p{N}_'.«»]/u)[0] || s.name),
        ),
        cellWidth: (width * 0.88) / columns,
        id: `${d.data.path}:${s.line}:${s.name}`,
        file: d.data.path,
        x: d.x0 + width * (0.06 + 0.88 * (((i % columns) + 0.5) / columns)),
        y:
          d.y0 +
          height * (0.18 + 0.76 * ((Math.floor(i / columns) + 0.5) / rows)),
      }));
      return {
        path: d.data.path,
        file: d.data.file,
        parent: d.parent?.data.path || "",
        x: d.x0,
        y: d.y0,
        width,
        height,
        depth: d.depth,
        color: colors.get(top.data.path) || 0,
        files: count(d.data),
        declarations: d.value || 0,
        symbols,
      };
    });
  const symbols = new Map(
    areas.flatMap((a) => a.symbols.map((s) => [s.id, s] as const)),
  );
  const names = new Map<string, Symbol[]>();
  const local = new Map<string, Symbol[]>();
  for (const symbol of symbols.values()) {
    names.set(symbol.name, [...(names.get(symbol.name) || []), symbol]);
    const key = symbol.file + ":" + symbol.name.split(".").at(-1);
    local.set(key, [...(local.get(key) || []), symbol]);
  }
  const edges: MapScene["edges"] = [];
  const seenEdges = new Set<string>();
  for (const symbol of symbols.values())
    for (const reference of symbol.references) {
      const parts = symbol.name.split(".").slice(0, -1);
      const candidates = [
        ...parts.map(
          (_, i) =>
            parts.slice(0, parts.length - i).join(".") + "." + reference,
        ),
        reference,
      ];
      let target: Symbol | undefined;
      for (const name of candidates) {
        const found = (names.get(name) || []).filter(
          (s) => !s.private || s.file === symbol.file,
        );
        if (found.length === 1) {
          target = found[0];
          break;
        }
      }
      const sameFile = local.get(symbol.file + ":" + reference);
      target ||= sameFile?.length === 1 ? sameFile[0] : undefined;
      if (
        target &&
        target.id !== symbol.id &&
        !seenEdges.has(symbol.id + "->" + target.id)
      ) {
        seenEdges.add(symbol.id + "->" + target.id);
        edges.push({ source: symbol.id, target: target.id });
      }
    }
  const edgesByFile = new Map<string, typeof edges>();
  for (const edge of edges) {
    const file = symbols.get(edge.source)!.file;
    const found = edgesByFile.get(file) || [];
    found.push(edge);
    edgesByFile.set(file, found);
  }
  return {
    inventoryOnly: snapshot.inventory_only,
    areas,
    byPath: new Map(areas.map((a) => [a.path, a])),
    symbols,
    edges,
    edgesByFile,
  };
}

export const fade = (pixels: number, start = 600, end = 900) =>
  Math.max(0, Math.min(1, (pixels - start) / (end - start)));
