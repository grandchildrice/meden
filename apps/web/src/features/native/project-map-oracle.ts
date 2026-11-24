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
