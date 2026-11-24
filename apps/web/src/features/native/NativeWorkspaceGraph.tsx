import { memo, useEffect, useMemo, useRef, useState } from "react";
import { Alert, Button, LinearProgress } from "@mui/material";
import {
  ReactFlow,
  Controls,
  useViewport,
  type Node,
  type NodeProps,
  type ReactFlowInstance,
} from "@xyflow/react";
import type { NativeState } from "@meden/agent-protocol/native";
import {
  declarationDetail,
  mapHasDeclarations,
  mapLocation,
  visibleMapAreas,
  MAP_WIDTH,
  MAP_HEIGHT,
  type MapScene,
  type Symbol,
} from "./project-map-model";

import { declarationRoles } from "./declaration-role";

function SymbolMarker({
  symbol,
  zoom,
  selected,
}: {
  symbol: Symbol;
  zoom: number;
  selected: boolean;
}) {
  const r = selected ? 8 : 6;
  const style = {
    fill: symbol.role.color,
    stroke: selected ? "#fff2ad" : "#263a46",
    strokeWidth: selected ? 2 : 1,
  };
  const shape = symbol.role.shape;
  return (
    <g transform={`translate(${symbol.x}, ${symbol.y}) scale(${1 / zoom})`}>
      {shape === "circle" ? (
        <circle r={r} {...style} />
      ) : shape === "square" ? (
        <rect x={-r} y={-r} width={r * 2} height={r * 2} rx={1} {...style} />
      ) : (
        <path
          d={
            shape === "diamond"
              ? `M 0 ${-r - 2} L ${r + 1} 0 L 0 ${r + 2} L ${-r - 1} 0 Z`
              : shape === "triangle"
                ? `M 0 ${-r - 2} L ${r + 2} ${r} L ${-r - 2} ${r} Z`
                : Array.from({ length: 10 }, (_, i) => {
                    const angle = (i * Math.PI) / 5 - Math.PI / 2,
                      radius = i % 2 ? r * 0.45 : r + 2;
                    return `${i ? "L" : "M"} ${Math.cos(angle) * radius} ${Math.sin(angle) * radius}`;
                  }).join(" ") + " Z"
          }
          {...style}
        />
      )}
    </g>
  );
}

type SceneData = Record<string, unknown> & {
  scene: MapScene;
  layoutRevision?: string;
  layoutSource?: string;
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
  const visible = visibleMapAreas(data.scene, v, data);
  const fileGraphs = new Map(
    visible
      .filter((x) => x.area.file)
      .map((x) => [
        x.area.path,
        x.opacity * declarationDetail(x.area, v.zoom, data),
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
      aria-label="Project in stable coordinates"
      data-scene-revision={data.layoutRevision}
      data-layout-source={data.layoutSource}
    >
      {visible.map(({ area: a, opacity: alpha }) => (
        <g
          key={a.path}
          pointerEvents={alpha < 0.5 ? "none" : undefined}
          className="nodrag"
          data-map-path={a.path}
          data-world-x={a.x}
          data-world-y={a.y}
          role="button"
          tabIndex={alpha > 0.5 ? 0 : -1}
          aria-hidden={alpha <= 0.5}
          aria-label={`${a.file ? (a.file.indexed ? "Dependencies" : "File") : "Category"}: ${a.path}`}
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
            fill={
              a.depth === 1
                ? colors[a.color]
                : `hsl(${(a.color * 67 + a.depth * 31 + (a.path.split("").reduce((n, c) => n + c.charCodeAt(0), 0) % 47)) % 360} 32% 28%)`
            }
            fillOpacity={alpha}
            stroke="#c7d9ee"
            strokeOpacity={0.75 * alpha}
            strokeWidth={0.6 / v.zoom}
          />
          {alpha > 0.65 && a.width * v.zoom > 65 && a.height * v.zoom > 28 && (
            <g pointerEvents="none">
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
              aria-label={`Declaration: ${symbol.name} · ${symbol.file}:${symbol.line}`}
              aria-description={symbol.role.label}
              data-declaration-role={symbol.role.id}
              opacity={alpha}
              data-symbol-id={symbol.id}
              onClick={(e) => {
                e.stopPropagation();
                data.select(symbol);
              }}
              onKeyDown={(e) => keyboard(e, () => data.select(symbol))}
            >
              <title>{symbol.role.label + " · " + symbol.name}</title>
              <SymbolMarker
                symbol={symbol}
                zoom={v.zoom}
                selected={data.selected === symbol.id}
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
      {data.scene.mainPins
        .filter((pin) => declarationDetail(pin.area, v.zoom, data) < 0.5)
        .map(({ area, symbols, origin }) => (
          <g
            key={area.path}
            className="nodrag map-main-pin"
            role="button"
            tabIndex={0}
            aria-label={`Main theorems: ${area.path} (${symbols.length})`}
            transform={`translate(${area.x + area.width / 2}, ${area.y + area.height / 2}) scale(${1 / v.zoom})`}
            onClick={(event) => {
              event.stopPropagation();
              data.focus(area.path);
            }}
            onKeyDown={(event) => keyboard(event, () => data.focus(area.path))}
          >
            <title>
              {origin + ": " + symbols.map((s) => s.name).join(", ")}
            </title>
            <circle r={13} fill="#132631" stroke="#ffdc78" strokeWidth={1.5} />
            <text textAnchor="middle" y={5} fill="#ffdc78" fontSize={18}>
              ★
            </text>
            {symbols.length > 1 && (
              <text x={16} y={4} fill="#ffdc78" fontSize={11}>
                {symbols.length}
              </text>
            )}
          </g>
        ))}
    </svg>
  );
}
const nodeTypes = { scene: Scene };

function WorkspaceMap({
  scene,
  layoutRevision,
  layoutSource,
  state,
  selectedId,
  onSelect,
  active,
  canIndex,
  onIndex,
  error,
  loading,
  onRetry,
}: {
  scene?: MapScene;
  layoutRevision?: string;
  layoutSource?: string;
  state: NativeState;
  selectedId?: string;
  onSelect: (symbol: Symbol) => void;
  active: boolean;
  canIndex: boolean;
  onIndex: (path: string) => Promise<boolean>;
  error?: string;
  loading?: boolean;
  onRetry?: () => void;
}) {
  const [canvas, setCanvas] = useState<HTMLDivElement | null>(null);
  const [flow, setFlow] = useState<ReactFlowInstance<SceneNode>>();
  const [size, setSize] = useState({ width: 1, height: 1 });
  const [camera, setCamera] = useState({ x: 0, y: 0, zoom: 0.08 });
  const visible = useMemo(
    () => (scene ? visibleMapAreas(scene, camera, size) : []),
    [scene, camera, size],
  );
  const location = useMemo(
    () => (scene ? mapLocation(scene, camera, size) : ""),
    [scene, camera, size],
  );
  const initial = useRef<string | undefined>(undefined);
  const requested = useRef(new Set<string>());
  const indexing = useRef(false);
  useEffect(() => {
    if (state.overview.state === "loading" || !state.overview.error)
      requested.current.clear();
  }, [state.overview.state, state.overview.error, state.overview.revision]);
  useEffect(() => {
    if (!active || !canIndex || !scene?.inventoryOnly || indexing.current)
      return;
    const file = visible.find(
      ({ area, opacity }) =>
        area.file &&
        !area.file.indexed &&
        opacity > 0.5 &&
        declarationDetail(area, camera.zoom, size) > 0.01 &&
        !requested.current.has(area.path),
    )?.area;
    if (!file) return;
    // Declarations are already prepared on disk; zoom only reads that cache.
    const timer = setTimeout(() => {
      indexing.current = true;
      void onIndex(file.path)
        .then((accepted) => {
          if (accepted) requested.current.add(file.path);
        })
        .finally(() => {
          indexing.current = false;
        });
    }, 0);
    return () => clearTimeout(timer);
  }, [
    active,
    canIndex,
    scene,
    camera,
    size,
    visible,
    onIndex,
    state.overview.error,
  ]);
  const focus = (path: string) => {
    const a = scene?.index.byPath.get(path);
    if (flow)
      void flow.fitBounds(
        a
          ? { x: a.x, y: a.y, width: a.width, height: a.height }
          : { x: 0, y: 0, width: MAP_WIDTH, height: MAP_HEIGHT },
        {
          padding: "28px",
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
      const identity = layoutRevision || scene?.index.geometry.input;
      if (scene && initial.current !== identity) {
        initial.current = identity;
        void flow.fitBounds(
          { x: 0, y: 0, width: MAP_WIDTH, height: MAP_HEIGHT },
          { padding: "32px" },
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
  }, [canvas, flow, scene?.index.geometry.input, layoutRevision]);
  const nodes = useMemo<SceneNode[]>(
    () =>
      scene
        ? [
            {
              id: "project-scene",
              type: "scene",
              position: { x: 0, y: 0 },
              width: MAP_WIDTH,
              height: MAP_HEIGHT,
              style: { width: MAP_WIDTH, height: MAP_HEIGHT },
              data: {
                scene,
                layoutRevision,
                layoutSource,
                ...size,
                selected: selectedId,
                focus,
                select: onSelect,
              },
            },
          ]
        : [],
    [scene, size, flow, onSelect, selectedId, layoutRevision, layoutSource],
  );
  return (
    <section
      aria-label="Project map"
      className="workspace-overview continuous-map"
    >
      <div className="workspace-overview-canvas" ref={setCanvas}>
        {location && (
          <nav className="map-breadcrumbs" aria-label="Map location">
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
        )}
        {(loading ||
          state.overview.state === "loading" ||
          (active && state.running && state.operation === "inspection")) && (
          <LinearProgress
            className="map-load-progress"
            aria-label={
              loading
                ? "Loading prepared map"
                : state.overview.state === "loading"
                  ? "Preparing map"
                  : "Loading declarations"
            }
          />
        )}
        {(error || state.overview.error) && (
          <Alert className="map-load-error" severity="error">
            {state.overview.state === "failed" && "Could not refresh the map. "}
            {error || state.overview.error}
            {onRetry && <Button onClick={onRetry}>Retry map</Button>}
          </Alert>
        )}
        <ReactFlow
          proOptions={{ hideAttribution: true }}
          nodes={nodes}
          nodeTypes={nodeTypes}
          onInit={setFlow}
          minZoom={0.003}
          maxZoom={scene?.maxZoom || 500}
          nodesDraggable={false}
          nodesConnectable={false}
          zoomOnDoubleClick={false}
          onMove={(_, v) => setCamera(v)}
          aria-label="Continuous project map"
        >
          <Controls
            showInteractive={false}
            fitViewOptions={{ padding: "32px" }}
          />
        </ReactFlow>
        {mapHasDeclarations(visible, camera.zoom, size) && (
          <div
            className="map-declaration-legend"
            aria-label="Declaration types"
          >
            {declarationRoles.map((role) => (
              <span
                key={role.id}
                title={
                  role.id === "main_theorem"
                    ? "Saved target, compiled main-theorem attribute or theorem in the project entry module. This does not indicate verified status."
                    : role.label
                }
              >
                <b style={{ color: role.color }}>{role.glyph}</b> {role.label}
              </span>
            ))}
          </div>
        )}
        {scene && (
          <svg
            className="project-minimap"
            viewBox={`0 0 ${MAP_WIDTH} ${MAP_HEIGHT}`}
            aria-label="Position in the whole project"
          >
            {(scene.index.children.get("") || []).map((a) => (
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

// Hidden tabs retain their camera but do no parent-driven map work. Activation
// always supplies the latest scene, callbacks and native status.
export const NativeWorkspaceGraph = memo(
  WorkspaceMap,
  (previous, next) => !previous.active && !next.active,
);
