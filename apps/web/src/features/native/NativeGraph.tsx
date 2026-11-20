import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import {
  Alert,
  Button,
  Chip,
  CircularProgress,
  Dialog,
  DialogContent,
  DialogTitle,
  Typography,
} from "@mui/material";
import { workspaceSchema, type Declaration } from "@meden/workspace-schema";
import type {
  SourceSnapshot,
  MeaningRecord,
  MeaningInput,
} from "@meden/agent-protocol/native";
import { sourceIndex } from "@meden/atlas-adapter/source-index";
import { importAtlas } from "@meden/atlas-adapter";
import { SourceView } from "../source/SourceView";
import { NativeMeaning } from "./NativeMeaning";
const GraphView = lazy(() =>
  import("../graph/GraphView").then((module) => ({
    default: module.GraphView,
  })),
);

export function NativeGraph({
  snapshot,
  analysis,
  history,
  onSelectAnalysis,
  onRepair,
  authorized,
  running,
  onAnalyze,
}: {
  snapshot: SourceSnapshot;
  analysis: MeaningRecord | null;
  history: MeaningRecord[];
  onSelectAnalysis: (id: string) => Promise<boolean>;
  onRepair: (finding: string) => Promise<boolean>;
  authorized: boolean;
  running: boolean;
  onAnalyze: (input: MeaningInput) => Promise<boolean>;
}) {
  const [selected, setSelected] = useState<Declaration>();
  const [sourceOpen, setSourceOpen] = useState(false);
  const [notification, setNotification] = useState("");
  const result = useMemo(() => {
    try {
      return {
        workspace: workspaceSchema.parse({
          schemaVersion: 1,
          id: "native-sources",
          name: "選択したローカルソース",
          sourceRevision: snapshot.revision,
          inputRevision: 0,
          origin: "local",
          updatedAt: new Date(0).toISOString(),
          ...(snapshot.atlas
            ? importAtlas(snapshot.atlas.graph)
            : sourceIndex(snapshot)),
          requirements: [],
          analyses: [],
          evidence: [],
          conversations: [],
          reviewDrafts: [],
        }),
      };
    } catch (error) {
      return { error: String(error) };
    }
  }, [snapshot.revision, snapshot.atlas?.revision]);
  useEffect(() => {
    if (!selected && analysis && result.workspace) {
      const target = analysis.input.target;
      setSelected(
        result.workspace.declarations.find(
          (item) =>
            item.name === target.name &&
            item.source?.file === target.file &&
            item.source?.line === target.line,
        ),
      );
    }
  }, [analysis?.input.target.name, result.workspace]);
  if (!result.workspace)
    return (
      <Alert severity="error">
        ソースのグラフを表示できません。{result.error}
      </Alert>
    );
  const workspace = result.workspace;
  return (
    <section
      className="native-source-graph"
      aria-label="ローカルソースのグラフ"
    >
      <details className="native-graph-information">
        <summary>解析の情報</summary>
        <Alert severity="info">
          {snapshot.atlas
            ? `Lean Atlasによるコンパイル後の依存です（Lean ${snapshot.atlas.graph.metadata.leanVersion}）。プロジェクト内のimportを自動収集し、外部ライブラリは展開しない境界として表示します。Medenの契約・証明の受理状態は変更しません。`
            : "選択したファイルの字句解析による参照候補です。Leanの精密な依存抽出・証明確認は実行していません。"}
        </Alert>

        {snapshot.collection && (
          <Typography variant="body2">
            自動収集したプロジェクト内の依存です。対象:{" "}
            {snapshot.collection.roots.join(", ")}。
            {snapshot.collection.packages
              .map(
                (pkg) => `Lake: ${pkg.name} (${pkg.source_roots.join(", ")})`,
              )
              .join(" · ")}
            外部モジュール（宣言を展開しない境界）:{" "}
            {snapshot.collection.external_modules.join(", ") || "なし"}。
          </Typography>
        )}
        <Typography variant="caption">
          {Object.keys(snapshot.files).length}ファイル ·{" "}
          {workspace.declarations.length}宣言 · {workspace.edges.length}
          {snapshot.atlas ? "依存" : "参照候補"}
        </Typography>
      </details>
      {notification && <p role="status">{notification}</p>}
      <Suspense
        fallback={
          <CircularProgress aria-label="グラフを読み込み中" size={24} />
        }
      >
        {workspace.declarations.length ? (
          <GraphView
            workspace={workspace}
            selectedId={selected?.id}
            onSelect={setSelected}
          />
        ) : (
          <p>
            この索引で扱えるRust／Lean宣言がありません。対象ファイルを確認してください。
          </p>
        )}
      </Suspense>
      {selected && (
        <div className="native-declaration" aria-label="選択した宣言">
          <Typography component="h3">{selected.name}</Typography>
          <Chip
            label={
              selected.proofStatus === "unproved"
                ? "未完成の証明を検出"
                : "証明は未確認"
            }
            size="small"
          />
          <pre>{selected.statement}</pre>
          <Button onClick={() => setSourceOpen(true)}>
            {selected.source?.file}:{selected.source?.line} のソースを開く
          </Button>
          <Typography variant="caption" component="p">
            参照先:{" "}
            {workspace.edges
              .filter((edge) => edge.source === selected.id)
              .map(
                (edge) =>
                  workspace.declarations.find(
                    (declaration) => declaration.id === edge.target,
                  )?.name,
              )
              .join(", ") || "この索引内では見つかりません"}
          </Typography>
        </div>
      )}
      <NativeMeaning
        record={analysis}
        history={history}
        onSelect={onSelectAnalysis}
        onRepair={onRepair}
        sourceRevision={snapshot.revision}
        graphRevision={snapshot.atlas?.revision}
        authorized={authorized}
        running={running}
        onRun={onAnalyze}
        target={
          selected?.source?.line
            ? {
                file: selected.source.file,
                line: selected.source.line,
                name: selected.name,
              }
            : undefined
        }
      />

      <Dialog
        open={sourceOpen}
        onClose={() => setSourceOpen(false)}
        fullWidth
        maxWidth="lg"
        slotProps={{ paper: { className: "native-source-dialog" } }}
      >
        <DialogTitle>
          読み取ったソース{" "}
          <Button onClick={() => setSourceOpen(false)}>閉じる</Button>
        </DialogTitle>
        <DialogContent>
          <SourceView
            files={snapshot.files}
            target={selected}
            onBack={() => setSourceOpen(false)}
            onNotify={setNotification}
            showBack={false}
          />
        </DialogContent>
      </Dialog>
    </section>
  );
}
