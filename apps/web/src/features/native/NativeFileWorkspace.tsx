import {
  lazy,
  memo,
  Suspense,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { GraphCamera } from "../graph/GraphView";
import { Alert, Button, LinearProgress } from "@mui/material";
import { NativePreparation, PreparationLog } from "./NativePreparation";
import type { Declaration, Workspace } from "@meden/workspace-schema";
import type {
  NativeState,
  NativeCommand,
  SourceSnapshot,
  FileView,
  PreparationLogPublication,
  MeaningRecord,
  MeaningInput,
} from "@meden/agent-protocol/native";
import { mapArea, type MapScene, type Symbol } from "./project-map-model";
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
  const local = mapArea(scene, path)?.symbols || [];
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

function FileWorkspace({
  state,
  analysis,
  history,
  analysesLoading,
  analysesError,
  onRetryAnalyses,
  snapshot,
  sourceError,
  preparationLog,
  preparationLogError,
  onRetryLog,
  file,
  readerError,
  onRetryReader,
  onRetrySource,
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
  active: boolean;
  state: NativeState;
  analysis?: MeaningRecord;
  history?: MeaningRecord[];
  analysesLoading?: boolean;
  analysesError?: string;
  onRetryAnalyses?: () => void;
  snapshot?: SourceSnapshot;
  sourceError?: string;
  preparationLog?: PreparationLogPublication["data"];
  preparationLogError?: string;
  onRetryLog: () => void;
  file?: FileView;
  readerError?: string;
  onRetryReader: () => void;
  onRetrySource: () => void;
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
  const graphCamera = useRef<GraphCamera | undefined>(undefined);
  const analyzing =
    state.running && state.operation === "analysis" && analysisFile === path;
  useEffect(() => {
    if (analysisFile !== path || state.operation !== "analysis") return;
    if (state.running) setFailure(undefined);
    else if (state.stopped) setFailure(state.stopped);
  }, [state.running, state.operation, state.stopped, analysisFile, path]);
  const preparation = state.source_preparation[path];
  const logMatches =
    preparationLog?.job === preparation?.job &&
    preparationLog?.resource === path;
  const [lastLog, setLastLog] = useState<PreparationLogPublication["data"]>();
  useEffect(() => {
    if (logMatches && preparationLog) setLastLog(preparationLog);
    else if (lastLog?.job !== preparation?.job) setLastLog(undefined);
  }, [logMatches, preparationLog, preparation?.job, lastLog?.job]);
  const logProps = {
    log: logMatches
      ? preparationLog
      : lastLog?.job === preparation?.job
        ? lastLog
        : undefined,
    error:
      preparationLogError ||
      (preparationLog && !logMatches
        ? "The build log does not match this preparation."
        : undefined),
    onRetry: onRetryLog,
  };

  const unavailable =
    file &&
    (file.reading.state !== "ready" || file.reading.content.kind !== "text");
  const sourceOnly = file?.navigation === "source_only" && !snapshot;
  const loading = Boolean(
    state.sources.files[path] && !snapshot && !sourceError,
  );
  const semanticGraph = useMemo(
    () => (snapshot?.graph ? displayGraph(snapshot) : graph),
    [snapshot, graph],
  );
  const selectedHere =
    selected?.file === path && selected.line
      ? { file: selected.file, line: selected.line, name: selected.name }
      : undefined;
  const [selection, setSelection] = useState<MeaningInput["target"]>();
  useEffect(() => {
    if (selected?.file !== path || !selected.line) return;
    const next = {
      file: selected.file,
      line: selected.line,
      name: selected.name,
    };
    setSelection((previous) =>
      previous?.file === next.file &&
      previous.line === next.line &&
      previous.name === next.name
        ? previous
        : next,
    );
  }, [selected?.file, selected?.line, selected?.name, path]);
  const target = [
    selectedHere,
    selection,
    focused,
    analysis?.input.target,
  ].find((item) => item?.file === path && item.line);
  const declaration = snapshot?.graph?.declarations.find(
    (item) =>
      target &&
      item.name === target.name &&
      item.source.file === target.file &&
      item.source.line === target.line,
  );
  const meaningTarget = declaration?.source.line
    ? {
        file: declaration.source.file,
        line: declaration.source.line,
        name: declaration.name,
      }
    : undefined;
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
    !unavailable && !sourceOnly && preparation?.outcome.state === "preparing";
  const saved = preparing && Boolean(snapshot?.compiled);
  return (
    <section className="file-split" aria-label={"File: " + path}>
      <div className="file-source-pane">
        {readerError && !file && !snapshot ? (
          <Alert severity="error">
            Could not load the source.
            <details>
              <summary>Details</summary>
              {readerError}
            </details>
            <Button onClick={onRetryReader}>Retry source</Button>
          </Alert>
        ) : sourceError && !file && !snapshot ? (
          <p className="file-empty" role="status">
            Source could not be loaded.
          </p>
        ) : (
          <NativeCode
            source={snapshot}
            observation={file}
            path={path}
            line={selectedHere?.line ?? selection?.line}
            running={running}
            onCommand={onCommand}
            explanations={preparing && !saved ? [] : explanations}
            hoveredLine={
              hovered?.source?.file === path ? hovered.source.line : undefined
            }
          />
        )}
      </div>
      <div
        className="file-graph-pane"
        role="region"
        aria-label="File reference graph"
      >
        {sourceError && (
          <Alert severity="error">
            Could not load the prepared graph.
            <details>
              <summary>Details</summary>
              {sourceError}
            </details>
            <Button onClick={onRetrySource}>Retry graph</Button>
          </Alert>
        )}
        {loading && !preparing && !sourceError && (
          <LinearProgress aria-label="Loading prepared graph" />
        )}
        {analysesError && (
          <Alert severity="warning">
            Could not load saved analyses.
            <details>
              <summary>Details</summary>
              {analysesError}
            </details>
            {onRetryAnalyses && (
              <Button onClick={onRetryAnalyses}>Retry analyses</Button>
            )}
          </Alert>
        )}
        {analysesLoading && <p role="status">Loading saved analyses…</p>}
        {preparing && !saved ? (
          <NativePreparation
            preparation={preparation}
            {...logProps}
            path={path}
            onCancel={
              state.running && state.operation === "inspection"
                ? () => {
                    void onCommand({ type: "cancel" });
                  }
                : undefined
            }
          />
        ) : loading || sourceError ? null : unavailable || sourceOnly ? (
          <p className="file-empty">
            No dependency analysis is available for this file.
          </p>
        ) : (
          <>
            {saved && (
              <details className="preparation-history">
                <summary>Checking saved analysis</summary>
                <NativePreparation
                  preparation={preparation}
                  {...logProps}
                  path={path}
                  onCancel={() => void onCommand({ type: "cancel" })}
                />
              </details>
            )}
            {preparation?.outcome.state === "failed" && (
              <Alert severity="warning">
                <p>{preparation.phase} stopped.</p>
                <p>{preparation.outcome.error.split("\n")[0]}</p>
                <details>
                  <summary>Details</summary>
                  <p>{preparation.outcome.error}</p>
                </details>
                <Button
                  onClick={() =>
                    void onCommand({
                      type: "inspect",
                      path,
                      directory: false,
                    })
                  }
                >
                  Retry preparation
                </Button>
              </Alert>
            )}
            {preparation?.outcome.state === "failed" && (
              <PreparationLog {...logProps} />
            )}
            {preparation?.outcome.state === "ready" && preparation.log && (
              <details className="preparation-history">
                <summary>Build log</summary>
                <PreparationLog {...logProps} />
              </details>
            )}
            {failure && preparation?.outcome.state !== "failed" && (
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
                camera={graphCamera}
                focusName={focused?.file === path ? focused.name : undefined}
                selection={meaningTarget}
                snapshot={snapshot}
                onHover={setHovered}
                onSource={(file, line, name) => {
                  if (file === path) setSelection({ file, line, name });
                  const found =
                    scene &&
                    [...scene.symbols.values()].find(
                      (s) => s.file === file && s.line === line,
                    );
                  if (found) onSelect({ ...found, name });
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
                  camera={graphCamera}
                  focusName={focused?.file === path ? focused.name : undefined}
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
          </>
        )}
        <div
          hidden={
            preparing ||
            loading ||
            !!sourceError ||
            unavailable ||
            sourceOnly ||
            (!snapshot && analysis?.input.target.file !== path)
          }
        >
          <NativeMeaning
            record={analysis?.input.target.file === path ? analysis : null}
            history={history ?? []}
            onSelect={(id) => onCommand({ type: "select_analysis", id })}
            onRepair={(finding_id) =>
              onCommand({ type: "prepare_repair", finding_id })
            }
            authorized={state.authorized}
            running={running}
            sourceRevision={snapshot?.revision}
            graphRevision={
              snapshot?.compiled ? snapshot.graph?.revision : undefined
            }
            target={meaningTarget}
            onRun={(input) => onCommand({ type: "analyze_meaning", input })}
          />
        </div>
      </div>
    </section>
  );
}

// Data and jobs are retained by the parent. Hidden presentation keeps its draft
// and DOM without measuring inputs or rebuilding graphs on unrelated updates.
// Activation always renders the current source, callbacks and analysis state.
export const NativeFileWorkspace = memo(
  FileWorkspace,
  (_, next) => !next.active,
);
