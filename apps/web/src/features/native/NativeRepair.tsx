import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Typography,
} from "@mui/material";
import type { NativeCommand, RepairReview } from "@meden/agent-protocol/native";

/** The fixed authority heading stays outside the scrolling requirement/diff. */
export function NativeRepair({
  review,
  running,
  onCommand,
}: {
  review: RepairReview | null;
  running: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
}) {
  const [heading, setHeading] = useState<HTMLDivElement | null>(null);
  const [visible, setVisible] = useState<string>();
  useEffect(() => {
    setVisible(undefined);
    if (!review || !heading) return;
    const hash = review.reference.sha256;
    const observer = new IntersectionObserver(
      ([entry]) => {
        setVisible(
          entry?.isIntersecting && entry.intersectionRatio === 1
            ? hash
            : undefined,
        );
      },
      { threshold: [0, 1] },
    );
    observer.observe(heading);
    return () => observer.disconnect();
  }, [review?.reference.sha256, heading]);
  function act(type: "apply_repair" | "reject_repair") {
    if (review) void onCommand({ type, reference: review.reference });
  }
  return (
    <Dialog
      open={Boolean(review)}
      fullWidth
      maxWidth="md"
      aria-labelledby="native-repair-title"
      onClose={() => {
        if (!running) act("reject_repair");
      }}
    >
      {review && (
        <>
          <DialogTitle
            id="native-repair-title"
            component="div"
            ref={setHeading}
            sx={{ overflowWrap: "anywhere" }}
          >
            <Typography component="h2" variant="h3">
              Review patch and original requirement
            </Typography>
            <Typography>File to change: {review.file}</Typography>
            <Typography variant="body2">
              Patch ID: {review.reference.sha256}
            </Typography>
            <Typography variant="body2">
              Finding: {review.finding_id}
            </Typography>
            <Typography variant="body2">
              Requirement ID: {review.requirement_id}
            </Typography>
            <Typography variant="body2">
              Accepted contract: {review.accepted_reference?.contract || "None"}
            </Typography>
          </DialogTitle>
          <DialogContent dividers>
            <Alert severity="info">
              Approval applies this exact patch once and reanalyzes the original
              requirement. Previous proof results are invalidated. A matching
              analysis is not a proof.
            </Alert>
            <Typography component="h3">Original requirement</Typography>
            <pre>{review.input.requirement}</pre>
            <Typography variant="body2">
              Generation model: {review.model}
            </Typography>
            <Typography component="h3">Patch rationale</Typography>
            <pre>{review.explanation}</pre>
            <pre aria-label="Exact patch to apply">{review.diff}</pre>
          </DialogContent>
          <DialogActions>
            <Button disabled={running} onClick={() => act("reject_repair")}>
              Reject patch
            </Button>
            <Button
              disabled={running || visible !== review.reference.sha256}
              onClick={() => act("apply_repair")}
            >
              Apply this patch once and reanalyze
            </Button>
          </DialogActions>
        </>
      )}
    </Dialog>
  );
}
