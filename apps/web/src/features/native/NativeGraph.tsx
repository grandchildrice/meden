import { lazy, Suspense, useMemo } from "react";
import { Alert, CircularProgress, Typography } from "@mui/material";
import { explanationSchema, type Declaration } from "@meden/workspace-schema";
import type {
  SourceSnapshot,
  MeaningInput,
} from "@meden/agent-protocol/native";
import type { GraphCameraStore } from "../graph/GraphView";
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
  selection,
  onSource,
  onHover,
  camera,
}: {
  snapshot: SourceSnapshot;
  focusName?: string;
  selection?: MeaningInput["target"];
  onSource: (file: string, line: number, name: string) => void;
  onHover?: (declaration: Declaration | undefined) => void;
  camera?: GraphCameraStore;
}) {
  const result = useMemo(() => {
    try {
      return { workspace: displayGraph(snapshot) };
    } catch (error) {
      return { error: String(error) };
    }
  }, [snapshot]);
  const focused = result.workspace?.declarations.find(
    (d) => d.name === focusName,
  );
  const selected = result.workspace?.declarations.find(
    (d) =>
      selection &&
      d.name === selection.name &&
      d.source?.file === selection.file &&
      d.source?.line === selection.line,
  );
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
            camera={camera}
            focusName={focusName}
            onHover={onHover}
            workspace={workspace}
            focusId={focused?.id}
            selectedId={selected?.id}
            onSelect={(declaration) => {
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
    </section>
  );
}
