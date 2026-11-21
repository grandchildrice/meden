import { lazy, Suspense, useEffect, useMemo, useState } from "react";
import { Alert, Chip, CircularProgress, Typography } from "@mui/material";
import { workspaceSchema, type Declaration } from "@meden/workspace-schema";
import type {
  SourceSnapshot,
  MeaningRecord,
  MeaningInput,
} from "@meden/agent-protocol/native";
import { sourceIndex } from "@meden/atlas-adapter/source-index";
import { importAtlas } from "@meden/atlas-adapter";
import { NativeMeaning } from "./NativeMeaning";
const GraphView = lazy(() =>
  import("../graph/GraphView").then((module) => ({
    default: module.GraphView,
  })),
);

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
}) {
  const [selected, setSelected] = useState<Declaration>();
  const result = useMemo(() => {
    try {
      return {
        workspace: workspaceSchema.parse({
          schemaVersion: 1,
          id: "native-sources",
          name: "Selected local sources",
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
        <Alert severity="info">
          {snapshot.atlas
            ? `Compiled dependencies from Lean Atlas (Lean ${snapshot.atlas.graph.metadata.leanVersion}). Local imports are collected automatically; external libraries are shown as unexpanded boundaries. Contract and proof acceptance are unchanged.`
            : "Lexical reference candidates from selected files. No compiler dependency extraction or proof checking was performed."}
        </Alert>

        {snapshot.collection && (
          <Typography variant="body2">
            Collected local dependencies. Roots:{" "}
            {snapshot.collection.roots.join(", ")}.
            {snapshot.collection.packages
              .map(
                (pkg) => `Lake: ${pkg.name} (${pkg.source_roots.join(", ")})`,
              )
              .join(" · ")}
            External module boundaries:{" "}
            {snapshot.collection.external_modules.join(", ") || "None"}.
          </Typography>
        )}
        <Typography variant="caption">
          {Object.keys(snapshot.files).length} files ·{" "}
          {workspace.declarations.length} declarations ·{" "}
          {workspace.edges.length}
          {snapshot.atlas ? " dependencies" : " reference candidates"}
        </Typography>
      </details>
      <Suspense
        fallback={<CircularProgress aria-label="Loading graph" size={24} />}
      >
        {workspace.declarations.length ? (
          <GraphView
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
          <p>
            No supported Rust or Lean declarations were found. Check the
            selected files.
          </p>
        )}
      </Suspense>
      {selected && (
        <div className="native-declaration" aria-label="Selected declaration">
          <Typography component="h3">{selected.name}</Typography>
          <Chip
            label={
              selected.proofStatus === "unproved"
                ? "Unfinished proof detected"
                : "Proof unchecked"
            }
            size="small"
          />
          <pre>{selected.statement}</pre>
          <Typography variant="caption" component="p">
            References:{" "}
            {workspace.edges
              .filter((edge) => edge.source === selected.id)
              .map(
                (edge) =>
                  workspace.declarations.find(
                    (declaration) => declaration.id === edge.target,
                  )?.name,
              )
              .join(", ") || "Not found in this index"}
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
    </section>
  );
}
