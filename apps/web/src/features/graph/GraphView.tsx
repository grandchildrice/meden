import {
  useEffect,
  useMemo,
  useState,
  useRef,
  type RefObject,
  type CSSProperties,
} from "react";
import {
  Autocomplete,
  IconButton,
  MenuItem,
  Select,
  TextField,
  Tooltip,
} from "@mui/material";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import {
  ReactFlow,
  ReactFlowProvider,
  useViewport,
  useReactFlow,
  Controls,
  Handle,
  Position,
  BaseEdge,
  getBezierPath,
  type Node,
  type NodeProps,
  type EdgeProps,
} from "@xyflow/react";
import { radialLayout, type GraphScope } from "@meden/atlas-adapter";
import type { Workspace, Declaration } from "@meden/workspace-schema";

type RadarData = Record<string, unknown> & {
  declaration: Declaration;
  root: boolean;
  active: boolean;
  radii: number[];
  select: () => void;
  center: () => void;
};
type RadarNode = Node<RadarData, "declaration">;
const nodeTypes = { declaration: DeclarationNode };
const edgeTypes = { dependency: DependencyEdge };
export function GraphView({
  workspace,
  focusId,
  selectedId,
  onSelect,
}: {
  workspace: Workspace;
  focusId?: string;
  selectedId?: string;
  onSelect: (declaration: Declaration) => void;
}) {
  const [root, setRoot] = useState(
    focusId ||
      workspace.declarations.find(
        (item) => item.id === "KernelProof.actual_spec",
      )?.id ||
      workspace.declarations[0]?.id ||
      "",
  );
  const canvas = useRef<HTMLDivElement>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [scope, setScope] = useState<GraphScope>("project");
  const center = (id: string) => {
    if (id !== root) {
      setHistory((previous) => [...previous, root]);
      setRoot(id);
    }
  };
  useEffect(() => {
    if (focusId && workspace.declarations.some((item) => item.id === focusId)) {
      if (focusId !== root) setHistory((previous) => [...previous, root]);
      setRoot(focusId);
    }
  }, [focusId, workspace.declarations]);
  const positions = useMemo(
    () => radialLayout(workspace, root, scope),
    [workspace, root, scope],
  );
  const radii = [
    ...new Set(
      [...positions.values()]
        .filter((p) => p.depth > 0)
        .map((p) => Math.round(Math.hypot(p.x, p.y))),
    ),
  ].sort((a, b) => a - b);
  const nodes: RadarNode[] = workspace.declarations
    .filter((item) => positions.has(item.id))
    .map((declaration) => {
      const position = positions.get(declaration.id)!;
      return {
        id: declaration.id,
        type: "declaration",
        position: {
          x: position.x - (declaration.id === root ? 82 : 90),
          y: position.y - (declaration.id === root ? 82 : 38),
        },
        focusable: false,
        data: {
          declaration,
          radii,
          root: declaration.id === root,
          active: selectedId === declaration.id,
          select: () => onSelect(declaration),
          center: () => center(declaration.id),
        },
      };
    });
  const index = new Map(workspace.declarations.map((item) => [item.id, item]));
  const edges = workspace.edges
    .filter(
      (edge) =>
        positions.has(edge.source) &&
        positions.has(edge.target) &&
        !(
          scope === "meaning" &&
          index.get(edge.source)?.kind === "theorem" &&
          edge.site === "value"
        ),
    )
    .map((edge, i) => ({
      id: edge.source + ":" + edge.target + ":" + i,
      source: edge.source,
      target: edge.target,
      type: "dependency",
      data: {
        site: edge.site,
        active: edge.source === selectedId || edge.target === selectedId,
      },
    }));
  if (!workspace.declarations.length)
    return (
      <div className="panel-empty">
        宣言がありません。Lean
        AtlasまたはワークスペースのJSONを読み込んでください。
      </div>
    );
  return (
    <section className="graph-page" aria-label="証明の依存関係">
      <div className="proof-graph-toolbar">
        <Autocomplete
          size="small"
          options={workspace.declarations}
          getOptionLabel={(item) => item.label || item.name}
          value={index.get(root) || null}
          isOptionEqualToValue={(a, b) => a.id === b.id}
          onChange={(_, item) => {
            if (item) {
              center(item.id);
              onSelect(item);
            }
          }}
          renderInput={(params) => <TextField {...params} label="中心の宣言" />}
        />
        <Select
          size="small"
          value={scope}
          onChange={(event) => setScope(event.target.value as GraphScope)}
          inputProps={{ "aria-label": "依存関係の範囲" }}
        >
          <MenuItem value="project">関連する宣言</MenuItem>
          <MenuItem value="dependencies">この宣言の依存</MenuItem>
          <MenuItem value="meaning">意味に関わる依存</MenuItem>
        </Select>
      </div>
      <div className="graph-canvas" ref={canvas}>
        <div className="canvas-toolbar">
          <Tooltip title="前の中心に戻る">
            <span>
              <IconButton
                aria-label="前の中心に戻る"
                disabled={!history.length}
                onClick={() => {
                  setRoot(history.at(-1)!);
                  setHistory(history.slice(0, -1));
                }}
              >
                <ArrowBackRounded fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
          <code>
            {positions.size} / {workspace.declarations.length} 宣言 ·{" "}
            {edges.length} 参照
          </code>
          <span className="graph-source-note">保存ソース</span>
        </div>
        <ReactFlowProvider>
          <ReactFlow
            key={root + scope}
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            fitView
            fitViewOptions={{
              padding: {
                top: "72px",
                bottom: "60px",
                left: "28px",
                right: "28px",
              },
            }}
            minZoom={0.08}
            maxZoom={2.5}
            nodesDraggable={false}
            nodesConnectable={false}
            elementsSelectable={false}
            zoomOnDoubleClick={false}
            forceColorMode="dark"
            aria-label="宣言の依存グラフ"
          >
            <FitToPane canvas={canvas} />
            <Controls showInteractive={false} />
          </ReactFlow>
        </ReactFlowProvider>
        <div className="graph-legend">
          <span>
            <i className="theorem" />
            定理
          </span>
          <span>
            <i className="definition" />
            定義
          </span>
          <span>実線：型 · 破線：本体</span>
        </div>
      </div>
    </section>
  );
}

function FitToPane({ canvas }: { canvas: RefObject<HTMLDivElement | null> }) {
  const { fitView } = useReactFlow();
  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    let frame = 0;
    const observer = new ResizeObserver((entries) => {
      const bounds = entries[0]?.contentRect;
      if (!bounds || bounds.width === 0 || bounds.height === 0) return;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(
        () =>
          void fitView({
            padding: {
              top: "68px",
              bottom: "56px",
              left: "26px",
              right: "26px",
            },
          }),
      );
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [canvas, fitView]);
  return null;
}

function DeclarationNode({ data }: NodeProps<RadarNode>) {
  const { declaration, root, active } = data;
  const { zoom } = useViewport();
  return (
    <div
      data-compact={zoom < 0.32}
      style={
        {
          "--graph-scale": Math.max(1, (root ? 0.62 : 0.43) / zoom),
          ...(root
            ? {
                "--ring-one": (data.radii[0] ?? 180) * 2 + "px",
                "--ring-two": (data.radii[1] ?? 360) * 2 + "px",
              }
            : {}),
        } as CSSProperties
      }
      className={
        "radar-node " +
        (root ? "root-node " : "") +
        (active ? "active " : "") +
        declaration.kind
      }
    >
      <Handle type="target" position={Position.Top} />
      <Handle type="source" position={Position.Bottom} />
      <button
        type="button"
        onClick={(event) => {
          event.stopPropagation();
          data.select();
        }}
        onDoubleClick={(event) => {
          event.stopPropagation();
          data.center();
        }}
        title={declaration.name}
      >
        <span className="node-dot" />
        {root && (
          <span className="node-kind">
            {declaration.kind === "theorem"
              ? "定理"
              : declaration.kind === "axiom"
                ? "公理"
                : "定義"}
          </span>
        )}
        <strong>{declaration.label || declaration.name}</strong>
        <code>{declaration.name}</code>
        <span className="visually-hidden">を選択</span>
      </button>
    </div>
  );
}
function DependencyEdge(props: EdgeProps) {
  const [path] = getBezierPath(props);
  const active = props.data?.active;
  return (
    <BaseEdge
      path={path}
      style={{
        stroke: active ? "#a4d3d5" : "#66818b",
        strokeWidth: active ? 1.6 : 1,
        strokeDasharray:
          props.data?.site === "value"
            ? "5 5"
            : props.data?.site === "unknown"
              ? "2 4"
              : undefined,
        opacity: active ? 1 : 0.65,
      }}
    />
  );
}
