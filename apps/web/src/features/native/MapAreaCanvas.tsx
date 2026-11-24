import { memo, useEffect, useLayoutEffect, useRef } from "react";
import { useViewport } from "@xyflow/react";
import {
  visibleMapAreas,
  type MapArea,
  type MapScene,
} from "./project-map-model";
import { mapAreaColor } from "./MapSceneElements";

const appearances = new WeakMap<MapArea, { color: string; name: string }>();
const appearance = (area: MapArea) => {
  let value = appearances.get(area);
  if (!value) {
    value = { color: mapAreaColor(area), name: area.path.split("/").at(-1)! };
    appearances.set(area, value);
  }
  return value;
};

/** Area paint only. SVG retains hit targets, keyboard semantics and the complete
 * declaration/reference layers; no camera sizing mutates every area's DOM.
 */
export const MapAreaCanvas = memo(function MapAreaCanvas({
  scene,
  width,
  height,
}: {
  scene: MapScene;
  width: number;
  height: number;
}) {
  const element = useRef<HTMLCanvasElement>(null),
    buffer = useRef<OffscreenCanvas>(undefined),
    draw = useRef<() => void>(() => {}),
    family = useRef<string>(undefined),
    focused = useRef<string>(undefined);
  const viewport = useViewport();
  useLayoutEffect(() => {
    const canvas = element.current!;
    const surface = (buffer.current ??= canvas.transferControlToOffscreen());
    const context = surface.getContext("2d");
    if (!context) throw Error("Project map drawing context unavailable");
    const drawCurrent = () => {
      const ratio = window.devicePixelRatio || 1;
      const w = Math.max(1, Math.round(width * ratio)),
        h = Math.max(1, Math.round(height * ratio));
      if (surface.width !== w) surface.width = w;
      if (surface.height !== h) surface.height = h;
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, width, height);
      context.font =
        "14px " + (family.current ??= getComputedStyle(canvas).fontFamily);
      context.textBaseline = "alphabetic";
      for (const { area, opacity } of visibleMapAreas(scene, viewport, {
        width,
        height,
      })) {
        const x = viewport.x + area.x * viewport.zoom,
          y = viewport.y + area.y * viewport.zoom,
          w = area.width * viewport.zoom,
          h = area.height * viewport.zoom;
        const paint = appearance(area);
        context.beginPath();
        context.roundRect(x, y, w, h, Math.min(3, w / 2, h / 2));
        context.fillStyle = paint.color;
        context.globalAlpha = opacity;
        context.fill();
        context.strokeStyle =
          focused.current === area.path ? "#ffe59c" : "#c7d9ee";
        context.lineWidth = 0.6;
        context.globalAlpha = 0.75 * opacity;
        context.stroke();
        if (opacity > 0.65 && w > 65 && h > 28) {
          context.fillStyle = "white";
          context.globalAlpha = 1;
          context.fillText(paint.name, x + 10, y + Math.min(h / 2, 25));
        }
      }
      context.globalAlpha = 1;
    };
    draw.current = drawCurrent;
    drawCurrent();
  }, [scene, viewport, width, height]);
  useEffect(() => {
    const canvas = element.current!,
      flow = canvas.closest(".react-flow")!;
    let pending = 0,
      alive = true;
    const redraw = () => {
      if (!pending)
        pending = requestAnimationFrame(() => {
          pending = 0;
          if (!alive) return;
          const active = document.activeElement;
          focused.current =
            active instanceof Element &&
            active.closest(".react-flow") === flow &&
            active.matches(":focus-visible")
              ? (active
                  .closest("[data-map-path]")
                  ?.getAttribute("data-map-path") ?? undefined)
              : undefined;
          draw.current();
        });
    };
    for (const event of ["focusin", "focusout", "keydown", "pointerdown"])
      flow.addEventListener(event, redraw);
    window.addEventListener("resize", redraw);
    void document.fonts.ready.then(() => {
      if (alive) redraw();
    });
    return () => {
      alive = false;
      cancelAnimationFrame(pending);
      window.removeEventListener("resize", redraw);
      for (const event of ["focusin", "focusout", "keydown", "pointerdown"])
        flow.removeEventListener(event, redraw);
    };
  }, []);
  return (
    <canvas
      ref={element}
      className="react-flow__background map-area-canvas"
      aria-hidden="true"
      style={{ width, height }}
    />
  );
});
