import {
  layoutInput,
  mapGeometry,
  comparePath,
  MAP_WIDTH,
  MAP_HEIGHT,
  type MapGeometry,
} from "./project-map-layout";
export { MAP_WIDTH, MAP_HEIGHT } from "./project-map-layout";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";

import { declarationRole, type DeclarationRole } from "./declaration-role";
import { referenceEdges } from "./project-map-references";

type File = WorkspaceOverview["files"][number];
export type Symbol = File["symbols"][number] & {
  id: string;
  file: string;
  x: number;
  y: number;
  cellWidth: number;
  role: DeclarationRole;
};
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
  geometry: MapGeometry;
  tracked: string;
  symbolAreas: MapArea[];
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
  geometry: MapGeometry = mapGeometry(layoutInput(snapshot)),
  previous?: MapScene,
): MapScene {
  const files = new Map(snapshot.files.map((file) => [file.path, file]));
  const targetKey = JSON.stringify([...tracked].sort(comparePath));
  const reuse = previous?.geometry === geometry ? previous : undefined;
  const areas: MapArea[] = geometry.areas.map((box) => {
    const file = files.get(box.path);
    const old = reuse?.byPath.get(box.path);
    if (
      old &&
      old.file === file &&
      (!file?.symbols.length || reuse?.tracked === targetKey)
    )
      return old;
    const { width, height } = box;
    const columns = Math.max(
      1,
      Math.min(
        file?.symbols.length || 1,
        Math.ceil(
          Math.sqrt(
            ((file?.symbols.length || 1) * width) / Math.max(1, height),
          ),
        ),
      ),
    );
    const rows = Math.ceil((file?.symbols.length || 1) / columns);
    const symbols = [...(file?.symbols || [])]
      .sort((a, b) => a.line - b.line || comparePath(a.name, b.name))
      .map((s, i) => ({
        ...s,
        name: s.name.split(/[^\p{L}\p{N}_'.«»]/u)[0] || s.name,
        role: declarationRole(
          s.declaration,
          tracked.has(s.name.split(/[^\p{L}\p{N}_'.«»]/u)[0] || s.name) ||
            s.main_theorem ||
            (s.kind === "theorem" &&
              box.path.split("/").length <= 2 &&
              box.path.split("/").at(-1) === "Main.lean"),
        ),
        cellWidth: (width * 0.88) / columns,
        id: `${box.path}:${s.line}:${s.name}`,
        file: box.path,
        x: box.x + width * (0.06 + 0.88 * (((i % columns) + 0.5) / columns)),
        y:
          box.y +
          height * (0.18 + 0.76 * ((Math.floor(i / columns) + 0.5) / rows)),
      }));
    return {
      ...box,
      file,
      main: symbols.some((symbol) => symbol.role.id === "main_theorem"),
      cellHeight: (height * 0.76) / rows,
      symbols,
    };
  });
  const changed = reuse
    ? areas.filter((area, i) => area !== reuse.areas[i])
    : areas;
  const changedPaths = reuse
    ? new Set(changed.map((area) => area.path))
    : undefined;
  const symbolAreas = (
    reuse
      ? [
          ...reuse.symbolAreas.filter((area) => !changedPaths!.has(area.path)),
          ...changed.filter((area) => area.symbols.length),
        ]
      : areas.filter((area) => area.symbols.length)
  ).sort((a, b) => comparePath(a.path, b.path));
  const symbols = new Map(
    symbolAreas.flatMap((a) => a.symbols.map((s) => [s.id, s] as const)),
  );
  const edges = referenceEdges(symbols);
  const edgesByFile = new Map<string, typeof edges>();
  for (const edge of edges) {
    const file = symbols.get(edge.source)!.file;
    const found = edgesByFile.get(file) || [];
    found.push(edge);
    edgesByFile.set(file, found);
  }
  const byPath = reuse
    ? new Map(reuse.byPath)
    : new Map(areas.map((area) => [area.path, area]));
  if (reuse) for (const area of changed) byPath.set(area.path, area);
  const children = reuse
    ? new Map(reuse.children)
    : new Map<string, MapArea[]>();
  if (reuse) {
    for (const parent of new Set(changed.map((area) => area.parent)))
      children.set(
        parent,
        reuse.children.get(parent)!.map((area) => byPath.get(area.path)!),
      );
  } else
    for (const area of areas) {
      const siblings = children.get(area.parent) || [];
      siblings.push(area);
      children.set(area.parent, siblings);
    }
  return {
    geometry,
    tracked: targetKey,
    symbolAreas,
    inventoryOnly: snapshot.inventory_only,
    mainPins: symbolAreas.flatMap((area) => {
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
    maxZoom:
      reuse?.maxZoom ??
      areas.reduce(
        (zoom, a) =>
          a.file ? Math.max(zoom, 600 / Math.min(a.width, a.height)) : zoom,
        500,
      ),
    areas,
    byPath,
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

/** The legend describes declarations actually rendered in the current viewport. */
export function mapHasDeclarations(
  visible: ReturnType<typeof visibleMapAreas>,
  zoom: number,
  size: { width: number; height: number },
) {
  return visible.some(
    ({ area, opacity }) =>
      area.symbols.length > 0 &&
      opacity * declarationDetail(area, zoom, size) >= 0.01,
  );
}

/** Resolve the center breadcrumb through containing branches, not the inventory. */
export function mapLocation(
  scene: MapScene,
  view: { x: number; y: number; zoom: number },
  size: { width: number; height: number },
) {
  if (
    view.zoom <=
    Math.min(size.width / MAP_WIDTH, size.height / MAP_HEIGHT) * 1.2
  )
    return "";
  const x = (size.width / 2 - view.x) / view.zoom,
    y = (size.height / 2 - view.y) / view.zoom;
  const queue = (scene.children.get("") || []).map((area) => ({
    area,
    opacity: 1,
  }));
  let location = "";
  for (let i = 0; i < queue.length; i++) {
    const { area, opacity } = queue[i]!;
    if (
      opacity <= 0.5 ||
      x < area.x ||
      x > area.x + area.width ||
      y < area.y ||
      y > area.y + area.height
    )
      continue;
    location = area.path;
    const childOpacity =
      opacity * fade(Math.min(area.width, area.height) * view.zoom);
    if (childOpacity > 0.5)
      for (const child of scene.children.get(area.path) || [])
        queue.push({ area: child, opacity: childOpacity });
  }
  return location;
}
