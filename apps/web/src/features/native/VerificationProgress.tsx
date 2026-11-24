import { Alert, Button, Typography } from "@mui/material";
import type { VerificationJobPublication } from "@meden/agent-protocol/native";

type Data = VerificationJobPublication["data"];
type Stage = Extract<Data["phase"], { state: "running" }>["stage"];
function stageLabel(stage: Stage): string {
  switch (stage.state) {
    case "reading_source":
      return "Reading source…";
    case "preparing_checker":
      return "Preparing verification…";
    case "checking":
      return "Checking the accepted contract…";
    case "refreshing_source":
      return "Checking source freshness…";
    case "retaining_evidence":
      return "Saving verification results…";
  }
}
function stoppedLabel(
  phase: Extract<Data["phase"], { state: "failed" | "cancelled" }>,
): string {
  if (phase.state === "cancelled") return "Verification stopped";
  return phase.kind === "unverified"
    ? "The check did not establish the contract"
    : phase.kind === "unavailable"
      ? "Verification unavailable"
      : "Verification did not finish";
}

/** Use the existing chat activity row, without another progress panel. */
export function verificationActivity(
  data: Data | undefined,
  loading: boolean,
): string | undefined {
  if (!data) return loading ? "Loading verification status…" : undefined;
  return data.phase.state === "running"
    ? stageLabel(data.phase.stage)
    : undefined;
}

/** Render only original prepared observations. Retry reads the same view; it
 * never dispatches a check, approves a model or interprets verifier syntax.
 */
export function VerificationProgress({
  data,
  error,
  retry,
}: {
  data?: Data;
  error?: string;
  retry?: () => void;
}) {
  if (error)
    return (
      <article className="chat-contract" aria-label="Verification progress">
        <Alert severity="warning">
          Could not load verification progress.
          <details>
            <summary>Details</summary>
            {error}
          </details>
          {retry && <Button onClick={retry}>Retry check status</Button>}
        </Alert>
      </article>
    );
  const phase = data?.phase;
  if (
    !data ||
    !phase ||
    (phase.state !== "failed" && phase.state !== "cancelled")
  )
    return null;
  return (
    <article className="chat-contract" aria-label="Verification progress">
      <Typography component="p">{data.context.name}</Typography>
      <Alert severity={phase.state === "cancelled" ? "info" : "warning"}>
        {stoppedLabel(phase)}
        <details>
          <summary>Details</summary>
          <pre>{phase.message}</pre>
        </details>
      </Alert>
    </article>
  );
}
