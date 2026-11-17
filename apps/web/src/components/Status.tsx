import { Chip } from "@mui/material";
import CheckCircleOutlineRounded from "@mui/icons-material/CheckCircleOutlineRounded";
import ScheduleRounded from "@mui/icons-material/ScheduleRounded";
import ErrorOutlineRounded from "@mui/icons-material/ErrorOutlineRounded";
import {
  proofLabels,
  verdictLabels,
  type ProofStatus,
  type Analysis,
} from "@meden/workspace-schema";

export function ProofStatusChip({ status }: { status: ProofStatus }) {
  const good =
    status === "proved" || status === "bounded" || status === "empirical";
  return (
    <Chip
      size="small"
      className={
        "status-chip " +
        (good ? "positive" : status === "refuted" ? "negative" : "neutral")
      }
      icon={good ? <CheckCircleOutlineRounded /> : <ScheduleRounded />}
      label={proofLabels[status]}
    />
  );
}
export function AnalysisStatusChip({
  analysis,
  current = true,
}: {
  analysis?: Analysis;
  current?: boolean;
}) {
  if (!analysis) return <Chip size="small" label="未解析" variant="outlined" />;
  if (!current || analysis.status === "stale")
    return (
      <Chip
        size="small"
        icon={<ScheduleRounded />}
        label="再解析が必要"
        className="status-chip neutral"
      />
    );
  if (analysis.status !== "complete")
    return (
      <Chip
        size="small"
        label={analysis.status === "failed" ? "解析失敗" : "解析待ち"}
        variant="outlined"
      />
    );
  const good = analysis.verdict === "full";
  return (
    <Chip
      size="small"
      icon={good ? <CheckCircleOutlineRounded /> : <ErrorOutlineRounded />}
      label={verdictLabels[analysis.verdict]}
      className={"status-chip " + (good ? "positive" : "attention")}
    />
  );
}
