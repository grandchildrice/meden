import {
  createContext,
  useContext,
  useEffect,
  useEffectEvent,
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
import {
  radialLayout,
  coneLayout,
  type GraphScope,
} from "@meden/atlas-adapter";
import type { Workspace, Declaration } from "@meden/workspace-schema";

type RadarData = Record<string, unknown> & {
  declaration: Declaration;
  root: boolean;
  radii: number[];
};

const GraphInteraction = createContext<{
  emphasized?: string;
  nearby: Set<string | undefined>;
  onHover: (id: string, active: boolean) => void;
  onSelect: (declaration: Declaration) => void;
  onCenter: (id: string) => void;
} | null>(null);

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
  const [scope, setScope] = useState<GraphScope | "related">("related");
  const [hovered, setHovered] = useState<string>();
  const emphasized = hovered || selectedId;
  const nearby = new Set([emphasized, root]);
  for (const edge of workspace.edges)
    if (edge.source === emphasized || edge.target === emphasized) {
      nearby.add(edge.source);
      nearby.add(edge.target);
    }
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
    () =>
      scope === "related"
        ? coneLayout(workspace, root)
        : radialLayout(workspace, root, scope),
    [workspace, root, scope],
  );
  // Keep the measured React Flow nodes stable while hover and selection change.
  const nodes = useMemo<RadarNode[]>(() => {
    const radii = [
      ...new Set(
        [...positions.values()]
          .filter((p) => p.depth > 0)
          .map((p) => Math.round(Math.hypot(p.x, p.y))),
      ),
    ].sort((a, b) => a - b);
    return workspace.declarations
      .filter((item) => positions.has(item.id))
      .map((declaration) => {
        const position = positions.get(declaration.id)!;
        const isRoot = declaration.id === root;
        return {
          id: declaration.id,
          type: "declaration",
          position: {
            x: position.x - (isRoot ? 82 : 90),
            y: position.y - (isRoot ? 82 : 25),
          },
          focusable: false,
          data: { declaration, radii, root: isRoot },
        };
      });
  }, [workspace.declarations, positions, root]);
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
        active: edge.source === emphasized || edge.target === emphasized,
        muted:
          Boolean(emphasized) &&
          edge.source !== emphasized &&
          edge.target !== emphasized,
        side:
          "side" in positions.get(edge.source)!
            ? (
                positions.get(edge.source)! as {
                  x: number;
                  y: number;
                  depth: number;
                  side?: string;
                }
              ).side
            : undefined,
        start: positions.get(edge.source),
        end: positions.get(edge.target),
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
          onChange={(event) =>
            setScope(event.target.value as GraphScope | "related")
          }
          inputProps={{ "aria-label": "依存関係の範囲" }}
        >
          <MenuItem value="related">依存と参照</MenuItem>
          <MenuItem value="project">プロジェクト全体</MenuItem>
          <MenuItem value="dependencies">この宣言の依存</MenuItem>
          <MenuItem value="meaning">意味に関わる依存</MenuItem>
        </Select>
      </div>
      <div
        className={"graph-canvas " + (scope === "related" ? "cone-graph" : "")}
        ref={canvas}
      >
        {scope === "related" && (
          <>
            <div className="cone-label cone-label-up">この宣言が使うもの</div>
            <div className="cone-label cone-label-down">この宣言を使うもの</div>
          </>
        )}
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
        </div>
        <GraphInteraction.Provider
          value={{
            emphasized,
            nearby,
            onHover: (id, active) =>
              setHovered((previous) =>
                active ? id : previous === id ? undefined : previous,
              ),
            onSelect,
            onCenter: center,
          }}
        >
          <ReactFlowProvider>
            <ReactFlow
              proOptions={{ hideAttribution: true }}
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
        </GraphInteraction.Provider>
        <details className="graph-information">
          <summary>グラフの詳細</summary>
          <div className="graph-legend">
            <span>
              <i className="theorem" />
              定理
            </span>
            <span>
              <i className="definition" />
              定義
            </span>
            {workspace.edges.some((edge) => edge.site !== "unknown") && (
              <span>実線：型 · 破線：本体</span>
            )}
            {workspace.edges.some((edge) => edge.site === "unknown") && (
              <span>点線：種類未確認の参照候補</span>
            )}
          </div>
        </details>
      </div>
    </section>
  );
}

function FitToPane({ canvas }: { canvas: RefObject<HTMLDivElement | null> }) {
  const { fitView } = useReactFlow();
  const fitToCanvas = useEffectEvent(() => {
    void fitView({
      padding: {
        top: "68px",
        bottom: "56px",
        left: "26px",
        right: "26px",
      },
    });
  });
  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    let frame = 0;
    const observer = new ResizeObserver((entries) => {
      const bounds = entries[0]?.contentRect;
      if (!bounds || bounds.width === 0 || bounds.height === 0) return;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => fitToCanvas());
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [canvas]);
  return null;
}

function DeclarationNode({ data }: NodeProps<RadarNode>) {
  const { declaration, root } = data;
  const { emphasized, nearby, onHover, onSelect, onCenter } =
    useContext(GraphInteraction)!;
  const active = emphasized === declaration.id;
  const { zoom } = useViewport();
  return (
    <div
      data-compact={zoom < 0.24}
      data-muted={Boolean(emphasized) && !nearby.has(declaration.id)}
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
        className="nopan"
        onMouseEnter={() => onHover(declaration.id, true)}
        onMouseLeave={() => onHover(declaration.id, false)}
        onFocus={() => onHover(declaration.id, true)}
        onBlur={() => onHover(declaration.id, false)}
        onClick={(event) => {
          event.stopPropagation();
          onSelect(declaration);
        }}
        onDoubleClick={(event) => {
          event.stopPropagation();
          onCenter(declaration.id);
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
  const source = props.data?.start as { x: number; y: number } | undefined;
  const target = props.data?.end as { x: number; y: number } | undefined;
  const fallback = getBezierPath(props)[0];
  let path = fallback;
  if (source && target) {
    const sr = Math.hypot(source.x, source.y),
      tr = Math.hypot(target.x, target.y),
      middle = (sr + tr) / 2;
    const sa = sr
      ? Math.atan2(source.y, source.x)
      : Math.atan2(target.y, target.x);
    const ta = tr ? Math.atan2(target.y, target.x) : sa;
    path = `M ${source.x},${source.y} C ${middle * Math.cos(sa)},${middle * Math.sin(sa)} ${middle * Math.cos(ta)},${middle * Math.sin(ta)} ${target.x},${target.y}`;
  }
  const active = props.data?.active;
  return (
    <BaseEdge
      path={path}
      style={{
        stroke:
          props.data?.side === "ref"
            ? props.data?.site === "type"
              ? "#56cdb6"
              : "#d6ad76"
            : props.data?.site === "type"
              ? "#9dacf1"
              : "#bca7e7",
        strokeWidth: active ? 1.6 : 1,
        strokeDasharray:
          props.data?.site === "value"
            ? "5 5"
            : props.data?.site === "unknown"
              ? "2 4"
              : undefined,
        opacity: props.data?.muted?.valueOf() ? 0.1 : active ? 1 : 0.65,
      }}
    />
  );
}
