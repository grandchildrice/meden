import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  mapAreaBatches,
  navigationReady,
  mapLocation,
  visibleMapAreas,
  MAP_WIDTH,
  MAP_HEIGHT,
  type MapScene,
  type Symbol,
} from "./project-map-model";

import { MapAreaCanvas } from "./MapAreaCanvas";

import { declarationRoles } from "./declaration-role";
import {
  MapAreaCard,
  MapDeclaration,
  MapReference,
  MapMainPin,
  mapColors,
} from "./MapSceneElements";

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

function Scene({ data }: NodeProps<SceneNode>) {
  const v = useViewport();
  const [focusedArea, setFocusedArea] = useState<string>();
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
  return (
    <svg
      className="continuous-map-scene"
      width={MAP_WIDTH}
      height={MAP_HEIGHT}
      aria-label="Project in stable coordinates"
      data-scene-revision={data.layoutRevision}
      data-layout-source={data.layoutSource}
      onFocusCapture={(event) => {
        const path = (event.target as Element).getAttribute("data-map-path");
        setFocusedArea(path ?? undefined);
      }}
      onBlurCapture={(event) => {
        const next = event.relatedTarget;
        setFocusedArea(
          next instanceof Element && event.currentTarget.contains(next)
            ? (next.getAttribute("data-map-path") ?? undefined)
            : undefined,
        );
      }}
    >
      <g data-map-layer="areas">
        {/* Canvas retains every faded area. SVG needs only operable targets,
            plus the current focus until it leaves, even while fading out. */}
        {mapAreaBatches(
          visible.filter(
            ({ area, opacity }) => opacity >= 0.5 || area.path === focusedArea,
          ),
        ).map((batch) => (
          <g key={batch.key}>
            {batch.visible.map(({ area: a, opacity: alpha }) => (
              <MapAreaCard
                key={a.path}
                area={a}
                pointer={alpha >= 0.5}
                accessible={alpha > 0.5 || a.path === focusedArea}
                focus={data.focus}
              />
            ))}
          </g>
        ))}
      </g>
      <g data-map-layer="references">
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
              <MapReference
                key={`${edge.source}:${edge.target}`}
                from={a}
                to={b}
                opacity={alpha}
                zoom={v.zoom}
              />
            );
          })}
      </g>
      <g data-map-layer="declarations">
        {visible
          .filter((x) => x.area.file)
          .flatMap(({ area: a }) => {
            const alpha = fileGraphs.get(a.path) || 0;
            if (alpha < 0.01) return [];
            return a.symbols.map((symbol) => (
              <MapDeclaration
                key={symbol.id}
                symbol={symbol}
                opacity={alpha}
                zoom={v.zoom}
                selected={data.selected === symbol.id}
                select={data.select}
                label={(symbol.name.split(".").at(-1) || symbol.name).slice(
                  0,
                  Math.max(3, Math.floor((symbol.cellWidth * v.zoom - 20) / 6)),
                )}
              />
            ));
          })}
      </g>
      <g data-map-layer="targets">
        {data.scene.mainPins
          .filter((pin) => declarationDetail(pin.area, v.zoom, data) < 0.5)
          .map(({ area, symbols, origin }) => (
            <MapMainPin
              key={area.path}
              area={area}
              symbols={symbols}
              origin={origin}
              zoom={v.zoom}
              focus={data.focus}
            />
          ))}
      </g>
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
  requestGeneration,
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
  requestGeneration: string;
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
  const indexing = useRef<{ generation: string; path: string } | undefined>(
    undefined,
  );
  const currentGeneration = useRef(requestGeneration);
  currentGeneration.current = requestGeneration;
  useEffect(() => {
    requested.current.clear();
  }, [requestGeneration]);
  useEffect(() => {
    if (
      !active ||
      !canIndex ||
      !scene ||
      indexing.current?.generation === requestGeneration
    )
      return;
    const file = visible.find(
      ({ area, opacity }) =>
        area.file &&
        !navigationReady(area.file) &&
        opacity > 0.5 &&
        declarationDetail(area, camera.zoom, size) > 0.01 &&
        !requested.current.has(area.path),
    )?.area;
    if (!file) return;
    const request = { generation: requestGeneration, path: file.path };
    // Declarations are already prepared on disk; zoom only reads that cache.
    const timer = setTimeout(() => {
      indexing.current = request;
      void onIndex(file.path)
        .then((accepted) => {
          if (accepted && currentGeneration.current === request.generation)
            requested.current.add(file.path);
        })
        .finally(() => {
          if (indexing.current === request) indexing.current = undefined;
        });
    }, 0);
    return () => clearTimeout(timer);
  }, [
    active,
    canIndex,
    requestGeneration,
    scene,
    camera,
    size,
    visible,
    onIndex,
    state.overview.error,
  ]);
  const focus = useCallback(
    (path: string) => {
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
    },
    [scene?.index, flow],
  );
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
          {scene && <MapAreaCanvas scene={scene} {...size} />}
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
                fill={mapColors[a.color]}
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
