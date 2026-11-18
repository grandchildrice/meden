import { lazy, Suspense, useMemo, useState } from "react";
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
import type { SourceSnapshot } from "@meden/agent-protocol/native";
import { sourceIndex } from "@meden/atlas-adapter/source-index";
import { SourceView } from "../source/SourceView";
const GraphView = lazy(() =>
  import("../graph/GraphView").then((module) => ({
    default: module.GraphView,
  })),
);

export function NativeGraph({ snapshot }: { snapshot: SourceSnapshot }) {
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
          ...sourceIndex(snapshot),
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
  }, [snapshot.revision]);
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
      <Alert severity="info">
        選択したファイルの字句解析による参照候補です。Leanの精密な依存抽出・証明確認は実行していません。
      </Alert>
      <Typography variant="caption">
        {Object.keys(snapshot.files).length}ファイル ·{" "}
        {workspace.declarations.length}宣言 · {workspace.edges.length}参照候補
      </Typography>
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
          <Chip label="証明は未確認" size="small" />
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
      <Typography className="native-revision" variant="caption">
        読み取った版: {snapshot.revision}
      </Typography>
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
