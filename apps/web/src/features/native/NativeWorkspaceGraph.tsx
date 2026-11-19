import { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Autocomplete,
  Button,
  Chip,
  TextField,
  Typography,
} from "@mui/material";
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
  WorkItem,
  PortableWorkspace,
} from "@meden/agent-protocol/native";

type Data = Record<string, unknown> & {
  title: string;
  subtitle: string;
  kind: "area" | "file" | "requirement" | "proof";
  file?: string;
  task?: string;
};
type Item = Node<Data, "item">;
function GraphItem({ data }: NodeProps<Item>) {
  return (
    <div className={`workspace-graph-node ${data.kind}`}>
      <Handle type="target" position={Position.Left} />
      <strong>{data.title}</strong>
      <span>{data.subtitle}</span>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}
const nodeTypes = { item: GraphItem };
function area(path: string) {
  const parts = path.split("/");
  return parts.length > 2
    ? parts.slice(0, 2).join("/")
    : parts.length > 1
      ? parts[0]!
      : "ルート";
}
function checkLabel(task: WorkItem, workspace: PortableWorkspace) {
  const check = workspace.document?.events
    .filter((event) => event.task === task.id && event.check)
    .at(-1)?.check;
  if (!check) return "チェック記録なし";
  if (
    check.requirement !== task.requirement ||
    check.module !== task.module ||
    JSON.stringify(check.theorems) !== JSON.stringify(task.theorems)
  )
    return "要件・対象の変更後は未チェック";
  return check.success
    ? "保存チェック成功（履歴）"
    : "保存チェック失敗・中断（履歴）";
}

export function NativeWorkspaceGraph({
  snapshot,
  state,
  onRead,
  onDependencies,
  onRefresh,
}: {
  snapshot?: WorkspaceOverview;
  state: NativeState;
  onRead: (file: string) => void;
  onDependencies: (file: string) => void;
  onRefresh: () => void;
}) {
  const [selected, setSelected] = useState<Data>();
  const [expanded, setExpanded] = useState<string>();
  const [flow, setFlow] = useState<ReactFlowInstance<Item>>();
  const files = snapshot?.files || [];
  const tasks = state.portable.document?.tasks || [];
  const taskRevision = JSON.stringify(tasks);
  const checkRevision = JSON.stringify(
    tasks.map((task) => checkLabel(task, state.portable)),
  );
  useEffect(() => {
    if (flow && selected?.kind === "file")
      void flow.fitView({
        nodes: [{ id: `file:${selected.file}` }],
        padding: 0.5,
        minZoom: 0.4,
        maxZoom: 1,
      });
  }, [flow, selected?.file, expanded]);
  const layout = useMemo(() => {
    const nodes: Item[] = [];
    const edges: Edge[] = [];
    const groups = new Map<string, typeof files>();
    for (const file of files) {
      const key = area(file.path);
      groups.set(key, [...(groups.get(key) || []), file]);
    }
    const target = (path: string) =>
      area(path) === expanded ? `file:${path}` : `area:${area(path)}`;
    let row = 0;
    for (const [name, members] of groups) {
      if (name === expanded) {
        members.forEach((file) =>
          nodes.push({
            id: `file:${file.path}`,
            type: "item",
            position: { x: 390, y: row++ * 125 },
            data: {
              title: file.path.split("/").at(-1)!,
              subtitle: `${file.declarations}宣言 · ${file.theorems}定理候補`,
              kind: "file",
              file: file.path,
            },
          }),
        );
      } else
        nodes.push({
          id: `area:${name}`,
          type: "item",
          position: { x: 390, y: row++ * 125 },
          data: {
            title: name,
            subtitle: `${members.length}ファイル · ${members.reduce((n, file) => n + file.theorems, 0)}定理候補`,
            kind: "area",
            file: name,
          },
        });
    }
    const modules = new Map<string, string | null>();
    for (const file of files.filter((file) => file.path.endsWith(".lean"))) {
      const parts = file.path.replace(/\.lean$/, "").split("/");
      for (let index = 0; index < parts.length; index++) {
        const module = parts.slice(index).join(".");
        modules.set(module, modules.has(module) ? null : file.path);
      }
    }
    const seen = new Set<string>();
    const edge = (source: string, destination: string, label?: string) => {
      const id = `${source}->${destination}`;
      if (source !== destination && !seen.has(id)) {
        seen.add(id);
        edges.push({
          id,
          source,
          target: destination,
          label,
          type: "smoothstep",
        });
      }
    };
    for (const file of files)
      for (const imported of file.imports) {
        const destination = modules.get(imported);
        if (destination) edge(target(file.path), target(destination));
      }
    for (const [index, task] of tasks.entries()) {
      const id = `requirement:${task.id}`;
      nodes.push({
        id,
        type: "item",
        position: { x: 0, y: index * 175 },
        data: {
          title: task.title,
          subtitle: "保存された要件",
          kind: "requirement",
          task: task.id,
        },
      });
      const file = task.module.replaceAll(".", "/") + ".lean";
      if (task.theorems.length) {
        const proof = `proof:${task.id}`;
        nodes.push({
          id: proof,
          type: "item",
          position: { x: 195, y: index * 175 },
          data: {
            title: `${task.theorems.length}件の証明対象`,
            subtitle: checkLabel(task, state.portable),
            kind: "proof",
            task: task.id,
            file,
          },
        });
        edge(id, proof);
        if (files.some((source) => source.path === file))
          edge(proof, target(file));
      }
    }
    if (state.project.proposed || state.project.accepted) {
      nodes.push({
        id: "native-contract",
        type: "item",
        position: { x: 0, y: tasks.length * 175 },
        data: {
          title: "Medenの契約",
          subtitle: state.project.verified
            ? "現在の契約範囲で検証済み"
            : state.project.accepted
              ? "受理済み・現在の証拠なし"
              : "提案・未受理",
          kind: "requirement",
        },
      });
      if (files.some((file) => file.path === state.project.source_path))
        edge("native-contract", target(state.project.source_path));
    }
    return { nodes, edges };
  }, [
    snapshot?.revision,
    expanded,
    taskRevision,
    checkRevision,
    state.project.reference.contract,
    state.project.verified,
    state.project.accepted,
  ]);
  const task = tasks.find((task) => task.id === selected?.task);
  const check = state.portable.document?.events
    .filter((event) => event.task === task?.id && event.check)
    .at(-1)?.check;
  return (
    <section
      aria-label="ワークスペース全体の契約と証明"
      className="workspace-overview"
    >
      <Typography variant="h3">ワークスペース全体の契約・証明</Typography>
      <Typography variant="body2">
        {state.overview.state === "loading"
          ? `読み込み中：${state.overview.scanned} / ${state.overview.total}ファイル`
          : `${files.length}ファイル · ${tasks.length}要件 · ${tasks.reduce((sum, task) => sum + task.theorems.length, 0)}証明対象`}
      </Typography>
      <Button disabled={state.running} onClick={onRefresh}>
        ワークスペース全体を読み直す
      </Button>
      {state.overview.error && (
        <Alert severity="error">
          全体グラフを読み込めませんでした。{state.overview.error}{" "}
          ファイルを直して読み直してください。
        </Alert>
      )}
      {state.overview.state === "stale" && (
        <Alert severity="info">
          ファイルが更新されました。全体グラフを読み直しています。
        </Alert>
      )}
      {snapshot && (
        <>
          <Typography variant="caption" component="p">
            全Rust／Leanソースを領域別に表示しています。線はimport候補、要件から定理・ファイルへの対応です。保存されたチェック結果は履歴として表示します。
          </Typography>
          <Autocomplete
            options={files.map((file) => file.path)}
            value={null}
            onChange={(_, file) => {
              if (file) {
                setExpanded(area(file));
                setSelected({
                  title: file,
                  subtitle: "ソースファイル",
                  kind: "file",
                  file,
                });
              }
            }}
            renderInput={(params) => (
              <TextField {...params} label="全ファイルから探す" size="small" />
            )}
          />
          {expanded && (
            <Button
              onClick={() => {
                setExpanded(undefined);
                setSelected(undefined);
              }}
            >
              領域の全体表示に戻る
            </Button>
          )}
          <div className="workspace-overview-canvas">
            <ReactFlow
              key={`${snapshot.revision}:${expanded || "all"}`}
              nodes={layout.nodes}
              edges={layout.edges}
              nodeTypes={nodeTypes}
              onInit={setFlow}
              defaultViewport={{ x: 8, y: 24, zoom: 0.95 }}
              minZoom={0.05}
              maxZoom={2}
              onlyRenderVisibleElements
              onNodeClick={(_, node) => {
                setSelected(node.data);
                if (node.data.kind === "area") setExpanded(node.data.file);
              }}
              nodesDraggable={false}
              nodesConnectable={false}
              aria-label="全ファイルの契約・証明グラフ"
            >
              <Background />
              <Controls showInteractive={false} />
            </ReactFlow>
          </div>
          {!files.length && (
            <p>
              このworkspaceにはRust／Leanソースがありません。保存された要件はそのまま表示します。
            </p>
          )}
          {selected && (
            <article aria-label="全体グラフの選択内容">
              <Typography component="h3">{selected.title}</Typography>
              {selected.file && selected.kind !== "area" && (
                <>
                  <Button
                    variant="contained"
                    disabled={state.running}
                    onClick={() => onDependencies(selected.file!)}
                  >
                    このファイルの依存グラフを開く
                  </Button>
                  <Button
                    disabled={state.running}
                    onClick={() => onRead(selected.file!)}
                  >
                    このファイルを読む
                  </Button>
                </>
              )}
              {task && (
                <>
                  <Typography component="h4">元の要件</Typography>
                  <pre>{task.requirement}</pre>
                  <Typography component="h4">証明対象</Typography>
                  <pre>{task.theorems.join("\n")}</pre>
                  <Typography component="h4">次にすること</Typography>
                  <pre>{task.next}</pre>
                  <Chip label={checkLabel(task, state.portable)} />
                  {check && (
                    <details>
                      <summary>保存された定理の型と公理</summary>
                      <p>チェック時の要件：{check.requirement}</p>
                      <p>チェック時のモジュール：{check.module}</p>
                      <pre>{check.inspection}</pre>
                    </details>
                  )}
                </>
              )}
              {selected.title === "Medenの契約" && (
                <pre>{state.project.review}</pre>
              )}
            </article>
          )}
          <details>
            <summary>索引の対象と読み取った版</summary>
            <p>
              隠し項目、シンボリックリンク、特殊ファイル、
              {snapshot.excluded.join("、")}を除くRust／Leanソースです。
            </p>
            <code>{snapshot.revision}</code>
          </details>
        </>
      )}
    </section>
  );
}
