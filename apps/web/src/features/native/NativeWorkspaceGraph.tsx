import { useEffect, useMemo, useRef, useState } from "react";
import { Alert, Button, Typography } from "@mui/material";
import {
  ReactFlow,
  Controls,
  useViewport,
  type Node,
  type NodeProps,
  type ReactFlowInstance,
} from "@xyflow/react";
import type {
  NativeState,
  WorkspaceOverview,
} from "@meden/agent-protocol/native";
import {
  projectMap,
  fade,
  MAP_WIDTH,
  MAP_HEIGHT,
  type MapScene,
  type MapArea,
  type Symbol,
} from "./project-map-model";

type SceneData = Record<string, unknown> & {
  scene: MapScene;
  width: number;
  height: number;
  selected?: string;
  focus: (path: string) => void;
  select: (symbol: Symbol) => void;
};
type SceneNode = Node<SceneData, "scene">;
const colors = ["#275f69", "#414b7e", "#73516c", "#695a38", "#365b51"];
function Scene({ data }: NodeProps<SceneNode>) {
  const v = useViewport();
  const left = -v.x / v.zoom,
    top = -v.y / v.zoom,
    right = left + data.width / v.zoom,
    bottom = top + data.height / v.zoom;
  const opacity = new Map<string, number>([["", 1]]);
  const visible: Array<{ area: MapArea; opacity: number; detail: number }> = [];
  for (const area of data.scene.areas) {
    const parent = data.scene.byPath.get(area.parent);
    const alpha = parent
      ? (opacity.get(parent.path) || 0) *
        fade(Math.min(parent.width, parent.height) * v.zoom)
      : 1;
    opacity.set(area.path, alpha);
    if (
      alpha < 0.015 ||
      area.x > right ||
      area.y > bottom ||
      area.x + area.width < left ||
      area.y + area.height < top ||
      area.width * v.zoom < 2 ||
      area.height * v.zoom < 2
    )
      continue;
    visible.push({
      area,
      opacity: alpha,
      detail: fade(Math.min(area.width, area.height) * v.zoom),
    });
  }
  const fileGraphs = new Map(
    visible
      .filter((x) => x.area.file)
      .map((x) => [
        x.area.path,
        x.opacity *
          fade(Math.min(x.area.width, x.area.height) * v.zoom, 320, 500),
      ]),
  );
  const edgeNodes = new Set(
    [...fileGraphs].filter(([, alpha]) => alpha > 0.01).map(([path]) => path),
  );
  const keyboard = (event: React.KeyboardEvent, action: () => void) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      action();
    }
  };
  return (
    <svg
      className="continuous-map-scene"
      width={MAP_WIDTH}
      height={MAP_HEIGHT}
      aria-label="同じ座標に保持されたプロジェクト"
      data-scene-revision="fixed"
    >
      {visible.map(({ area: a, opacity: alpha, detail }) => (
        <g
          key={a.path}
          opacity={alpha}
          className="nodrag"
          data-map-path={a.path}
          data-world-x={a.x}
          data-world-y={a.y}
          role="button"
          tabIndex={alpha > 0.5 ? 0 : -1}
          aria-hidden={alpha <= 0.5}
          aria-label={`${a.file ? "依存関係" : "カテゴリ"}: ${a.path}`}
          onClick={(e) => {
            e.stopPropagation();
            data.focus(a.path);
          }}
          onKeyDown={(e) => keyboard(e, () => data.focus(a.path))}
        >
          <rect
            x={a.x}
            y={a.y}
            width={a.width}
            height={a.height}
            rx={3 / v.zoom}
            fill={colors[a.color]}
            stroke="#c7d9ee"
            strokeOpacity={0.28}
            strokeWidth={0.6 / v.zoom}
          />
          {a.width * v.zoom > 65 && a.height * v.zoom > 28 && (
            <g pointerEvents="none" opacity={a.file ? 1 : 1 - detail * 0.6}>
              <text
                x={a.x + 10 / v.zoom}
                y={a.y + Math.min(a.height * 0.5, 25 / v.zoom)}
                fill="white"
                fontSize={14 / v.zoom}
              >
                {a.path.split("/").at(-1)}
              </text>
            </g>
          )}
        </g>
      ))}
      {[...edgeNodes]
        .flatMap((file) => data.scene.edgesByFile.get(file) || [])
        .map((edge) => {
          const a = data.scene.symbols.get(edge.source)!,
            b = data.scene.symbols.get(edge.target)!;
          if (!edgeNodes.has(a.file) || !edgeNodes.has(b.file)) return null;
          const alpha = Math.min(
            fileGraphs.get(a.file) || 0,
            fileGraphs.get(b.file) || 0,
          );
          return (
            <line
              key={`${edge.source}:${edge.target}`}
              x1={a.x}
              y1={a.y}
              x2={b.x}
              y2={b.y}
              stroke="#bbd4de"
              strokeOpacity={alpha * 0.65}
              strokeWidth={1 / v.zoom}
              pointerEvents="none"
            />
          );
        })}
      {visible
        .filter((x) => x.area.file)
        .flatMap(({ area: a }) => {
          const alpha = fileGraphs.get(a.path) || 0;
          if (alpha < 0.01) return [];
          return a.symbols.map((symbol) => (
            <g
              key={symbol.id}
              className="nodrag"
              role="button"
              tabIndex={alpha > 0.5 ? 0 : -1}
              aria-hidden={alpha <= 0.5}
              aria-label={`宣言: ${symbol.name} · ${symbol.file}:${symbol.line}`}
              opacity={alpha}
              data-symbol-id={symbol.id}
              onClick={(e) => {
                e.stopPropagation();
                data.select(symbol);
              }}
              onKeyDown={(e) => keyboard(e, () => data.select(symbol))}
            >
              <title>{symbol.name}</title>
              <circle
                cx={symbol.x}
                cy={symbol.y}
                r={(data.selected === symbol.id ? 7 : 5) / v.zoom}
                fill={symbol.kind === "theorem" ? "#b7e6cd" : "#e0eafa"}
                stroke={data.selected === symbol.id ? "#ffe59c" : "#2c4855"}
                strokeWidth={2 / v.zoom}
              />
              <text
                x={symbol.x + 9 / v.zoom}
                y={symbol.y + 4 / v.zoom}
                fill="#fff"
                fontSize={11 / v.zoom}
              >
                {(symbol.name.split(".").at(-1) || symbol.name).slice(
                  0,
                  Math.max(3, Math.floor((symbol.cellWidth * v.zoom - 20) / 6)),
                )}
              </text>
            </g>
          ));
        })}
    </svg>
  );
}
const nodeTypes = { scene: Scene };

export function NativeWorkspaceGraph({
  snapshot,
  state,
  focusPath,
  focusSequence = 0,
  selectedId,
  onSelect,
  onRefresh,
}: {
  snapshot?: WorkspaceOverview;
  state: NativeState;
  focusPath?: string;
  focusSequence?: number;
  selectedId?: string;
  onSelect: (symbol: Symbol) => void;
  onRefresh: () => void;
}) {
  const scene = useMemo(
    () => (snapshot ? projectMap(snapshot) : undefined),
    [snapshot?.revision],
  );
  const [canvas, setCanvas] = useState<HTMLDivElement | null>(null);
  const [flow, setFlow] = useState<ReactFlowInstance<SceneNode>>();
  const [size, setSize] = useState({ width: 1, height: 1 });
  const [location, setLocation] = useState("");
  const [camera, setCamera] = useState({ x: 0, y: 0, zoom: 0.08 });
  const initial = useRef(false);
  const handledFocus = useRef(0);
  const focus = (path: string) => {
    const a = scene?.byPath.get(path);
    if (flow)
      void flow.fitBounds(
        a
          ? { x: a.x, y: a.y, width: a.width, height: a.height }
          : { x: 0, y: 0, width: MAP_WIDTH, height: MAP_HEIGHT },
        {
          padding: 0.12,
          duration: window.matchMedia("(prefers-reduced-motion: reduce)")
            .matches
            ? 0
            : 650,
        },
      );
  };
  useEffect(() => {
    if (!canvas || !flow) return;
    let previous = { width: 0, height: 0 };
    const observer = new ResizeObserver(() => {
      const width = canvas.clientWidth,
        height = canvas.clientHeight;
      if (!width || !height) return;
      setSize({ width, height });
      if (!initial.current && scene) {
        initial.current = true;
        void flow.fitBounds(
          { x: 0, y: 0, width: MAP_WIDTH, height: MAP_HEIGHT },
          { padding: 0.05 },
        );
      } else if (
        previous.width &&
        (previous.width !== width || previous.height !== height)
      ) {
        const v = flow.getViewport();
        void flow.setViewport({
          ...v,
          x: v.x + (width - previous.width) / 2,
          y: v.y + (height - previous.height) / 2,
        });
      }
      previous = { width, height };
    });
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [canvas, flow, Boolean(scene)]);
  useEffect(() => {
    if (flow && scene && focusSequence !== handledFocus.current) {
      handledFocus.current = focusSequence;
      focus(focusPath || "");
    }
  }, [flow, scene, focusPath, focusSequence]);
  const nodes = useMemo<SceneNode[]>(
    () =>
      scene
        ? [
            {
              id: "project-scene",
              type: "scene",
              position: { x: 0, y: 0 },
              style: { width: MAP_WIDTH, height: MAP_HEIGHT },
              data: {
                scene,
                ...size,
                selected: selectedId,
                focus,
                select: onSelect,
              },
            },
          ]
        : [],
    [scene, size, flow, onSelect, selectedId],
  );
  return (
    <section
      aria-label="プロジェクトマップ"
      className="workspace-overview continuous-map"
    >
      <div className="project-map-heading">
        <Typography variant="caption">
          {state.overview.state === "loading" ? "地図を準備中…" : ""}
        </Typography>
        <Button onClick={onRefresh} disabled={state.running}>
          読み直す
        </Button>
      </div>
      {state.overview.error && (
        <Alert severity="error">
          地図を更新できませんでした。{state.overview.error}
        </Alert>
      )}
      <nav className="map-breadcrumbs" aria-label="マップの位置">
        <Button onClick={() => focus("")}>プロジェクト全体</Button>
        {location
          .split("/")
          .filter(Boolean)
          .map((part, i, parts) => (
            <Button
              key={i}
              onClick={() => focus(parts.slice(0, i + 1).join("/"))}
            >
              / {part}
            </Button>
          ))}
      </nav>
      <div className="workspace-overview-canvas" ref={setCanvas}>
        <ReactFlow
          nodes={nodes}
          nodeTypes={nodeTypes}
          onInit={setFlow}
          minZoom={0.003}
          maxZoom={500}
          nodesDraggable={false}
          nodesConnectable={false}
          zoomOnDoubleClick={false}
          onMove={(_, v) => setCamera(v)}
          onMoveEnd={(_, v) => {
            if (!scene) return;
            const x = (size.width / 2 - v.x) / v.zoom,
              y = (size.height / 2 - v.y) / v.zoom;
            if (
              v.zoom <=
              Math.min(size.width / MAP_WIDTH, size.height / MAP_HEIGHT) * 1.2
            ) {
              setLocation("");
              return;
            }
            const visibility = new Map<string, number>([["", 1]]);
            let a: MapArea | undefined;
            for (const area of scene.areas) {
              const parent = scene.byPath.get(area.parent);
              const alpha = parent
                ? (visibility.get(parent.path) || 0) *
                  fade(Math.min(parent.width, parent.height) * v.zoom)
                : 1;
              visibility.set(area.path, alpha);
              if (
                alpha > 0.5 &&
                x >= area.x &&
                x <= area.x + area.width &&
                y >= area.y &&
                y <= area.y + area.height
              )
                a = area;
            }
            setLocation(a?.path || "");
          }}
          aria-label="連続ズームのプロジェクトマップ"
        >
          <Controls showInteractive={false} />
        </ReactFlow>
        {scene && (
          <svg
            className="project-minimap"
            viewBox={`0 0 ${MAP_WIDTH} ${MAP_HEIGHT}`}
            aria-label="全体に対する現在位置"
          >
            {scene.areas
              .filter((a) => a.depth === 1)
              .map((a) => (
                <rect
                  key={a.path}
                  x={a.x}
                  y={a.y}
                  width={a.width}
                  height={a.height}
                  fill={colors[a.color]}
                />
              ))}
            <rect
              x={-camera.x / camera.zoom}
              y={-camera.y / camera.zoom}
              width={size.width / camera.zoom}
              height={size.height / camera.zoom}
              fill="none"
              stroke="#ffe59c"
              strokeWidth={50}
            />
          </svg>
        )}
      </div>
    </section>
  );
}
