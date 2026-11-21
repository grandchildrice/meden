import { useEffect, useState } from "react";
import { Alert, Button, Chip, TextField, Typography } from "@mui/material";
import type { MeaningInput, MeaningRecord } from "@meden/agent-protocol/native";

type Target = MeaningInput["target"];
export function NativeMeaning({
  record,
  history,
  onSelect,
  onRepair,
  target,
  sourceRevision,
  graphRevision,
  authorized,
  running,
  onRun,
}: {
  record: MeaningRecord | null;
  history: MeaningRecord[];
  onSelect: (id: string) => Promise<boolean>;
  onRepair: (finding: string) => Promise<boolean>;
  target?: Target;
  sourceRevision?: string;
  graphRevision?: string;
  authorized: boolean;
  running: boolean;
  onRun: (input: MeaningInput) => Promise<boolean>;
}) {
  const [requirement, setRequirement] = useState("");
  useEffect(() => {
    setRequirement(
      record && sameTarget(record.input.target, target)
        ? record.input.requirement
        : "",
    );
  }, [target?.file, target?.line, target?.name, record?.id]);
  const sameInput = Boolean(
    record &&
    target &&
    sourceRevision === record?.input.source_revision &&
    graphRevision === record?.input.graph_revision &&
    sameTarget(record.input.target, target) &&
    requirement === record.input.requirement,
  );
  const current = !running && record?.state === "complete" && sameInput;
  const result = record?.assessment;
  const active =
    record && ["observing", "translating", "comparing"].includes(record.state);
  const verdict =
    result?.verdict === "match"
      ? "Candidate match"
      : result?.verdict === "mismatch"
        ? "Candidate mismatch"
        : "Undetermined";
  return (
    <section className="native-meaning" aria-label="Meaning analysis">
      {history.length > 0 && (
        <TextField
          select
          fullWidth
          size="small"
          label="Saved analyses"
          value={record?.id || ""}
          disabled={running}
          slotProps={{ select: { native: true } }}
          onChange={(event) => void onSelect(event.target.value)}
        >
          {history.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.sequence}. {entry.input.target.name} ·{" "}
              {entry.input.requirement.slice(0, 80)} · {entry.state}
            </option>
          ))}
        </TextField>
      )}
      <Typography component="h3">
        Compare requirements with declaration meaning
      </Typography>
      {target && sourceRevision && (
        <>
          <TextField
            label="Requirement to compare"
            value={requirement}
            onChange={(event) => setRequirement(event.target.value)}
            multiline
            minRows={3}
            fullWidth
            size="small"
          />
          <Button
            disabled={running || !authorized || !requirement.trim()}
            onClick={() =>
              void onRun({
                source_revision: sourceRevision,
                ...(graphRevision ? { graph_revision: graphRevision } : {}),
                target,
                requirement,
              })
            }
          >
            Analyze declaration meaning
          </Button>
          {record?.finding_id && (
            <Button
              disabled={
                running ||
                !authorized ||
                !sameInput ||
                !["complete", "historical"].includes(record.state)
              }
              onClick={() => void onRepair(record.finding_id!)}
            >
              Prepare patch for this Finding
            </Button>
          )}
          {!authorized && (
            <p>Review and approve a connection to run this analysis.</p>
          )}
          <Typography variant="caption" component="p">
            {graphRevision
              ? "Translate the elaborated declaration and meaning-related dependencies. The first request excludes original source, comments and proof-only dependencies."
              : "Translate the selected file into natural language."}
            Compare with the requirement in a separate request. A separate judge
            can be configured at launch. Both model identities are recorded.
            This does not change proof or contract acceptance.
          </Typography>
        </>
      )}
      {record && (
        <div aria-label="Meaning analysis result">
          {active && (
            <p role="status">
              {
                (
                  {
                    observing: "Rechecking target source",
                    translating:
                      "Translating declaration (requirement not sent)",
                    comparing: "Comparing with original requirement",
                  } as Record<string, string>
                )[record.state]
              }
            </p>
          )}
          {record.state === "historical" && (
            <Alert severity="info">
              Historical analysis. Refresh the graph and explicitly rerun it for
              the current source.
            </Alert>
          )}
          {record.state === "stale" && (
            <Alert severity="warning">
              The source changed. Refresh the graph and rerun the analysis.
            </Alert>
          )}
          {record.state === "complete" && !current && (
            <Alert severity="warning">
              {running
                ? "An operation is running. The displayed analysis is from the previous run."
                : "This analysis uses different requirements, declarations or source revisions. Rerun it."}
            </Alert>
          )}
          {record.state === "cancelled" && (
            <Alert severity="warning">
              Meaning analysis was cancelled. Explicitly request a new run to
              retry.
            </Alert>
          )}
          {record.state === "failed" && (
            <Alert severity="error">
              Meaning analysis failed. Check the input and connection, then
              retry.
            </Alert>
          )}
          {record.error && <pre>{record.error}</pre>}
          {!result && record.partial_translation && (
            <details>
              <summary>
                Candidate translation from an incomplete comparison
              </summary>
              <pre>{record.partial_translation.naturalLanguage}</pre>
            </details>
          )}
          {result && (
            <>
              <Chip
                label={`${verdict} (Model assessment${current ? "" : " · Historical target"})`}
                color={
                  current && result.verdict === "mismatch"
                    ? "warning"
                    : "default"
                }
              />
              <Typography variant="caption" component="p">
                This is not proof. Natural-language meaning and analysis
                accuracy are not formally verified.
              </Typography>
              <div className="native-meaning-comparison">
                <div>
                  <Typography component="h4">
                    Requirement at analysis time
                  </Typography>
                  <pre>{record.input.requirement}</pre>
                </div>
                <div>
                  <Typography component="h4">
                    Declaration translated into natural language
                  </Typography>
                  <pre>{result.translation.naturalLanguage}</pre>
                </div>
              </div>
              <Typography component="h4">Assumptions</Typography>
              <pre>{result.translation.preconditions}</pre>
              <Typography component="h4">Conclusion and behavior</Typography>
              <pre>{result.translation.postcondition}</pre>
              <Typography component="h4">Scope</Typography>
              <pre>{result.translation.scope}</pre>
              <Typography component="h4">Differences and reasoning</Typography>
              <pre>
                {[result.comparison.discrepancy, result.comparison.explanation]
                  .filter(Boolean)
                  .join("\n\n")}
              </pre>
            </>
          )}
          <details>
            <p className="native-revision">Analysis ID: {record.id}</p>
            <p className="native-revision">
              Requirement ID: {record.requirement_id}
            </p>
            {record.finding_id && (
              <p className="native-revision">Finding ID: {record.finding_id}</p>
            )}
            <summary>Analysis target and process details</summary>
            <p>
              {record.input.target.name} · {record.input.target.file}:
              {record.input.target.line}
            </p>
            <p>
              Model: {record.model} · {record.engine}
            </p>
            {record.source_artifact && (
              <p className="native-revision">
                Source artifact at analysis time: {record.source_artifact}
              </p>
            )}
            <p className="native-revision">
              Translation: {record.model} · {record.processor}
            </p>
            <p className="native-revision">
              Comparison: {record.judge?.model ?? record.model} ·{" "}
              {record.judge?.endpoint ?? record.processor}
            </p>
            <p className="native-revision">{record.input.source_revision}</p>
            <p>
              Meden applies claimcheck's two-stage approach to Rust and Lean
              using separate requests, without the original conversation. It
              does not guarantee complete dependency extraction or analysis
              accuracy.
            </p>
          </details>
        </div>
      )}
    </section>
  );
}
function sameTarget(left: Target, right?: Target) {
  return Boolean(
    right &&
    left.file === right.file &&
    left.line === right.line &&
    left.name === right.name,
  );
}
