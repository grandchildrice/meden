import { useEffect, useMemo, useState } from "react";
import { Alert, Button, Typography } from "@mui/material";
import {
  ReactFlow,
  Background,
  Controls,
  Handle,
  Position,
  type Node,
  type NodeProps,
  type Edge,
  type ReactFlowInstance,
} from "@xyflow/react";
import type {
  NativeState,
  WorkspaceOverview,
} from "@meden/agent-protocol/native";

type File = WorkspaceOverview["files"][number];
type Tile = { path: string; files: File[]; folder: boolean; weight: number };
type Data = Record<string, unknown> &
  Tile & {
    running: boolean;
    width: number;
    height: number;
    color: number;
    open: (tile: Tile) => void;
  };
type MapNode = Node<Data, "tile">;
function MapTile({ data }: NodeProps<MapNode>) {
  return (
    <button
      className={`project-map-tile map-color-${data.color} nodrag`}
      style={{ width: data.width, height: data.height }}
      disabled={data.running}
      title={data.path}
      onClick={() => data.open(data)}
      aria-label={`${data.folder ? "カテゴリ" : "依存関係"}: ${data.path}`}
    >
      {!data.folder && <Handle type="target" position={Position.Left} />}
      <strong>{data.path.split("/").at(-1)}</strong>
      <span>
        {data.folder ? `${data.files.length}ファイル · ` : ""}
        {data.files.reduce((n, f) => n + f.declarations, 0)}宣言
      </span>
      <small>{data.files.reduce((n, f) => n + f.theorems, 0)}定理候補</small>
      {!data.folder && <Handle type="source" position={Position.Right} />}
    </button>
  );
}
const nodeTypes = { tile: MapTile };
/** Stable binary treemap. Area represents declarations, never proof authority. */
function partition(
  items: Tile[],
  x = 0,
  y = 0,
  width = 1000,
  height = 660,
): Array<{ tile: Tile; x: number; y: number; width: number; height: number }> {
  if (!items.length) return [];
  if (items.length === 1) return [{ tile: items[0]!, x, y, width, height }];
  const total = items.reduce((n, item) => n + item.weight, 0);
  let split = 1,
    left = items[0]!.weight;
  while (
    split < items.length - 1 &&
    left + items[split]!.weight / 2 < total / 2
  )
    left += items[split++]!.weight;
  const ratio = left / total;
  return width >= height
    ? [
        ...partition(items.slice(0, split), x, y, width * ratio, height),
        ...partition(
          items.slice(split),
          x + width * ratio,
          y,
          width * (1 - ratio),
          height,
        ),
      ]
    : [
        ...partition(items.slice(0, split), x, y, width, height * ratio),
        ...partition(
          items.slice(split),
          x,
          y + height * ratio,
          width,
          height * (1 - ratio),
        ),
      ];
}
export function NativeWorkspaceGraph({
  snapshot,
  state,
  onDependencies,
  onRefresh,
}: {
  snapshot?: WorkspaceOverview;
  state: NativeState;
  onDependencies: (file: string) => void;
  onRefresh: () => void;
}) {
  const [prefix, setPrefix] = useState("");
  const [canvas, setCanvas] = useState<HTMLDivElement | null>(null);
  const [flow, setFlow] = useState<ReactFlowInstance<MapNode>>();
  useEffect(() => {
    if (!canvas || !flow) return;
    const observer = new ResizeObserver(() => {
      if (canvas.clientWidth && canvas.clientHeight)
        void flow.fitView({ padding: 0.05 });
    });
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [canvas, flow]);
  const layout = useMemo(() => {
    const groups = new Map<string, File[]>();
    for (const file of snapshot?.files || []) {
      if (!file.path.startsWith(prefix)) continue;
      const path = prefix + file.path.slice(prefix.length).split("/")[0]!;
      groups.set(path, [...(groups.get(path) || []), file]);
    }
    // Skip a single container namespace to expose useful project categories.
    const dominant = [...groups].sort((a, b) => b[1].length - a[1].length)[0];
    if (
      dominant &&
      (groups.size === 1 ||
        (!prefix && dominant[1].length > (snapshot?.files.length || 0) * 0.8))
    ) {
      const [path, members] = dominant;
      if (members.some((f) => f.path !== path)) {
        const children = new Map<string, File[]>();
        for (const f of members) {
          const key = path + "/" + f.path.slice(path.length + 1).split("/")[0]!;
          children.set(key, [...(children.get(key) || []), f]);
        }
        groups.delete(path);
        for (const [key, value] of children) groups.set(key, value);
      }
    }
    const tiles: Tile[] = [...groups]
      .map(([path, files]) => ({
        path,
        files,
        folder: files.some((f) => f.path !== path),
        weight: files.reduce((n, f) => n + Math.max(1, f.declarations), 0),
      }))
      .sort((a, b) => b.weight - a.weight || a.path.localeCompare(b.path));
    const rectangles = partition(tiles);
    const nodes: MapNode[] = rectangles.map(
      ({ tile, x, y, width, height }, index) => ({
        id: tile.path,
        type: "tile",
        position: { x, y },
        data: {
          ...tile,
          running: state.running,
          width: Math.max(1, width - 5),
          height: Math.max(1, height - 5),
          color: index % 5,
          open: (item) =>
            item.folder
              ? setPrefix(item.path + "/")
              : onDependencies(item.path),
        },
        style: { width: width - 5, height: height - 5 },
      }),
    );
    const modules = new Map<string, string | null>();
    for (const file of snapshot?.files || []) {
      const parts = file.path.replace(/\.lean$/, "").split("/");
      for (let i = 0; i < parts.length; i++) {
        const name = parts.slice(i).join(".");
        modules.set(name, modules.has(name) ? null : file.path);
      }
    }
    const fileTiles = new Set(
      tiles.filter((t) => !t.folder).map((t) => t.path),
    );
    const edges: Edge[] = [];
    for (const tile of tiles.filter((t) => !t.folder))
      for (const name of tile.files[0]!.imports) {
        const target = modules.get(name);
        if (target && target !== tile.path && fileTiles.has(target))
          edges.push({
            id: `${tile.path}:${target}`,
            source: tile.path,
            target,
            type: "smoothstep",
            style: { stroke: "#c7d9ee", strokeWidth: 2 },
          });
      }
    return { nodes, edges, rectangles };
  }, [snapshot?.revision, prefix, onDependencies, state.running]);
  const files = snapshot?.files || [];
  return (
    <section aria-label="プロジェクトマップ" className="workspace-overview">
      <div className="project-map-heading">
        <div>
          <Typography variant="h3">プロジェクトマップ</Typography>
          <Typography variant="body2">
            {state.overview.state === "loading"
              ? `読み込み中：${state.overview.scanned} / ${state.overview.total}ファイル`
              : `${files.length}ファイル · ${files.reduce((n, f) => n + f.theorems, 0)}定理候補`}
          </Typography>
        </div>
        <Button disabled={state.running} onClick={onRefresh}>
          読み直す
        </Button>
      </div>
      {state.overview.error && (
        <Alert severity="error">
          全体グラフを読み込めませんでした。{state.overview.error}
        </Alert>
      )}
      {state.overview.state === "stale" && (
        <Alert severity="info">ファイルを読み直しています。</Alert>
      )}
      <nav className="map-breadcrumbs" aria-label="マップの階層">
        <Button onClick={() => setPrefix("")}>プロジェクト全体</Button>
        {prefix
          .split("/")
          .filter(Boolean)
          .map((part, index, parts) => (
            <Button
              key={index}
              onClick={() =>
                setPrefix(parts.slice(0, index + 1).join("/") + "/")
              }
            >
              / {part}
            </Button>
          ))}
      </nav>
      {snapshot && files.length > 0 && (
        <>
          <div className="workspace-overview-canvas" ref={setCanvas}>
            <ReactFlow
              key={`${snapshot.revision}:${prefix}`}
              nodes={layout.nodes}
              edges={layout.edges}
              nodeTypes={nodeTypes}
              onInit={setFlow}
              fitView
              fitViewOptions={{ padding: 0.05 }}
              minZoom={0.08}
              maxZoom={3}
              nodesDraggable={false}
              nodesConnectable={false}
              onMoveEnd={(event, viewport) => {
                if (!event || state.running || viewport.zoom < 1.5) return;
                const canvas = (event.target as Element)?.closest?.(
                  ".workspace-overview-canvas",
                );
                if (!canvas) return;
                const bounds = canvas.getBoundingClientRect();
                const pointer = event.type === "wheel" && "clientX" in event;
                const x =
                  ((pointer
                    ? event.clientX - bounds.left
                    : canvas.clientWidth / 2) -
                    viewport.x) /
                  viewport.zoom;
                const y =
                  ((pointer
                    ? event.clientY - bounds.top
                    : canvas.clientHeight / 2) -
                    viewport.y) /
                  viewport.zoom;
                const hit = layout.rectangles.find(
                  (r) =>
                    x >= r.x &&
                    x <= r.x + r.width &&
                    y >= r.y &&
                    y <= r.y + r.height,
                );
                if (hit?.tile.folder) setPrefix(hit.tile.path + "/");
                else if (hit) onDependencies(hit.tile.path);
              }}
              aria-label="カテゴリ別の依存マップ"
            >
              <Background color="#334353" />
              <Controls showInteractive={false} />
            </ReactFlow>
          </div>
          <Typography variant="caption" component="p">
            面積＝宣言数（最小1）。カテゴリをクリック、またはズームして詳細へ。ファイルをクリックすると宣言の依存関係を開きます。線はソースのimport候補、定理数は証明済み件数ではありません。
          </Typography>
        </>
      )}
      {snapshot && !files.length && (
        <p>
          Rust／Leanソースがありません。コードタブでプロジェクトのファイルを開けます。
        </p>
      )}
    </section>
  );
}
