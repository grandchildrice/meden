// Test-only complete scans, independent of production branch pruning.
import {
  declarationDetail,
  fade,
  MAP_WIDTH,
  MAP_HEIGHT,
  type MapScene,
} from "./project-map-model";

export type NavigationInventory = {
  generation: string;
  excluded: string[];
  inventory_only: boolean;
  files: import("./project-map-model").NavigationFile[];
};

// Deliberately scan every area, without the production parent/children index.
export function scan(
  scene: MapScene,
  view: { x: number; y: number; zoom: number },
  size: { width: number; height: number },
) {
  const opacity = new Map([["", 1]]);
  const left = -view.x / view.zoom,
    top = -view.y / view.zoom;
  const result: Array<{
    area: import("./project-map-model").MapArea;
    opacity: number;
  }> = [];
  for (const base of scene.index.byPath.values()) {
    const area = scene.descriptions.get(base.path) || base;
    const parent = scene.index.byPath.get(area.parent);
    const alpha = parent
      ? opacity.get(parent.path)! *
        fade(Math.max(parent.width, parent.height) * view.zoom, 650, 950)
      : 1;
    opacity.set(area.path, alpha);
    if (
      alpha < 0.015 ||
      area.x > left + size.width / view.zoom ||
      area.y > top + size.height / view.zoom ||
      area.x + area.width < left ||
      area.y + area.height < top ||
      ((area.width * view.zoom < 2 || area.height * view.zoom < 2) &&
        !(area.main && declarationDetail(area, view.zoom, size) > 0.01))
    )
      continue;
    result.push({ area, opacity: alpha });
  }
  return result;
}

// Retired sibling walk, retained only as the measured pre-index counterfactual.
// Keep the independent complete scan above as the correctness oracle.
export function walkVisibility(
  scene: MapScene,
  view: { x: number; y: number; zoom: number },
  size: { width: number; height: number },
) {
  const left = -view.x / view.zoom,
    top = -view.y / view.zoom,
    right = left + size.width / view.zoom,
    bottom = top + size.height / view.zoom,
    queue = (scene.index.children.get("") || []).map((area) => ({
      area,
      opacity: 1,
    })),
    visible: Array<(typeof queue)[number]> = [];
  for (let i = 0; i < queue.length; i++) {
    const { area: base, opacity } = queue[i]!,
      area = scene.descriptions.get(base.path) || base;
    if (
      opacity < 0.015 ||
      area.x > right ||
      area.y > bottom ||
      area.x + area.width < left ||
      area.y + area.height < top
    )
      continue;
    if (
      (area.width * view.zoom >= 2 && area.height * view.zoom >= 2) ||
      (area.main && declarationDetail(area, view.zoom, size) > 0.01)
    )
      visible.push({ area, opacity });
    const childOpacity =
      opacity * fade(Math.max(area.width, area.height) * view.zoom, 650, 950);
    if (childOpacity >= 0.015)
      for (const child of scene.index.children.get(area.path) || [])
        queue.push({ area: child, opacity: childOpacity });
  }
  return visible;
}

// Original breadcrumb rules, independently walking the complete flat inventory.
export function scanLocation(
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
  const opacity = new Map([["", 1]]);
  let location = "";
  for (const area of scene.index.byPath.values()) {
    const parent = scene.index.byPath.get(area.parent);
    const alpha = parent
      ? opacity.get(parent.path)! *
        fade(Math.min(parent.width, parent.height) * view.zoom)
      : 1;
    opacity.set(area.path, alpha);
    if (
      alpha > 0.5 &&
      x >= area.x &&
      x <= area.x + area.width &&
      y >= area.y &&
      y <= area.y + area.height
    )
      location = area.path;
  }
  return location;
}

import type { ReferenceSymbol } from "./project-map-references";

// Independent original array/filter algorithm. Kept only as an outcome oracle
// and a measured counterfactual, never as a production fallback.
export function scanReferences(symbols: ReadonlyMap<string, ReferenceSymbol>) {
  const names = new Map<string, ReferenceSymbol[]>(),
    local = new Map<string, ReferenceSymbol[]>();
  for (const symbol of symbols.values()) {
    names.set(symbol.name, [...(names.get(symbol.name) || []), symbol]);
    const key = symbol.file + ":" + symbol.name.split(".").at(-1);
    local.set(key, [...(local.get(key) || []), symbol]);
  }
  const edges: Array<{ source: string; target: string }> = [],
    seen = new Set<string>();
  for (const symbol of symbols.values())
    for (const reference of symbol.references) {
      const parts = symbol.name.split(".").slice(0, -1),
        candidates = [
          ...parts.map(
            (_, i) =>
              parts.slice(0, parts.length - i).join(".") + "." + reference,
          ),
          reference,
        ];
      let target: ReferenceSymbol | undefined;
      for (const name of candidates) {
        const visible = (names.get(name) || []).filter(
          (s) => !s.private || s.file === symbol.file,
        );
        if (visible.length === 1) {
          target = visible[0];
          break;
        }
      }
      const fallback = local.get(symbol.file + ":" + reference);
      target ||= fallback?.length === 1 ? fallback[0] : undefined;
      if (
        target &&
        target.id !== symbol.id &&
        !seen.has(symbol.id + "->" + target.id)
      ) {
        seen.add(symbol.id + "->" + target.id);
        edges.push({ source: symbol.id, target: target.id });
      }
    }
  return edges;
}
