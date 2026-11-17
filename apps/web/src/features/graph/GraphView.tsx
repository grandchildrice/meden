import { useEffect, useMemo, useState, type CSSProperties } from "react";
import {
  Autocomplete,
  Button,
  Chip,
  Divider,
  IconButton,
  MenuItem,
  Select,
  Snackbar,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import CenterFocusStrongOutlined from "@mui/icons-material/CenterFocusStrongOutlined";
import ContentCopyOutlined from "@mui/icons-material/ContentCopyOutlined";
import AccountTreeOutlined from "@mui/icons-material/AccountTreeOutlined";
import {
  ReactFlow,
  ReactFlowProvider,
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
import {
  type Workspace,
  type Declaration,
  analysisFor,
} from "@meden/workspace-schema";
import { EmptyState } from "../../components/EmptyState";
import { ProofStatusChip } from "../../components/Status";

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
  initialId,
  onClaim,
}: {
  workspace: Workspace;
  initialId?: string;
  onClaim: (id: string) => void;
}) {
  const [root, setRoot] = useState(
    initialId || workspace.declarations[0]?.id || "",
  );
  const [selected, setSelected] = useState(root);
  const [history, setHistory] = useState<string[]>([]);
  const [scope, setScope] = useState<GraphScope>("dependencies");
  const [notice, setNotice] = useState("");
  const [inspectorOpen, setInspectorOpen] = useState(true);
  useEffect(() => {
    if (
      initialId &&
      workspace.declarations.some((item) => item.id === initialId)
    ) {
      setRoot(initialId);
      setSelected(initialId);
    }
  }, [initialId, workspace.declarations]);
  const center = (id: string) => {
    if (id !== root) {
      setHistory((previous) => [...previous, root]);
      setRoot(id);
    }
    setSelected(id);
    setInspectorOpen(true);
  };
  const positions = useMemo(
    () => radialLayout(workspace, root, scope),
    [workspace, root, scope],
  );
  const radii = [
    ...new Set(
      [...positions.values()]
        .filter((position) => position.depth > 0)
        .map((position) => Math.round(Math.hypot(position.x, position.y))),
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
          active: selected === declaration.id,
          select: () => {
            setSelected(declaration.id);
            setInspectorOpen(true);
          },
          center: () => center(declaration.id),
        },
      };
    });
  const declarationIndex = new Map(
    workspace.declarations.map((item) => [item.id, item]),
  );
  const edges = workspace.edges
    .filter(
      (edge) =>
        positions.has(edge.source) &&
        positions.has(edge.target) &&
        !(
          scope === "meaning" &&
          declarationIndex.get(edge.source)?.kind === "theorem" &&
          edge.site === "value"
        ),
    )
    .map((edge, index) => ({
      id: edge.source + ":" + edge.target + ":" + index,
      source: edge.source,
      target: edge.target,
      type: "dependency",
      data: {
        site: edge.site,
        active: edge.source === selected || edge.target === selected,
      },
    }));
  const declaration = workspace.declarations.find(
    (item) => item.id === selected,
  );
  const analysis = analysisFor(workspace, { declarationId: selected });
  const rootDeclaration = workspace.declarations.find(
    (item) => item.id === root,
  );
  const direct = workspace.edges.filter((edge) => edge.source === selected);
  if (!workspace.declarations.length)
    return (
      <section className="graph-page">
        <header className="page-heading">
          <Typography variant="h1">依存関係</Typography>
        </header>
        <EmptyState
          icon={<AccountTreeOutlined />}
          title="宣言がありません"
          detail="Lean AtlasのグラフJSONまたはMedenワークスペースを読み込んでください。"
        />
      </section>
    );
  return (
    <section className="graph-page">
      <header className="page-heading graph-heading">
        <div>
          <Typography variant="h1">依存関係</Typography>
          <Typography variant="body2" color="text.secondary">
            {positions.size} / {workspace.declarations.length} 宣言 ·{" "}
            {edges.length} 依存
          </Typography>
        </div>
        <div className="graph-toolbar">
          <Autocomplete
            size="small"
            options={workspace.declarations}
            getOptionLabel={(option) => option.label || option.name}
            value={rootDeclaration || null}
            onChange={(_, value) => {
              if (value) center(value.id);
            }}
            isOptionEqualToValue={(option, value) => option.id === value.id}
            renderInput={(params) => (
              <TextField {...params} label="中心の宣言" />
            )}
            className="declaration-search"
          />
          <Select
            size="small"
            value={scope}
            onChange={(event) => setScope(event.target.value as GraphScope)}
            inputProps={{ "aria-label": "依存関係の範囲" }}
          >
            <MenuItem value="dependencies">すべての依存</MenuItem>
            <MenuItem value="meaning">意味に関わる依存</MenuItem>
          </Select>
        </div>
      </header>
      <div
        className={
          "graph-workspace " + (!inspectorOpen ? "inspector-closed" : "")
        }
      >
        <div className="graph-canvas">
          <div className="canvas-toolbar">
            <Tooltip title="前の中心に戻る">
              <span>
                <IconButton
                  aria-label="前の中心に戻る"
                  disabled={!history.length}
                  onClick={() => {
                    const previous = history[history.length - 1];
                    setHistory(history.slice(0, -1));
                    setRoot(previous);
                    setSelected(previous);
                  }}
                >
                  <ArrowBackRounded fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>
            <code>{rootDeclaration?.name}</code>
            <Button
              size="small"
              color="inherit"
              className="inspector-toggle"
              onClick={() => setInspectorOpen((value) => !value)}
            >
              {inspectorOpen ? "詳細を閉じる" : "詳細を開く"}
            </Button>
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
                  top: "80px",
                  bottom: "72px",
                  left: "48px",
                  right: "48px",
                },
              }}
              minZoom={0.2}
              maxZoom={2}
              nodesDraggable={false}
              nodesConnectable={false}
              elementsSelectable={false}
              zoomOnDoubleClick={false}
              forceColorMode="dark"
              aria-label="宣言の依存グラフ"
            >
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
            <span className="edge-legend">実線：型の依存</span>
            <span className="edge-legend">破線：値の依存</span>
            {edges.some((edge) => edge.data.site === "unknown") && (
              <span className="edge-legend">点線：未分類</span>
            )}
          </div>
        </div>
        {inspectorOpen && declaration && (
          <aside className="declaration-inspector" aria-label="宣言の詳細">
            <div className="panel-title">
              <Typography variant="h3">宣言の詳細</Typography>
              <Chip
                size="small"
                variant="outlined"
                label={
                  declaration.kind === "theorem"
                    ? "定理"
                    : declaration.kind === "axiom"
                      ? "公理"
                      : "定義"
                }
              />
            </div>
            <Typography variant="h2">
              {declaration.label || declaration.name}
            </Typography>
            <code className="declaration-name">{declaration.name}</code>
            {declaration.summary && (
              <Typography variant="body2" color="text.secondary">
                {declaration.summary}
              </Typography>
            )}
            {declaration.kind === "theorem" && (
              <ProofStatusChip status={declaration.proofStatus} />
            )}
            <div className="declaration-source">
              <Typography variant="caption" color="text.secondary">
                {declaration.source?.file}
                {declaration.source?.line ? ":" + declaration.source.line : ""}
              </Typography>
            </div>
            <div className="code-heading">
              <Typography variant="caption">宣言</Typography>
              <Tooltip title="宣言をコピー">
                <IconButton
                  size="small"
                  aria-label="宣言をコピー"
                  onClick={() => {
                    navigator.clipboard
                      .writeText(declaration.statement || declaration.name)
                      .then(() => setNotice("コピーしました"))
                      .catch(() => setNotice("コピーできませんでした"));
                  }}
                >
                  <ContentCopyOutlined fontSize="small" />
                </IconButton>
              </Tooltip>
            </div>
            <pre className="declaration-code">
              {declaration.statement || "宣言の本文は含まれていません。"}
            </pre>
            <Button
              variant="outlined"
              fullWidth
              startIcon={<CenterFocusStrongOutlined />}
              disabled={root === selected}
              onClick={() => center(selected)}
            >
              中心にする
            </Button>
            {analysis && (
              <Button fullWidth onClick={() => onClaim(analysis.claimId)}>
                要件との比較を見る
              </Button>
            )}
            <Divider />
            <Typography variant="h3">
              直接の依存 <span className="muted">{direct.length}</span>
            </Typography>
            <div className="dependency-list">
              {direct.map((edge, index) => {
                const target = workspace.declarations.find(
                  (item) => item.id === edge.target,
                );
                return (
                  <button
                    key={edge.target + index}
                    onClick={() => {
                      setSelected(edge.target);
                      if (!positions.has(edge.target)) center(edge.target);
                    }}
                  >
                    <span>{target?.label || edge.target}</span>
                    <small>
                      {edge.site === "type"
                        ? "型"
                        : edge.site === "value"
                          ? "値"
                          : "未分類"}
                    </small>
                  </button>
                );
              })}
              {!direct.length && (
                <Typography variant="body2" color="text.secondary">
                  依存はありません。
                </Typography>
              )}
            </div>
            {declaration.axioms.length > 0 && (
              <>
                <Typography variant="h3">使用公理</Typography>
                <pre className="axiom-list">
                  {declaration.axioms.join("\n")}
                </pre>
              </>
            )}
          </aside>
        )}
      </div>
      <Snackbar
        open={Boolean(notice)}
        message={notice}
        autoHideDuration={3000}
        onClose={() => setNotice("")}
      />
    </section>
  );
}

function DeclarationNode({ data }: NodeProps<RadarNode>) {
  const { declaration, root, active } = data;
  return (
    <div
      style={
        root
          ? ({
              "--ring-one": (data.radii[0] ?? 180) * 2 + "px",
              "--ring-two": (data.radii[1] ?? 360) * 2 + "px",
            } as CSSProperties)
          : undefined
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
        aria-label={[
          root
            ? declaration.kind === "theorem"
              ? "定理"
              : declaration.kind === "axiom"
                ? "公理"
                : "定義"
            : "",
          declaration.label,
          declaration.name,
          "を選択",
        ]
          .filter(Boolean)
          .join(" ")}
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
