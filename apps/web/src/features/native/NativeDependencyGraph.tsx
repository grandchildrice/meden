import {
  Alert,
  Autocomplete,
  Button,
  TextField,
  Tooltip,
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
    <section className="file-analysis-tools" aria-label="Lean tools">
      <Autocomplete
        size="small"
        options={overview?.files.map((f) => f.path) || []}
        value={paths.includes("\n") ? null : paths || null}
        onChange={(_, file) => onPaths(file || "")}
        renderInput={(params) => <TextField {...params} label="File" />}
      />
      <div className="file-tool-actions">
        <Tooltip title="Rebuild the selected modules and their saved observations. Maximum 32 files / 16 MiB.">
          <span>
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
              Extract compiled dependencies
            </Button>
          </span>
        </Tooltip>
        <Button
          disabled={running || !selected.length}
          onClick={() =>
            void onCommand({ type: "source_snapshot", paths: selected })
          }
        >
          Refresh references
        </Button>
      </div>
      {!state.atlas.available && (
        <Alert severity="info">
          {state.atlas.detail}
          <Button
            disabled={running}
            onClick={() => void onCommand({ type: "refresh_status" })}
          >
            Rediscover
          </Button>
        </Alert>
      )}
      <details>
        <summary>Multiple files and environment</summary>
        <TextField
          label="Graph source files (one per line)"
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
          Select Lake targets automatically
        </Button>
        <Typography variant="body2">
          Lean Atlas:{" "}
          {state.atlas.prepared
            ? "Offline extraction ready"
            : "Prepared on first extraction"}
        </Typography>
        <Button
          disabled={running || !state.atlas.available}
          onClick={() => void onCommand({ type: "prepare_atlas" })}
        >
          Prepare dependency extraction
        </Button>
        <p>
          Prepare dependencies such as Mathlib in the project. Meden does not
          download them during analysis.
        </p>
      </details>
    </section>
  );
}
