import { Alert, Button, Typography } from "@mui/material";
import type { VerificationJobPublication } from "@meden/agent-protocol/native";

type Data = VerificationJobPublication["data"];
type ProjectData = Extract<Data, { kind: "project" }>;
type Stage = Extract<ProjectData["phase"], { state: "running" }>["stage"];
type RecordedData = Extract<Data, { kind: "recorded" }>;
type RecordedStage = Extract<
  RecordedData["phase"],
  { state: "running" }
>["stage"];
function recordedLabel(stage: RecordedStage): string {
  switch (stage.state) {
    case "reading_record":
      return "Reading work record…";
    case "preparing":
      return "Preparing work check…";
    case "capturing":
      return "Reading check inputs…";
    case "recording_start":
      return "Saving check start…";
    case "building":
      return "Building recorded module…";
    case "inspecting":
      return "Inspecting named theorems…";
    case "refreshing":
      return "Checking source freshness…";
    case "persisting":
      return "Saving check history…";
  }
}
function stageLabel(stage: Stage): string {
  switch (stage.state) {
    case "checking_proposal":
      return "Checking contract proposal…";
    case "recording_proposal":
      return "Preparing contract review…";
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
  proposal: boolean,
  recorded: boolean,
): string {
  if (recorded)
    return phase.state === "cancelled"
      ? "Work check stopped"
      : "Work check could not finish";
  if (proposal)
    return phase.state === "cancelled"
      ? "Proposal check stopped"
      : phase.kind === "unverified"
        ? "The proposal did not pass its check"
        : "Proposal check could not finish";
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
  if (data.kind === "recorded")
    return data.phase.state === "running"
      ? recordedLabel(data.phase.stage)
      : undefined;
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
      <Typography component="p">
        {data.kind === "project" ? data.context.name : data.context.id}
      </Typography>
      <Alert severity={phase.state === "cancelled" ? "info" : "warning"}>
        {stoppedLabel(
          phase,
          data.kind === "project" && data.context.subject.state === "proposal",
          data.kind === "recorded",
        )}
        <details>
          <summary>Details</summary>
          <pre>{phase.message}</pre>
        </details>
      </Alert>
    </article>
  );
}
