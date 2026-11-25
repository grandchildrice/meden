import {
  layoutInput,
  mapGeometry,
  comparePath,
  MAP_WIDTH,
  MAP_HEIGHT,
  type MapGeometry,
  type MapInput,
} from "./project-map-layout";
export { MAP_WIDTH, MAP_HEIGHT } from "./project-map-layout";
import type {
  WorkspaceOverview,
  InventoryChunk,
} from "@meden/agent-protocol/native";

import { declarationRole, type DeclarationRole } from "./declaration-role";
import { ReferenceIndex } from "./project-map-references";
import { OrderedBounds, type Bounds } from "./ordered-bounds";

export type NavigationFile = InventoryChunk["data"]["files"][number];
export type IndexedFile = Extract<
  WorkspaceOverview["files"][number],
  { state: "indexed" }
>;
export type MapFile = WorkspaceOverview["files"][number] | NavigationFile;
type MapSnapshot = Omit<MapInput, "files"> & { files: readonly MapFile[] };
type Heading = IndexedFile["headers"][number] &
  Partial<Pick<NavigationFile["symbols"][number], "references" | "semantic">>;
const NO_HEADINGS: readonly Heading[] = [];
const headings = (file: MapFile): readonly Heading[] =>
  "headers" in file
    ? file.headers
    : "symbols" in file
      ? file.symbols
      : NO_HEADINGS;
export const fileIndexed = (file: MapFile) =>
  "state" in file ? file.state === "indexed" : file.indexed;
/** A complete navigation response can still report an unreadable file. */
export const navigationReady = (file: MapFile) => "symbols" in file;
export type Symbol = NavigationFile["symbols"][number] & {
  id: string;
  file: string;
  x: number;
  y: number;
  cellWidth: number;
  role: DeclarationRole;
};
export type MapArea = {
  path: string;
  file?: MapFile;
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
  batch: number;
  symbols: Symbol[];
};
export type MapIndex = {
  snapshot: MapSnapshot;
  geometry: MapGeometry;
  byPath: ReadonlyMap<string, MapArea>;
  children: ReadonlyMap<string, readonly MapArea[]>;
  branches: ReadonlyMap<string, OrderedBounds<MapArea>>;
  symbolFiles: readonly MapFile[];
  maxZoom: number;
};
export type MapScene = {
  index: MapIndex;
  tracked: string;
  descriptions: ReadonlyMap<string, MapArea>;
  symbolAreas: MapArea[];
  maxZoom: number;
  mainPins: Array<{ area: MapArea; symbols: Symbol[]; origin: string }>;
  symbols: Map<string, Symbol>;
  references: ReferenceIndex;
  edges: Array<{ source: string; target: string }>;
  edgesByFile: Map<string, Array<{ source: string; target: string }>>;
};

/** One immutable coordinate/branch index per original inventory and geometry. */
export function prepareMap(
  snapshot: MapSnapshot,
  geometry: MapGeometry = mapGeometry(layoutInput(snapshot)),
): MapIndex {
  const files = new Map(snapshot.files.map((file) => [file.path, file]));
  const children = new Map<string, MapArea[]>();
  const areas: MapArea[] = geometry.areas.map((box) => {
    const siblings = children.get(box.parent) || [];
    const area: MapArea = {
      ...box,
      file: files.get(box.path),
      main: false,
      cellHeight: box.height * 0.76,
      batch: Math.floor(siblings.length / 128),
      symbols: [],
    };
    siblings.push(area);
    children.set(box.parent, siblings);
    return area;
  });
  return {
    snapshot,
    geometry,
    byPath: new Map(areas.map((area) => [area.path, area])),
    children,
    branches: new Map(
      [...children]
        .filter(([, siblings]) => siblings.length > 64)
        .map(([path, siblings]) => [path, new OrderedBounds(siblings)]),
    ),
    symbolFiles: snapshot.files.filter((file) => headings(file).length),
    maxZoom: areas.reduce(
      (zoom, a) =>
        a.file ? Math.max(zoom, 600 / Math.min(a.width, a.height)) : zoom,
      500,
    ),
  };
}

function describe(
  box: MapArea,
  file: MapFile,
  tracked: ReadonlySet<string>,
): MapArea {
  const { width, height } = box;
  const columns = Math.max(
    1,
    Math.min(
      headings(file).length || 1,
      Math.ceil(
        Math.sqrt(((headings(file).length || 1) * width) / Math.max(1, height)),
      ),
    ),
  );
  const rows = Math.ceil((headings(file).length || 1) / columns);
  const symbols = [...headings(file)]
    .sort((a, b) => a.line - b.line || comparePath(a.name, b.name))
    .map((s, i) => ({
      ...s,
      references: s.references ?? [],
      semantic: s.semantic ?? null,
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
    main: symbols.some((s) => s.role.id === "main_theorem"),
    cellHeight: (height * 0.76) / rows,
    symbols,
  };
}

export function mapArea(scene: MapScene, path: string) {
  return scene.descriptions.get(path) || scene.index.byPath.get(path);
}

/** Project current file observations over fixed geometry. Only loaded/changed
 * descriptions participate in updates; no complete inventory/index is copied.
 * An unrelated index never inherits an older scene's descriptions or edges.
 */
export function projectMap(
  index: MapIndex,
  tracked: ReadonlySet<string> = new Set(),
  files: ReadonlyMap<string, MapFile> = new Map(),
  previous?: MapScene,
): MapScene {
  const targetKey = JSON.stringify([...tracked].sort(comparePath));
  const reuse = previous?.index === index ? previous : undefined;
  const changed = new Map<string, MapFile>();
  if (!reuse)
    for (const file of index.symbolFiles) changed.set(file.path, file);
  for (const [path, file] of files) {
    if (path !== file.path)
      throw Error("Map file observation does not match its path");
    if (!index.byPath.get(path)?.file) continue;
    if (!reuse || mapArea(reuse, path)!.file !== file) changed.set(path, file);
  }
  if (reuse && reuse.tracked !== targetKey)
    for (const area of reuse.symbolAreas)
      if (!changed.has(area.path)) changed.set(area.path, area.file!);
  if (reuse && !changed.size && reuse.tracked === targetKey) return reuse;
  const descriptions = new Map(reuse?.descriptions);
  let referencesChanged = !reuse,
    targetsChanged = !reuse;
  for (const [path, file] of changed) {
    const base = index.byPath.get(path)!;
    if (reuse && mapArea(reuse, path)!.file !== file) referencesChanged = true;
    const next = describe(base, file, tracked);
    const before = reuse && mapArea(reuse, path)!.symbols;
    if (
      !before ||
      before.length !== next.symbols.length ||
      before.some((old, i) => {
        const current = next.symbols[i]!;
        return (
          old.id !== current.id ||
          old.file !== current.file ||
          old.name !== current.name ||
          old.private !== current.private
        );
      })
    )
      targetsChanged = true;
    if (file === base.file && !headings(file).length) descriptions.delete(path);
    else descriptions.set(path, next);
  }
  const symbolAreas = [...descriptions.values()]
    .filter((area) => area.symbols.length)
    .sort((a, b) => comparePath(a.path, b.path));
  const symbols = new Map<string, Symbol>();
  for (const area of symbolAreas)
    for (const symbol of area.symbols) symbols.set(symbol.id, symbol);
  const references =
    reuse && !targetsChanged ? reuse.references : new ReferenceIndex(symbols);
  const edges =
    !referencesChanged && reuse ? reuse.edges : references.edges(symbols);
  let edgesByFile = reuse && !referencesChanged ? reuse.edgesByFile : undefined;
  if (!edgesByFile) {
    edgesByFile = new Map<string, typeof edges>();
    for (const edge of edges) {
      const file = symbols.get(edge.source)!.file,
        found = edgesByFile.get(file) || [];
      found.push(edge);
      edgesByFile.set(file, found);
    }
  }
  return {
    index,
    tracked: targetKey,
    descriptions,
    symbolAreas,
    maxZoom: index.maxZoom,
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
    symbols,
    references,
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

type VisibleAreas = ReadonlyArray<{ area: MapArea; opacity: number }>;
type Visibility = {
  x: number;
  y: number;
  zoom: number;
  width: number;
  height: number;
  areas: VisibleAreas;
};
// SVG, Canvas and prefetch observe the same view through separate subscriptions.
// Keep only the latest two exact views per immutable scene, including a transient
// previous viewport while those subscriptions catch up. This owns no geometry.
const visibility = new WeakMap<MapScene, readonly Visibility[]>();

function visitChildren(
  index: MapIndex,
  parent: string,
  bounds: Bounds,
  emit: (area: MapArea) => void,
) {
  const branch = index.branches.get(parent);
  if (branch) branch.visit(bounds, emit);
  else
    for (const area of index.children.get(parent) || [])
      if (!(
        area.x > bounds.right ||
        area.y > bounds.bottom ||
        area.x + area.width < bounds.left ||
        area.y + area.height < bounds.top
      ))
        emit(area);
}

/** Rendering and source prefetch share the same semantic-zoom visibility. */
export function visibleMapAreas(
  scene: MapScene,
  viewport: { x: number; y: number; zoom: number },
  size: { width: number; height: number },
): VisibleAreas {
  const { x, y, zoom } = viewport;
  const cached = visibility.get(scene) || [];
  const retained = cached.find(
    (view) =>
      view.x === x &&
      view.y === y &&
      view.zoom === zoom &&
      view.width === size.width &&
      view.height === size.height,
  );
  if (retained) return retained.areas;
  const left = -x / zoom,
    top = -y / zoom,
    right = left + size.width / zoom,
    bottom = top + size.height / zoom;
  const bounds = { left, top, right, bottom },
    queue: Array<{ area: MapArea; opacity: number }> = [];
  visitChildren(scene.index, "", bounds, (area) =>
    queue.push({ area, opacity: 1 }),
  );
  const visible: Array<{ area: MapArea; opacity: number }> = [];
  for (let i = 0; i < queue.length; i++) {
    const { area: base, opacity: alpha } = queue[i]!;
    const area = scene.descriptions.get(base.path) || base;
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
      visitChildren(scene.index, area.path, bounds, (child) =>
        queue.push({ area: child, opacity: childAlpha }),
      );
  }
  visibility.set(scene, [
    { x, y, zoom, ...size, areas: visible },
    ...cached.slice(0, 1),
  ]);
  return visible;
}

/** Visible siblings retain their original order and stable bounded SVG parents.
 * React placements need not search every later uncommitted file in a directory.
 */
export function mapAreaBatches(visible: ReturnType<typeof visibleMapAreas>) {
  const batches: Array<{
    key: string;
    parent: string;
    batch: number;
    visible: Array<VisibleAreas[number]>;
  }> = [];
  for (const item of visible) {
    const { parent, batch } = item.area;
    let last = batches.at(-1);
    if (!last || last.parent !== parent || last.batch !== batch) {
      last = { key: batch + ":" + parent, parent, batch, visible: [] };
      batches.push(last);
    }
    last.visible.push(item);
  }
  return batches;
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
  const bounds = { left: x, top: y, right: x, bottom: y },
    queue: Array<{ area: MapArea; opacity: number }> = [];
  visitChildren(scene.index, "", bounds, (area) =>
    queue.push({ area, opacity: 1 }),
  );
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
      visitChildren(scene.index, area.path, bounds, (child) =>
        queue.push({ area: child, opacity: childOpacity }),
      );
  }
  return location;
}
