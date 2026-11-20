import {
  Alert,
  Autocomplete,
  Button,
  TextField,
  Typography,
} from "@mui/material";
import type {
  NativeCommand,
  NativeState,
  WorkspaceOverview,
} from "@meden/agent-protocol/native";

/** Optional explicit compiler controls. Ordinary file navigation uses the cached graph. */
export function NativeDependencyGraph({
  state,
  overview,
  paths,
  onPaths,
  onCommand,
  running,
}: {
  state: NativeState;
  overview?: WorkspaceOverview;
  paths: string;
  onPaths: (paths: string) => void;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  running: boolean;
}) {
  const selected = paths
    .split("\n")
    .map((x) => x.trim())
    .filter(Boolean);
  return (
    <section className="file-analysis-tools" aria-label="Lean解析ツール">
      <Autocomplete
        size="small"
        options={overview?.files.map((f) => f.path) || []}
        value={paths.includes("\n") ? null : paths || null}
        onChange={(_, file) => onPaths(file || "")}
        renderInput={(params) => <TextField {...params} label="ファイル" />}
      />
      <div className="file-tool-actions">
        <Button
          variant="contained"
          disabled={
            running ||
            !state.atlas.available ||
            !selected.length ||
            !selected.every((p) => p.endsWith(".lean"))
          }
          onClick={() =>
            void onCommand({ type: "atlas_snapshot", paths: selected })
          }
        >
          Leanで解析
        </Button>
        <Button
          disabled={running || !selected.length}
          onClick={() =>
            void onCommand({ type: "source_snapshot", paths: selected })
          }
        >
          参照を更新
        </Button>
      </div>
      {!state.atlas.available && (
        <Alert severity="info">
          {state.atlas.detail}
          <Button
            disabled={running}
            onClick={() => void onCommand({ type: "refresh_status" })}
          >
            再検出
          </Button>
        </Alert>
      )}
      <details>
        <summary>複数ファイル・環境設定</summary>
        <TextField
          label="グラフの対象ファイル（1行に1つ）"
          fullWidth
          multiline
          minRows={2}
          value={paths}
          onChange={(e) => onPaths(e.target.value)}
          sx={{ mt: 2 }}
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
        <Typography variant="body2">
          Lean Atlas:{" "}
          {state.atlas.prepared
            ? "オフライン抽出の準備済み"
            : "初回の解析時に準備します"}
        </Typography>
        <Button
          disabled={running || !state.atlas.available}
          onClick={() => void onCommand({ type: "prepare_atlas" })}
        >
          依存抽出の準備
        </Button>
        <p>
          Mathlibなどの依存はプロジェクト側で準備します。Medenは解析中にダウンロードしません。
        </p>
      </details>
    </section>
  );
}
