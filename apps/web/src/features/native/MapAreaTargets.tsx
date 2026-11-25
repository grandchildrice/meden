import { memo, useLayoutEffect, useRef, type SyntheticEvent } from "react";
import {
  fileIndexed,
  type MapArea,
  type mapAreaBatches,
} from "./project-map-model";

type Areas = ReturnType<typeof mapAreaBatches>[number]["visible"];
type Target = {
  element: SVGRectElement;
  area: MapArea;
  pointer: boolean;
  accessible: boolean;
};
const label = (area: MapArea) =>
  `${area.file ? (fileIndexed(area.file) ? "Dependencies" : "File") : "Category"}: ${area.path}`;
const geometry = (area: MapArea) => ({
  x: area.x,
  y: area.y,
  width: area.width,
  height: area.height,
  "data-world-x": area.x,
  "data-world-y": area.y,
});

/** One bounded SVG batch. React owns its parent; this disposable DOM adapter
 * retains exact targets and clones a fixed rectangle template. It owns no
 * layout, selection or source data and never replaces a retained element.
 */
class Targets {
  #parent: SVGGElement;
  #targets = new Map<string, Target>();
  #template: SVGRectElement;
  constructor(parent: SVGGElement) {
    this.#parent = parent;
    this.#template = parent.ownerDocument.createElementNS(
      "http://www.w3.org/2000/svg",
      "rect",
    );
    for (const [name, value] of Object.entries({
      fill: "transparent",
      stroke: "none",
      "stroke-opacity": "0",
      "pointer-events": "fill",
      class: "nodrag",
      role: "button",
      tabindex: "0",
      "aria-hidden": "false",
    }))
      this.#template.setAttribute(name, value);
  }
  update(areas: Areas, focused?: string) {
    const wanted = new Set(areas.map(({ area }) => area.path));
    for (const [path, target] of this.#targets)
      if (!wanted.has(path)) {
        target.element.remove();
        this.#targets.delete(path);
      }
    for (const { area } of areas)
      if (!this.#targets.has(area.path)) {
        const element = this.#template.cloneNode(false) as SVGRectElement;
        for (const [name, value] of Object.entries({
          ...geometry(area),
          "data-map-path": area.path,
          "aria-label": label(area),
        }))
          element.setAttribute(name, String(value));
        this.#targets.set(area.path, {
          element,
          area,
          pointer: true,
          accessible: true,
        });
      }
    let next: SVGRectElement | null = null;
    for (let i = areas.length - 1; i >= 0; i--) {
      const { area, opacity } = areas[i]!,
        target = this.#targets.get(area.path)!;
      const pointer = opacity >= 0.5,
        accessible = opacity > 0.5 || area.path === focused;
      if (target.area !== area) {
        const before = geometry(target.area);
        for (const [name, value] of Object.entries(geometry(area)))
          if (before[name as keyof typeof before] !== value)
            target.element.setAttribute(name, String(value));
        if (label(target.area) !== label(area))
          target.element.setAttribute("aria-label", label(area));
      }
      if (target.pointer !== pointer)
        target.element.setAttribute(
          "pointer-events",
          pointer ? "fill" : "none",
        );
      if (target.accessible !== accessible) {
        target.element.setAttribute("tabindex", accessible ? "0" : "-1");
        target.element.setAttribute("aria-hidden", String(!accessible));
      }
      // Insert before the retained successor; unchanged targets never move.
      if (
        target.element.parentNode !== this.#parent ||
        target.element.nextSibling !== next
      )
        this.#parent.insertBefore(target.element, next);
      target.area = area;
      target.pointer = pointer;
      target.accessible = accessible;
      next = target.element;
    }
  }
}

export const MapAreaTargets = memo(function MapAreaTargets({
  areas,
  focused,
  focus,
}: {
  areas: Areas;
  focused?: string;
  focus: (path: string) => void;
}) {
  const parent = useRef<SVGGElement>(null),
    targets = useRef<Targets>(undefined);
  useLayoutEffect(() => {
    (targets.current ??= new Targets(parent.current!)).update(areas, focused);
  }, [areas, focused]);
  const activate = (event: SyntheticEvent<SVGGElement>) => {
    if (
      !(event.target instanceof SVGRectElement) ||
      event.target.parentNode !== event.currentTarget
    )
      return;
    const path = event.target.getAttribute("data-map-path");
    if (path !== null) {
      event.stopPropagation();
      focus(path);
    }
  };
  return (
    <g
      onClick={activate}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          activate(event);
        }
      }}
      ref={parent}
    />
  );
});
