import { lazy, Suspense } from "react";
import {
  Alert,
  Autocomplete,
  Button,
  CircularProgress,
  TextField,
  Typography,
} from "@mui/material";
import type {
  NativeCommand,
  NativeState,
  WorkspaceOverview,
} from "@meden/agent-protocol/native";
import { NativeMeaning } from "./NativeMeaning";
const NativeGraph = lazy(() =>
  import("./NativeGraph").then((module) => ({ default: module.NativeGraph })),
);

export function NativeDependencyGraph({
  state,
  overview,
  paths,
  onPaths,
  onCommand,
  running,
  elapsed,
  connectionError,
}: {
  state: NativeState;
  overview?: WorkspaceOverview;
  paths: string;
  onPaths: (paths: string) => void;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  running: boolean;
  elapsed: number;
  connectionError: string;
}) {
  const selectedPaths = paths
    .split("\n")
    .map((path) => path.trim())
    .filter(Boolean);
  const leanOnly =
    selectedPaths.length > 0 &&
    selectedPaths.every((path) => path.endsWith(".lean"));
  return (
    <section aria-label="依存グラフを表示する">
      <Typography variant="h3">ファイルの依存グラフ</Typography>
      <Typography variant="body2" sx={{ mb: 2 }}>
        ファイルを選んで表示します。Leanの依存はコンパイルして抽出し、Rustや準備前のLeanはソースの参照候補を確認できます。モデルへの接続は不要です。
      </Typography>
      <Autocomplete
        sx={{ mt: 2 }}
        size="small"
        disabled={running}
        options={overview?.files.map((file) => file.path) || []}
        value={paths.includes("\n") ? null : paths || null}
        onChange={(_, file) => {
          onPaths(file || "");
        }}
        renderInput={(params) => (
          <TextField {...params} label="依存を調べるファイルを探す" />
        )}
      />
      <details>
        <summary>複数ファイル・Lakeターゲットの指定</summary>
        <TextField
          label="グラフの対象ファイル（1行に1つ）"
          sx={{ mt: 2 }}
          multiline
          minRows={2}
          fullWidth
          size="small"
          value={paths}
          onChange={(event) => onPaths(event.target.value)}
          placeholder={"Spec.lean\nDomain.lean"}
          helperText="1行に1ファイル。Lakeターゲットは下のボタンで自動選択できます（依存を含め最大32ファイル）。大きなプロジェクトは対象ファイルを絞ってください。"
        />
        <Button
          disabled={running || !state.atlas.available}
          onClick={() => {
            onPaths("");
            void onCommand({ type: "atlas_snapshot", paths: [] });
          }}
        >
          Lakeターゲットを自動選択して表示
        </Button>
      </details>
      {state.running && (
        <Typography>
          {state.activity} · {elapsed}秒
        </Typography>
      )}
      <Typography variant="body2">
        Lean Atlas:{" "}
        {state.atlas.prepared
          ? "オフライン抽出の準備済み"
          : state.atlas.available
            ? "Leanを自動検出しました。初回の準備は依存抽出時に自動で行います"
            : "Lean実行環境が見つかりません。下の案内を確認してください。"}
      </Typography>
      {state.atlas.available && (
        <Typography variant="caption" component="p">
          プロジェクトの依存パッケージは表示時に確認します。
        </Typography>
      )}
      {!state.atlas.available && (
        <>
          <Alert severity="info" sx={{ whiteSpace: "pre-wrap", mt: 1 }}>
            {state.atlas.detail}
          </Alert>
          <Button
            disabled={running}
            onClick={() => void onCommand({ type: "refresh_status" })}
          >
            再検出
          </Button>
        </>
      )}
      <div
        style={{ display: "flex", flexWrap: "wrap", gap: 8, marginBlock: 16 }}
      >
        <Button
          variant="contained"
          disabled={running || !state.atlas.available || !leanOnly}
          onClick={() =>
            void onCommand({
              type: "atlas_snapshot",
              paths: selectedPaths,
            })
          }
        >
          依存グラフを表示
        </Button>
        <Button
          variant={
            selectedPaths.length > 0 && !leanOnly ? "contained" : "outlined"
          }
          disabled={running || !paths.trim()}
          onClick={() =>
            void onCommand({
              type: "source_snapshot",
              paths: selectedPaths,
            })
          }
        >
          ソースの参照候補を表示
        </Button>
      </div>
      {state.stopped && (
        <Alert
          severity={state.activity === "Complete" ? "info" : "warning"}
          sx={{ whiteSpace: "pre-wrap", my: 2 }}
        >
          {state.stopped}
          <br />
          対象を選び直すか、準備後に同じ操作を再実行してください。コンパイルできない場合はソースの参照候補も確認できます。
        </Alert>
      )}
      <details style={{ marginTop: 16 }}>
        <summary>初回の依存パッケージ準備</summary>
        <Typography variant="body2" sx={{ my: 1 }}>
          cloneには通常、.lake/packagesの依存パッケージは含まれません。
          Mathlibを使うプロジェクトでは、対象のプロジェクトディレクトリで
          <code>lake exe cache get</code>
          を一度実行してください。準備後に再度抽出できます。
          Medenの解析中にはダウンロードしません。
        </Typography>
        <Typography variant="body2" sx={{ my: 1 }}>
          依存の準備前でも、全体グラフや「ソースの参照候補を表示」で
          ソース内の宣言・参照を確認できます。
          コンパイル後の依存や新しい証明結果を示すものではありません。
        </Typography>
      </details>
      <details>
        <summary>抽出ツールの修復</summary>
        <Button
          disabled={running || !state.atlas.available}
          onClick={() => void onCommand({ type: "prepare_atlas" })}
        >
          依存抽出の準備
        </Button>
      </details>
      {state.source_snapshot && (
        <Typography variant="body2" sx={{ mt: 2, overflowWrap: "anywhere" }}>
          表示中のファイル:{" "}
          {Object.keys(state.source_snapshot.files).join("、")}
        </Typography>
      )}
      {state.source_snapshot ? (
        <Suspense
          fallback={
            <CircularProgress aria-label="ソースグラフを読み込み中" size={24} />
          }
        >
          <NativeGraph
            key={
              state.source_snapshot.atlas?.revision ||
              state.source_snapshot.revision
            }
            snapshot={state.source_snapshot}
            analysis={state.analysis}
            history={state.analysis_history}
            onRepair={(finding_id) =>
              onCommand({ type: "prepare_repair", finding_id })
            }
            onSelectAnalysis={(id) =>
              onCommand({ type: "select_analysis", id })
            }
            authorized={state.authorized && !connectionError}
            running={running}
            onAnalyze={(input) => onCommand({ type: "analyze_meaning", input })}
          />
        </Suspense>
      ) : (
        <p>
          上の検索でファイルを選び、「依存グラフを表示」を押してください。Leanの初回準備は自動で行います。
        </p>
      )}
      {!state.source_snapshot && state.analysis && (
        <NativeMeaning
          record={state.analysis}
          onRepair={(finding_id) =>
            onCommand({ type: "prepare_repair", finding_id })
          }
          history={state.analysis_history}
          onSelect={(id) => onCommand({ type: "select_analysis", id })}
          authorized={state.authorized}
          running={running}
          onRun={(input) => onCommand({ type: "analyze_meaning", input })}
        />
      )}
    </section>
  );
}
