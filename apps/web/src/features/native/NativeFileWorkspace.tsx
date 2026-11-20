import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import { Alert, Button, LinearProgress, Typography } from "@mui/material";
import type { Declaration, Workspace } from "@meden/workspace-schema";
import type { NativeState, NativeCommand } from "@meden/agent-protocol/native";
import type { MapScene, Symbol } from "./project-map-model";
import { NativeCode } from "./NativeExplorer";
import { NativeMeaning } from "./NativeMeaning";
import { NativeGraph } from "./NativeGraph";
const GraphView = lazy(() =>
  import("../graph/GraphView").then((m) => ({ default: m.GraphView })),
);

function references(
  scene: MapScene,
  path: string,
  revision: string,
): Workspace {
  const local = scene.byPath.get(path)?.symbols || [];
  const ids = new Set(local.map((s) => s.id));
  const edges = scene.edges.filter(
    (e) => ids.has(e.source) || ids.has(e.target),
  );
  const all = new Set([...ids, ...edges.flatMap((e) => [e.source, e.target])]);
  return {
    schemaVersion: 1,
    id: "file-references",
    name: path,
    sourceRevision: revision,
    inputRevision: 0,
    origin: "local",
    updatedAt: new Date(0).toISOString(),
    declarations: [...all].map((id) => {
      const s = scene.symbols.get(id)!;
      return {
        id,
        name: s.name,
        label: s.name.split(".").at(-1) || s.name,
        kind: s.kind,
        statement: s.name,
        source: { file: s.file, line: s.line },
        proofStatus: "unknown",
        evidenceRefs: [],
        axioms: [],
      };
    }),
    edges: edges.map((e) => ({ ...e, site: "unknown" })),
    requirements: [],
    analyses: [],
    evidence: [],
    conversations: [],
    reviewDrafts: [],
  };
}

export function NativeFileWorkspace({
  state,
  path,
  scene,
  revision,
  selected,
  running,
  analysisFile,
  onCommand,
  onSelect,
}: {
  state: NativeState;
  path: string;
  scene?: MapScene;
  revision: string;
  selected?: Symbol;
  running: boolean;
  analysisFile?: string;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  onSelect: (symbol: Symbol) => void;
}) {
  const graph = useMemo(
    () => (scene ? references(scene, path, revision) : undefined),
    [scene, path, revision],
  );
  const [failure, setFailure] = useState<string>();
  const analyzing =
    state.running && state.operation === "analysis" && analysisFile === path;
  useEffect(() => {
    if (analysisFile !== path || state.operation !== "analysis") return;
    if (state.running) setFailure(undefined);
    else if (state.stopped) setFailure(state.stopped);
  }, [state.running, state.operation, state.stopped, analysisFile, path]);
  const snapshot =
    state.source_snapshot?.files[path] !== undefined
      ? state.source_snapshot
      : undefined;
  const pick = (declaration: Declaration) => {
    const symbol = scene?.symbols.get(declaration.id);
    if (symbol) onSelect(symbol);
  };
  return (
    <section className="file-split" aria-label={"ファイル: " + path}>
      <div className="file-source-pane">
        <NativeCode
          state={state}
          path={path}
          line={selected?.file === path ? selected.line : undefined}
          running={running}
          onCommand={onCommand}
        />
      </div>
      <div
        className="file-graph-pane"
        role="region"
        aria-label="ファイルの参照グラフ"
      >
        {analyzing && (
          <div className="file-analysis-progress" role="status">
            <LinearProgress />
            <span>解析中…</span>
            <Button onClick={() => void onCommand({ type: "cancel" })}>
              中止
            </Button>
          </div>
        )}
        {failure && (
          <Alert severity="warning">
            <Typography>
              このファイルのLean解析を完了できませんでした。
            </Typography>
            {failure.includes("closure exceeds") && (
              <p>
                依存先が一度に解析できる範囲を超えています。ソースと参照グラフはそのまま閲覧できます。
              </p>
            )}
            <details>
              <summary>原因</summary>
              {failure}
            </details>
          </Alert>
        )}
        {snapshot ? (
          <NativeGraph
            key={snapshot.atlas?.revision || snapshot.revision}
            snapshot={snapshot}
            analysis={state.analysis}
            history={state.analysis_history}
            onRepair={(finding_id) =>
              onCommand({ type: "prepare_repair", finding_id })
            }
            onSelectAnalysis={(id) =>
              onCommand({ type: "select_analysis", id })
            }
            authorized={state.authorized}
            running={running}
            onAnalyze={(input) => onCommand({ type: "analyze_meaning", input })}
            onSource={(file, line, name) => {
              const found =
                scene &&
                [...scene.symbols.values()].find(
                  (s) => s.file === file && s.line === line,
                );
              if (found) onSelect(found);
              else if (file === path)
                onSelect({
                  id: name,
                  file,
                  name,
                  line,
                  kind: "definition",
                  private: false,
                  references: [],
                  x: 0,
                  y: 0,
                  cellWidth: 0,
                });
            }}
          />
        ) : graph?.declarations.length ? (
          <Suspense fallback={<LinearProgress />}>
            <GraphView
              workspace={graph}
              selectedId={selected?.id}
              focusId={selected?.file === path ? selected.id : undefined}
              onSelect={pick}
            />
          </Suspense>
        ) : (
          <p className="file-empty">
            このファイルには参照グラフの対象となる宣言がありません。
          </p>
        )}
        {!snapshot && state.analysis?.input.target.file === path && (
          <NativeMeaning
            record={state.analysis}
            history={state.analysis_history}
            onSelect={(id) => onCommand({ type: "select_analysis", id })}
            onRepair={(finding_id) =>
              onCommand({ type: "prepare_repair", finding_id })
            }
            authorized={state.authorized}
            running={running}
            onRun={(input) => onCommand({ type: "analyze_meaning", input })}
          />
        )}
      </div>
    </section>
  );
}
