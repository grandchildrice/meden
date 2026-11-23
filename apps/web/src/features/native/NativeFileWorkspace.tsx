import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import { Alert, Button, LinearProgress } from "@mui/material";
import { NativePreparation, PreparationLog } from "./NativePreparation";
import type { Declaration, Workspace } from "@meden/workspace-schema";
import type {
  NativeState,
  NativeCommand,
  SourceSnapshot,
  FileView,
} from "@meden/agent-protocol/native";
import type { MapScene, Symbol } from "./project-map-model";
import { NativeCode } from "./NativeExplorer";
import { NativeMeaning } from "./NativeMeaning";
import { declarationRole } from "./declaration-role";
import { NativeGraph, displayGraph } from "./NativeGraph";
import { reviewScope } from "@meden/atlas-adapter";
const GraphView = lazy(() =>
  import("../graph/GraphView").then((m) => ({ default: m.GraphView })),
);

function references(
  scene: MapScene,
  path: string,
): Pick<Workspace, "declarations" | "edges"> {
  const local = scene.byPath.get(path)?.symbols || [];
  const ids = new Set(local.map((s) => s.id));
  const edges = scene.edges.filter(
    (e) => ids.has(e.source) || ids.has(e.target),
  );
  const all = new Set([...ids, ...edges.flatMap((e) => [e.source, e.target])]);
  return {
    declarations: [...all].map((id) => {
      const s = scene.symbols.get(id)!;
      return {
        id,
        name: s.name,
        label: s.role.label + " · " + (s.name.split(".").at(-1) || s.name),
        kind: s.kind,
        isMainTheorem: s.role.id === "main_theorem",
        statement: s.name,
        source: { file: s.file, line: s.line },
        proofStatus: "unknown",
        evidenceRefs: [],
        axioms: [],
      };
    }),
    edges: edges.map((e) => ({ ...e, site: "unknown" })),
  };
}

export function NativeFileWorkspace({
  state,
  path,
  scene,
  revision,
  selected,
  focused,
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
  focused?: Symbol;
  running: boolean;
  analysisFile?: string;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  onSelect: (symbol: Symbol) => void;
}) {
  const graph = useMemo(
    () => (scene ? references(scene, path) : undefined),
    [scene, path, revision],
  );
  const [failure, setFailure] = useState<string>();
  const [hovered, setHovered] = useState<Declaration>();
  const analyzing =
    state.running && state.operation === "analysis" && analysisFile === path;
  useEffect(() => {
    if (analysisFile !== path || state.operation !== "analysis") return;
    if (state.running) setFailure(undefined);
    else if (state.stopped) setFailure(state.stopped);
  }, [state.running, state.operation, state.stopped, analysisFile, path]);
  const incoming =
    state.source_snapshot?.files[path] !== undefined
      ? state.source_snapshot
      : undefined;
  const preparation = state.source_preparation[path];
  const incomingFile = state.reader?.path === path ? state.reader : undefined;
  const [retainedFile, setRetainedFile] = useState<FileView>();
  useEffect(() => {
    if (incomingFile) setRetainedFile(incomingFile);
  }, [incomingFile]);
  const file = incomingFile || retainedFile;
  const unavailable =
    file &&
    (file.reading.state !== "ready" || file.reading.content.kind !== "text");
  const [retained, setRetained] = useState<SourceSnapshot>();
  const retainedRevision = state.source_views[path];
  useEffect(() => {
    if (incoming) setRetained(incoming);
    else if (!retainedRevision) setRetained(undefined);
  }, [incoming?.revision, incoming?.graph?.revision, retainedRevision]);
  const snapshot =
    incoming ||
    (retained?.revision === retainedRevision ? retained : undefined);
  const sourceOnly = file?.navigation === "source_only" && !snapshot;
  const displayed = snapshot ? { ...state, source_snapshot: snapshot } : state;
  const semanticGraph = useMemo(
    () => (snapshot?.graph ? displayGraph(snapshot) : graph),
    [snapshot?.graph?.revision, graph],
  );
  const explanations = useMemo(() => {
    if (!semanticGraph || !snapshot?.compiled) return [];
    const main = semanticGraph.declarations
      .filter((d) => d.isMainTheorem)
      .map((d) => d.id);
    const root =
      semanticGraph.declarations.find((d) => d.name === focused?.name) ||
      semanticGraph.declarations.find((d) => d.kind === "theorem");
    const scope = reviewScope(
      semanticGraph,
      main.length ? main : root ? [root.id] : [],
    );
    return semanticGraph.declarations.flatMap((d) =>
      d.semantic && d.source?.file === path && d.source.line
        ? [
            {
              name: d.name,
              line: d.source.line,
              semantic: d.semantic,
              proofStatus: d.proofStatus,
              review: scope.get(d.id) || "unknown",
            },
          ]
        : [],
    );
  }, [semanticGraph, focused?.name, path]);
  const pick = (declaration: Declaration) => {
    const symbol = scene?.symbols.get(declaration.id);
    if (symbol) onSelect(symbol);
  };
  const preparing =
    !unavailable && !sourceOnly && preparation?.state === "preparing";
  return (
    <section className="file-split" aria-label={"File: " + path}>
      <div className="file-source-pane">
        <NativeCode
          state={displayed}
          observation={file}
          path={path}
          line={selected?.file === path ? selected.line : undefined}
          running={running}
          onCommand={onCommand}
          explanations={preparing ? [] : explanations}
          hoveredLine={
            hovered?.source?.file === path ? hovered.source.line : undefined
          }
        />
      </div>
      <div
        className="file-graph-pane"
        role="region"
        aria-label="File reference graph"
      >
        {preparing ? (
          <NativePreparation
            preparation={preparation}
            path={path}
            onCancel={
              state.running && state.operation === "inspection"
                ? () => {
                    void onCommand({ type: "cancel" });
                  }
                : undefined
            }
          />
        ) : unavailable || sourceOnly ? (
          <p className="file-empty">
            No dependency analysis is available for this file.
          </p>
        ) : (
          <>
            {preparation?.state === "failed" && (
              <Alert severity="warning">
                <p>{preparation.phase} stopped.</p>
                <p>{preparation.error?.split("\n")[0]}</p>
                <details>
                  <summary>Details</summary>
                  <p>{preparation.error}</p>
                </details>
                <Button
                  onClick={() =>
                    void onCommand({
                      type: "inspect",
                      path,
                      directory: false,
                      offset: selected?.line || 1,
                    })
                  }
                >
                  Retry preparation
                </Button>
              </Alert>
            )}
            {preparation?.state === "failed" && (
              <PreparationLog preparation={preparation} />
            )}
            {preparation?.state === "ready" && preparation.log && (
              <details className="preparation-history">
                <summary>Build log</summary>
                <PreparationLog preparation={preparation} />
              </details>
            )}
            {failure && preparation?.state !== "failed" && (
              <Alert severity="warning">
                <p>Analysis stopped.</p>
                <details>
                  <summary>Details</summary>
                  <p>{failure}</p>
                </details>
              </Alert>
            )}
            {analyzing && (
              <div className="file-analysis-progress" role="status">
                <LinearProgress />
                <span>Analyzing…</span>
                <Button onClick={() => void onCommand({ type: "cancel" })}>
                  Cancel
                </Button>
              </div>
            )}
            {snapshot ? (
              <NativeGraph
                focusName={focused?.file === path ? focused.name : undefined}
                key={snapshot.graph?.revision || snapshot.revision}
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
                onHover={setHovered}
                onAnalyze={(input) =>
                  onCommand({ type: "analyze_meaning", input })
                }
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
                      declaration: "def",
                      role: declarationRole("def"),
                      private: false,
                      references: [],
                      semantic: null,
                      main_theorem: false,
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
                  focusId={focused?.file === path ? focused.id : undefined}
                  onSelect={pick}
                  onHover={setHovered}
                />
              </Suspense>
            ) : (
              <p className="file-empty">
                This file has no declarations supported by the reference graph.
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
          </>
        )}
      </div>
    </section>
  );
}
