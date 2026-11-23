import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import { Alert, CircularProgress, Typography } from "@mui/material";
import { explanationSchema, type Declaration } from "@meden/workspace-schema";
import type {
  SourceSnapshot,
  MeaningRecord,
  MeaningInput,
} from "@meden/agent-protocol/native";
import { NativeMeaning } from "./NativeMeaning";
const GraphView = lazy(() =>
  import("../graph/GraphView").then((module) => ({
    default: module.GraphView,
  })),
);

export function displayGraph(snapshot: SourceSnapshot) {
  return {
    declarations: (snapshot.graph?.declarations || []).map((node) => ({
      ...node,
      source: { file: node.source.file, line: node.source.line ?? undefined },
      semantic: node.semantic
        ? explanationSchema.parse(node.semantic)
        : undefined,
      evidenceRefs: [],
    })),
    edges: snapshot.graph?.edges || [],
  };
}

export function NativeGraph({
  snapshot,
  focusName,
  analysis,
  history,
  onSelectAnalysis,
  onRepair,
  authorized,
  running,
  onAnalyze,
  onSource,
  onHover,
}: {
  snapshot: SourceSnapshot;
  focusName?: string;
  analysis: MeaningRecord | null;
  history: MeaningRecord[];
  onSelectAnalysis: (id: string) => Promise<boolean>;
  onRepair: (finding: string) => Promise<boolean>;
  authorized: boolean;
  running: boolean;
  onAnalyze: (input: MeaningInput) => Promise<boolean>;
  onSource: (file: string, line: number, name: string) => void;
  onHover?: (declaration: Declaration | undefined) => void;
}) {
  const [selected, setSelected] = useState<Declaration>();
  const result = useMemo(() => {
    try {
      return { workspace: displayGraph(snapshot) };
    } catch (error) {
      return { error: String(error) };
    }
  }, [snapshot.revision, snapshot.graph?.revision]);
  const focused = result.workspace?.declarations.find(
    (d) => d.name === focusName,
  );
  useEffect(() => {
    if (focused) setSelected(focused);
  }, [focused]);
  useEffect(() => {
    if (!focused && !selected && analysis && result.workspace) {
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
  }, [analysis?.input.target.name, result.workspace, focused]);
  if (!result.workspace)
    return (
      <Alert severity="error">
        Could not display the source graph. {result.error}
      </Alert>
    );
  const workspace = result.workspace;
  return (
    <section className="native-source-graph" aria-label="Local source graph">
      <details className="native-graph-information">
        <summary>Analysis information</summary>
        <Alert severity="info">{snapshot.description}</Alert>
        {snapshot.external.length > 0 && (
          <Typography variant="caption">
            External module boundaries: {snapshot.external.join(", ")}.
          </Typography>
        )}
        <Typography variant="caption">
          {Object.keys(snapshot.files).length} files ·{" "}
          {workspace.declarations.length} declarations ·{" "}
          {workspace.edges.length}
          {snapshot.compiled ? " dependencies" : " reference candidates"}
        </Typography>
      </details>
      <Suspense
        fallback={<CircularProgress aria-label="Loading graph" size={24} />}
      >
        {workspace.declarations.length ? (
          <GraphView
            onHover={onHover}
            workspace={workspace}
            focusId={focused?.id}
            selectedId={selected?.id}
            onSelect={(declaration) => {
              setSelected(declaration);
              if (declaration.source?.line)
                onSource(
                  declaration.source.file,
                  declaration.source.line,
                  declaration.name,
                );
            }}
          />
        ) : (
          <p>No declaration graph is available for these retained inputs.</p>
        )}
      </Suspense>
      <NativeMeaning
        record={analysis}
        history={history}
        onSelect={onSelectAnalysis}
        onRepair={onRepair}
        sourceRevision={snapshot.revision}
        graphRevision={snapshot.compiled ? snapshot.graph?.revision : undefined}
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
    </section>
  );
}
