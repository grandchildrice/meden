import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { MathText } from "../source/MathText";
import {
  ReactFlow,
  ReactFlowProvider,
  Controls,
  ViewportPortal,
  Handle,
  Position,
  BaseEdge,
  getBezierPath,
  type Node,
  type NodeProps,
  type EdgeProps,
  type Viewport,
} from "@xyflow/react";
import {
  coneLayout,
  coneLayoutDefinition,
  reviewScope,
} from "@meden/atlas-adapter";
import type { Workspace, Declaration } from "@meden/workspace-schema";

type RadarData = Record<string, unknown> & {
  declaration: Declaration;
  root: boolean;
  width: number;
  review: string;
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
export type GraphCamera = {
  center: string;
  focus: string;
  viewport: Viewport;
};
export type GraphCameraStore = { current: GraphCamera | undefined };
const centerKey = (declaration: Declaration) =>
  JSON.stringify([declaration.source?.file || "", declaration.name]);
export function GraphView({
  workspace,
  focusId,
  selectedId,
  onSelect,
  onHover,
  camera,
  focusName,
}: {
  workspace: Pick<Workspace, "declarations" | "edges">;
  focusId?: string;
  selectedId?: string;
  onSelect: (declaration: Declaration) => void;
  onHover?: (declaration: Declaration | undefined) => void;
  camera?: GraphCameraStore;
  focusName?: string;
}) {
  const focused =
    workspace.declarations.find((d) => d.id === focusId) ||
    workspace.declarations.find((d) => d.name === focusName);
  const focus = focusName || focused?.name || focusId || "";
  const preferred =
    focused?.id ||
    workspace.declarations.find((item) => item.id === "KernelProof.actual_spec")
      ?.id ||
    workspace.declarations.find((item) => item.isMainTheorem)?.id ||
    workspace.declarations.find((item) => item.kind === "theorem")?.id ||
    workspace.declarations[0]?.id ||
    "";
  const [chosenRoot, setRoot] = useState(() => {
    const retained = camera?.current;
    return (
      (retained?.focus === focus &&
        workspace.declarations.find((d) => centerKey(d) === retained.center)
          ?.id) ||
      preferred
    );
  });
  const root = workspace.declarations.some((d) => d.id === chosenRoot)
    ? chosenRoot
    : preferred;
  const centered = workspace.declarations.find((d) => d.id === root);
  const center = centered ? centerKey(centered) : "";
  const initialViewport = useMemo(() => {
    const retained = camera?.current;
    return retained?.center === center && retained.focus === focus
      ? retained.viewport
      : undefined;
  }, [camera, center, focus]);
  const previousFocus = useRef(focus);
  const [hovered, setHovered] = useState<string>();
  const emphasized = hovered || selectedId;
  const nearby = new Set([emphasized, root]);
  for (const edge of workspace.edges)
    if (edge.source === emphasized || edge.target === emphasized) {
      nearby.add(edge.source);
      nearby.add(edge.target);
    }
  useEffect(() => {
    const changed = previousFocus.current !== focus;
    previousFocus.current = focus;
    setRoot((previous) =>
      changed || !workspace.declarations.some((d) => d.id === previous)
        ? preferred
        : previous,
    );
  }, [focus, preferred, workspace.declarations]);
  const positions = useMemo(
    () => coneLayout(workspace, root),
    [workspace, root],
  );
  const review = useMemo(() => {
    const main = workspace.declarations
      .filter((d) => d.isMainTheorem)
      .map((d) => d.id);
    const selected = workspace.declarations.find((d) => d.id === root);
    return reviewScope(
      workspace,
      main.length ? main : selected?.kind === "theorem" ? [root] : [],
    );
  }, [workspace, root]);
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
        const width = position.width;
        return {
          id: declaration.id,
          type: "declaration",
          position: {
            x: position.x - width / 2,
            y: position.y - 52,
          },
          focusable: false,
          data: {
            declaration,
            radii,
            root: isRoot,
            width,
            review: review.get(declaration.id) || "unknown",
          },
        };
      });
  }, [workspace.declarations, positions, root, review]);
  const edges = workspace.edges
    .filter((edge) => positions.has(edge.source) && positions.has(edge.target))
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
        No declarations found. Import a Lean Atlas graph or workspace JSON.
      </div>
    );
  return (
    <section className="graph-page" aria-label="Proof dependencies">
      <div
        data-layout="l4sb"
        data-layout-version={coneLayoutDefinition.version}
        className="graph-canvas cone-graph"
      >
        <div className="cone-label cone-label-up">Uses</div>
        <div className="cone-label cone-label-down">Used by</div>
        <div
          className="graph-review-legend"
          aria-label="Review scope"
          title="Meaning review scope excludes known theorem-body dependencies. Unknown edge kinds remain unknown. This is not proof or review completion."
        >
          <span>● Human review</span>
          <span>○ Proof only</span>
          <span>◌ Unknown</span>
        </div>
        <GraphInteraction.Provider
          value={{
            emphasized,
            nearby,
            onHover: (id, active) => {
              onHover?.(
                active
                  ? workspace.declarations.find((d) => d.id === id)
                  : undefined,
              );
              setHovered((previous) =>
                active ? id : previous === id ? undefined : previous,
              );
            },
            onSelect,
            onCenter: setRoot,
          }}
        >
          <ReactFlowProvider>
            <ReactFlow
              proOptions={{ hideAttribution: true }}
              key={root}
              nodes={nodes}
              edges={edges}
              nodeTypes={nodeTypes}
              edgeTypes={edgeTypes}
              defaultViewport={initialViewport}
              fitView={!initialViewport}
              onMove={(event, viewport) => {
                // Initial fitting is not a user's camera to restore over the
                // first compiled layout. Retain gestures and later adjustments.
                if (
                  camera &&
                  (event ||
                    (camera.current?.center === center &&
                      camera.current.focus === focus))
                )
                  camera.current = { center, focus, viewport: { ...viewport } };
              }}
              fitViewOptions={{
                padding: {
                  top: "60px",
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
              aria-label="Declaration dependency graph"
            >
              <ViewportPortal>
                <svg
                  className="l4sb-rings"
                  width="1"
                  height="1"
                  aria-hidden="true"
                >
                  {nodes[0]?.data.radii.map((r) => (
                    <circle key={r} cx="0" cy="0" r={r} />
                  ))}
                </svg>
              </ViewportPortal>
              <Controls showInteractive={false} />
            </ReactFlow>
          </ReactFlowProvider>
        </GraphInteraction.Provider>
      </div>
    </section>
  );
}

function DeclarationNode({ data }: NodeProps<RadarNode>) {
  const { declaration, root } = data;
  const { emphasized, nearby, onHover, onSelect, onCenter } =
    useContext(GraphInteraction)!;
  const active = emphasized === declaration.id;
  return (
    <div
      data-muted={Boolean(emphasized) && !nearby.has(declaration.id)}
      data-review={data.review}
      style={
        {
          "--node-width": data.width + "px",
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
        aria-label={
          (declaration.label || declaration.name) +
          " · " +
          declaration.name +
          " · " +
          (declaration.semantic?.sentence ||
            declaration.summary ||
            declaration.label) +
          ": select"
        }
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
        title={
          declaration.name +
          " · " +
          (data.review === "human"
            ? "Human meaning review target"
            : data.review === "proof-only"
              ? "Proof-only dependency; not a meaning-review target"
              : "Review scope unknown") +
          (declaration.proofStatus === "unproved" ? " · Unfinished proof" : "")
        }
      >
        <span className="node-dot" />
        <strong>
          <MathText
            text={
              declaration.semantic?.sentence ||
              declaration.summary ||
              declaration.label ||
              declaration.name
            }
          />
        </strong>
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
