import { hierarchy, treemap, treemapBinary } from "d3-hierarchy";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";

export const MAP_WIDTH = 10000;
export const MAP_HEIGHT = 6600;
export const MAP_LAYOUT = {
  version: 1,
  implementation: "d3-hierarchy@3.1.2/treemapBinary",
  order: "weight-descending/utf16-path-ascending",
  seed: null,
  extent: [MAP_WIDTH, MAP_HEIGHT],
  dominant: 0.8,
  inner: 0.012,
  rootGap: 22,
  minimumGap: 0.2,
  outer: 0.016,
  heading: 0.09,
  colors: 5,
  maximumDepth: 128,
  maximumAreas: 1000000,
} as const;
export const comparePath = (a: string, b: string) =>
  a < b ? -1 : a > b ? 1 : 0;

type Group = {
  index: number;
  path: string;
  parent: string;
  files: number;
  weight: number;
  leaf: boolean;
  children?: Group[];
};
export type LayoutInput = {
  text: string;
  groups: ReadonlyMap<string, Group>;
  dominant?: string;
  colors: ReadonlyMap<string, number>;
  roots: Group[];
};
export type MapBox = {
  path: string;
  parent: string;
  x: number;
  y: number;
  width: number;
  height: number;
  depth: number;
  color: number;
  files: number;
  declarations: number;
};
export type MapGeometry = { input: string; areas: MapBox[] };

/** Only geometry dependencies enter this key. Chunk descriptions, symbols,
 * source bytes and tracked proof targets cannot relocate an existing box.
 */
export type MapInput = Pick<WorkspaceOverview, "inventory_only"> & {
  files: readonly (
    | Pick<
        Extract<WorkspaceOverview["files"][number], { state: "indexed" }>,
        "path" | "declarations"
      >
    | { path: string; state: "unindexed" }
  )[];
};
export function layoutInput(snapshot: MapInput): LayoutInput {
  if (snapshot.files.length > 250000)
    throw new Error("Map inventory exceeds its file budget");
  const files = snapshot.files
    .map(
      (file) =>
        [
          file.path,
          snapshot.inventory_only
            ? 1
            : Math.max(1, "declarations" in file ? file.declarations : 0),
        ] as const,
    )
    .sort((a, b) => comparePath(a[0], b[0]));
  const groups = new Map<string, Group>();
  const roots: Group[] = [];
  let previous: string[] = [];
  const stack: Group[] = [];
  for (const [path, weight] of files) {
    const parts = path.split("/");
    if (
      !path ||
      path.length > 4096 ||
      path.includes("\0") ||
      parts.length > MAP_LAYOUT.maximumDepth ||
      parts.some((p) => !p || p === "." || p === "..") ||
      !Number.isSafeInteger(weight) ||
      weight < 1 ||
      weight > 1048576
    )
      throw new Error("Invalid map path or weight");
    // Sorted paths share a contiguous prefix. Reuse its original groups instead
    // of allocating/slicing/hashing every ancestor string again for each file.
    let common = 0;
    while (common < parts.length && parts[common] === previous[common]) {
      const group = stack[common]!;
      if (group.leaf || common === parts.length - 1)
        throw new Error("Conflicting map file and folder paths");
      group.files++;
      group.weight += weight;
      common++;
    }
    stack.length = common;
    for (let i = common; i < parts.length; i++) {
      const parent = stack[i - 1],
        leaf = i === parts.length - 1;
      const created: Group = {
        index: groups.size,
        path: parent ? parent.path + "/" + parts[i]! : parts[i]!,
        parent: parent?.path || "",
        files: 1,
        weight,
        leaf,
        ...(!leaf ? { children: [] } : {}),
      };
      groups.set(created.path, created);
      if (parent) parent.children!.push(created);
      else roots.push(created);
      stack.push(created);
    }
    previous = parts;
    if (groups.size > MAP_LAYOUT.maximumAreas)
      throw new Error("Map layout exceeds its area budget");
  }
  const dominant = roots.find(
    (g) => !g.leaf && g.files > files.length * MAP_LAYOUT.dominant,
  )?.path;
  const top = roots
    .flatMap((group) => (group.path === dominant ? group.children! : [group]))
    .sort((a, b) => b.weight - a.weight || comparePath(a.path, b.path));
  return {
    text: JSON.stringify([MAP_LAYOUT, snapshot.inventory_only, files]),
    groups,
    roots,
    dominant,
    colors: new Map(top.map((g, i) => [g.path, i % MAP_LAYOUT.colors])),
  };
}

/** Pure world coordinates, independent of camera, viewport, locale and clocks. */
export function mapGeometry(input: LayoutInput): MapGeometry {
  const root: Group = {
    index: -1,
    path: "",
    parent: "",
    weight: 0,
    files: 0,
    leaf: false,
    children: input.roots.flatMap((group) =>
      group.path === input.dominant ? group.children! : [group],
    ),
  };
  const tree = hierarchy(root)
    .sum((b) => (b.leaf ? b.weight : 0))
    .sort(
      (a, b) => b.value! - a.value! || comparePath(a.data.path, b.data.path),
    );
  const layout = treemap<Group>()
    .size([MAP_WIDTH, MAP_HEIGHT])
    .tile(treemapBinary)
    .paddingInner((d) => {
      const span = Math.min(d.x1 - d.x0, d.y1 - d.y0);
      const smallest = (d.children || []).reduce(
        (n, c) => Math.min(n, (c.value || 1) / (d.value || 1)),
        1,
      );
      return Math.min(
        d.depth ? span * MAP_LAYOUT.inner : MAP_LAYOUT.rootGap,
        span * smallest * MAP_LAYOUT.minimumGap,
      );
    })
    .paddingOuter((d) =>
      d.depth ? Math.min(d.x1 - d.x0, d.y1 - d.y0) * MAP_LAYOUT.outer : 0,
    )
    .paddingTop((d) =>
      d.depth ? Math.min(d.x1 - d.x0, d.y1 - d.y0) * MAP_LAYOUT.heading : 0,
    )(tree);
  const geometry = {
    input: input.text,
    areas: layout
      .descendants()
      .filter((d) => d.depth)
      .map((d) => {
        let top = d;
        while (top.parent?.depth) top = top.parent;
        return {
          path: d.data.path,
          parent: d.parent?.data.path || "",
          x: d.x0,
          y: d.y0,
          width: d.x1 - d.x0,
          height: d.y1 - d.y0,
          depth: d.depth,
          color: input.colors.get(top.data.path)!,
          files: input.groups.get(d.data.path)!.files,
          declarations: d.value || 0,
        };
      }),
  };
  if (
    geometry.areas.some(
      (box) =>
        ![box.x, box.y, box.width, box.height].every(Number.isFinite) ||
        box.width <= 0 ||
        box.height <= 0,
    )
  )
    throw new Error("Map coordinates cannot represent the complete inventory");
  return geometry;
}

/** Coordinates are disposable presentation data, never proof or source authority. */
function containsBox(box: MapBox, above?: MapBox) {
  const { x, y, width, height } = box;
  return (
    Number.isFinite(x) &&
    Number.isFinite(y) &&
    Number.isFinite(width) &&
    Number.isFinite(height) &&
    width > 0 &&
    height > 0 &&
    x >= (above?.x || 0) - 1e-7 &&
    y >= (above?.y || 0) - 1e-7 &&
    x + width <= (above ? above.x + above.width : MAP_WIDTH) + 1e-7 &&
    y + height <= (above ? above.y + above.height : MAP_HEIGHT) + 1e-7
  );
}

// Fixed little-endian rectangle artifact: dictionary index plus four f64 values.
// Paths/counts/roles come from the original input, never from disposable storage.
const MAGIC = 0x314c504d;
const ROW_BYTES = 36;
export function packGeometry(
  geometry: MapGeometry,
  input: LayoutInput,
): ArrayBuffer {
  if (geometry.input !== input.text) throw new Error("Foreign map geometry");
  const buffer = new ArrayBuffer(4 + geometry.areas.length * ROW_BYTES),
    view = new DataView(buffer);
  view.setUint32(0, MAGIC, true);
  geometry.areas.forEach((box, i) => {
    const offset = 4 + i * ROW_BYTES;
    view.setUint32(offset, input.groups.get(box.path)!.index, true);
    [box.x, box.y, box.width, box.height].forEach((n, j) =>
      view.setFloat64(offset + 4 + j * 8, n, true),
    );
  });
  return buffer;
}
export function unpackGeometry(
  buffer: ArrayBuffer,
  input: LayoutInput,
): MapGeometry | undefined {
  const count = input.groups.size - (input.dominant ? 1 : 0);
  if (buffer.byteLength !== 4 + count * ROW_BYTES) return undefined;
  const view = new DataView(buffer);
  if (view.getUint32(0, true) !== MAGIC) return undefined;
  const dictionary = [...input.groups.values()],
    areas: MapBox[] = [],
    seen = new Map<string, MapBox>();
  for (let i = 0; i < count; i++) {
    const offset = 4 + i * ROW_BYTES,
      group = dictionary[view.getUint32(offset, true)];
    if (!group || group.path === input.dominant || seen.has(group.path))
      return undefined;
    const parent = group.parent === input.dominant ? "" : group.parent,
      above = seen.get(parent);
    if (parent && !above) return undefined;
    const box = {
      path: group.path,
      parent,
      x: view.getFloat64(offset + 4, true),
      y: view.getFloat64(offset + 12, true),
      width: view.getFloat64(offset + 20, true),
      height: view.getFloat64(offset + 28, true),
      depth: above ? above.depth + 1 : 1,
      color: above ? above.color : input.colors.get(group.path)!,
      files: group.files,
      declarations: group.weight,
    };
    if (!containsBox(box, above)) return undefined;
    seen.set(box.path, box);
    areas.push(box);
  }
  const geometry = { input: input.text, areas };
  return geometry;
}
