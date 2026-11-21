import { useCallback, useEffect, useState, type CSSProperties } from "react";

type Widths = { files: number; chat: number };
const defaults: Widths = { files: 220, chat: 350 };
const minimum: Widths = { files: 160, chat: 280 };
const maximum: Widths = { files: 520, chat: 800 };
function fit(widths: Widths, available: number): Widths {
  let files = Math.max(minimum.files, Math.min(maximum.files, widths.files));
  let chat = Math.max(minimum.chat, Math.min(maximum.chat, widths.chat));
  if (available > 1150) {
    chat = Math.min(chat, Math.max(minimum.chat, available - files - 328));
    files = Math.min(files, available - chat - 328);
  } else if (available > 700) files = Math.min(files, available - 324);
  return { files, chat };
}
export function useSidebarResize(workspace?: string) {
  const [element, ref] = useState<HTMLElement | null>(null);
  const [available, setAvailable] = useState(window.innerWidth);
  const [preferred, setPreferred] = useState(defaults);
  const key = "meden-sidebar-widths:" + workspace;
  useEffect(() => {
    if (!workspace) return;
    try {
      const saved = JSON.parse(localStorage.getItem(key) || "null");
      setPreferred(
        saved && Number.isFinite(saved.files) && Number.isFinite(saved.chat)
          ? saved
          : defaults,
      );
    } catch {
      setPreferred(defaults);
    }
  }, [workspace, key]);
  useEffect(() => {
    if (!element) return;
    const observer = new ResizeObserver(() =>
      setAvailable(element.clientWidth),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [element]);
  const widths = fit(preferred, available);
  const change = useCallback(
    (side: keyof Widths, width: number) => {
      const other = side === "files" ? "chat" : "files";
      const limit =
        available > 1150 ? available - widths[other] - 328 : maximum[side];
      const next = {
        ...widths,
        [side]: Math.max(minimum[side], Math.min(maximum[side], limit, width)),
      };
      setPreferred(next);
      if (workspace) localStorage.setItem(key, JSON.stringify(next));
    },
    [widths.files, widths.chat, available, workspace, key],
  );
  return {
    ref,
    widths,
    change,
    style: {
      "--ide-files-width": widths.files + "px",
      "--ide-chat-width": widths.chat + "px",
    } as CSSProperties,
  };
}
export function SidebarResize({
  side,
  layout,
}: {
  side: keyof Widths;
  layout: ReturnType<typeof useSidebarResize>;
}) {
  const [drag, setDrag] = useState<{ x: number; width: number }>();
  const sign = side === "files" ? 1 : -1;
  return (
    <div
      role="separator"
      tabIndex={0}
      aria-label={
        side === "files" ? "Resize file sidebar" : "Resize chat sidebar"
      }
      aria-orientation="vertical"
      aria-valuenow={Math.round(layout.widths[side])}
      aria-valuemin={minimum[side]}
      aria-valuemax={maximum[side]}
      className={
        "sidebar-resize sidebar-resize-" + side + (drag ? " is-dragging" : "")
      }
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        setDrag({ x: event.clientX, width: layout.widths[side] });
      }}
      onPointerMove={(event) => {
        if (drag)
          layout.change(side, drag.width + sign * (event.clientX - drag.x));
      }}
      onPointerUp={(event) => {
        if (event.currentTarget.hasPointerCapture(event.pointerId))
          event.currentTarget.releasePointerCapture(event.pointerId);
        setDrag(undefined);
      }}
      onLostPointerCapture={() => setDrag(undefined)}
      onPointerCancel={() => setDrag(undefined)}
      onDoubleClick={() => layout.change(side, defaults[side])}
      onKeyDown={(event) => {
        if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
          event.preventDefault();
          layout.change(
            side,
            layout.widths[side] +
              (event.key === "ArrowRight" ? 1 : -1) *
                sign *
                (event.shiftKey ? 40 : 16),
          );
        } else if (event.key === "Home") {
          event.preventDefault();
          layout.change(side, defaults[side]);
        }
      }}
    />
  );
}
