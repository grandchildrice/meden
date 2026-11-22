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
  main: boolean;
  cellHeight: number;
  symbols: Symbol[];
};
export type MapScene = {
  inventoryOnly: boolean;
  maxZoom: number;
  mainPins: Array<{ area: MapArea; symbols: Symbol[]; origin: string }>;
  areas: MapArea[];
  byPath: Map<string, MapArea>;
  children: Map<string, MapArea[]>;
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
    .paddingInner((d) => {
      const span = Math.min(d.x1 - d.x0, d.y1 - d.y0);
      const smallest = (d.children || []).reduce(
        (value, child) => Math.min(value, (child.value || 1) / (d.value || 1)),
        1,
      );
      // A fixed gap can collapse a rare entry file next to a huge category.
      return Math.min(d.depth ? span * 0.012 : 22, span * smallest * 0.2);
    })
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
        Math.min(
          d.data.file?.symbols.length || 1,
          Math.ceil(
            Math.sqrt(
              ((d.data.file?.symbols.length || 1) * width) /
                Math.max(1, height),
            ),
          ),
        ),
      );
      const rows = Math.ceil((d.data.file?.symbols.length || 1) / columns);
      const symbols = (d.data.file?.symbols || []).map((s, i) => ({
        ...s,
        name: s.name.split(/[^\p{L}\p{N}_'.«»]/u)[0] || s.name,
        role: declarationRole(
          s.declaration,
          tracked.has(s.name.split(/[^\p{L}\p{N}_'.«»]/u)[0] || s.name) ||
            s.main_theorem ||
            (s.kind === "theorem" &&
              d.data.path.split("/").length <= 2 &&
              d.data.path.split("/").at(-1) === "Main.lean"),
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
        main: symbols.some((symbol) => symbol.role.id === "main_theorem"),
        cellHeight: (height * 0.76) / rows,
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
  const children = new Map<string, MapArea[]>();
  for (const area of areas) {
    const siblings = children.get(area.parent) || [];
    siblings.push(area);
    children.set(area.parent, siblings);
  }
  return {
    inventoryOnly: snapshot.inventory_only,
    mainPins: areas.flatMap((area) => {
      const main = area.symbols.filter((s) => s.role.id === "main_theorem");
      return main.length
        ? [
            {
              area,
              symbols: main,
              origin: main.some((s) => tracked.has(s.name))
                ? "Saved proof targets"
                : main.some((s) => s.main_theorem)
                  ? "Main-theorem attributes from compiled metadata"
                  : "Theorems in the project entry module",
            },
          ]
        : [];
    }),
    maxZoom: areas.reduce(
      (zoom, a) =>
        a.file ? Math.max(zoom, 600 / Math.min(a.width, a.height)) : zoom,
      500,
    ),
    areas,
    byPath: new Map(areas.map((a) => [a.path, a])),
    children,
    symbols,
    edges,
    edgesByFile,
  };
}

export const fade = (pixels: number, start = 600, end = 900) =>
  Math.max(0, Math.min(1, (pixels - start) / (end - start)));

/** A fitted file must reach readable detail even in a narrow map pane. */
export function declarationDetail(
  area: MapArea,
  zoom: number,
  size: { width: number; height: number },
) {
  if (area.main && area.symbols.length)
    return fade(
      Math.max(area.symbols[0]!.cellWidth, area.cellHeight) * zoom,
      32,
      64,
    );
  const end = Math.min(500, Math.min(size.width, size.height) * 0.4);
  return fade(Math.min(area.width, area.height) * zoom, end * 0.64, end);
}

/** Rendering and source prefetch share the same semantic-zoom visibility. */
export function visibleMapAreas(
  scene: MapScene,
  viewport: { x: number; y: number; zoom: number },
  size: { width: number; height: number },
) {
  const { x, y, zoom } = viewport;
  const left = -x / zoom,
    top = -y / zoom,
    right = left + size.width / zoom,
    bottom = top + size.height / zoom;
  const queue = (scene.children.get("") || []).map((area) => ({
    area,
    opacity: 1,
  }));
  const visible: Array<{ area: MapArea; opacity: number }> = [];
  for (let i = 0; i < queue.length; i++) {
    const { area, opacity: alpha } = queue[i]!;
    if (
      alpha < 0.015 ||
      area.x > right ||
      area.y > bottom ||
      area.x + area.width < left ||
      area.y + area.height < top
    )
      continue;
    if (
      (area.width * zoom >= 2 && area.height * zoom >= 2) ||
      (area.main && declarationDetail(area, zoom, size) > 0.01)
    )
      visible.push({ area, opacity: alpha });
    // Descendants are contained in their parent's rectangle and cannot become
    // more opaque. Invisible branches need no per-file traversal.
    const childAlpha =
      alpha * fade(Math.max(area.width, area.height) * zoom, 650, 950);
    if (childAlpha >= 0.015)
      for (const child of scene.children.get(area.path) || [])
        queue.push({ area: child, opacity: childAlpha });
  }
  return visible;
}
