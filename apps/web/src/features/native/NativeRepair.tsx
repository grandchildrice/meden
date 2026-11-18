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
              修正差分と元の要件を確認
            </Typography>
            <Typography>変更するファイル: {review.file}</Typography>
            <Typography variant="body2">
              差分ID: {review.reference.sha256}
            </Typography>
            <Typography variant="body2">
              Finding: {review.finding_id}
            </Typography>
            <Typography variant="body2">
              要件ID: {review.requirement_id}
            </Typography>
            <Typography variant="body2">
              受理済み契約: {review.accepted_reference?.contract || "なし"}
            </Typography>
          </DialogTitle>
          <DialogContent dividers>
            <Alert severity="info">
              承認するとこの差分を一度だけ適用し、元の要件で再解析します。既存の証明結果は失効し、再解析の一致は証明済みを意味しません。
            </Alert>
            <Typography component="h3">元の要件</Typography>
            <pre>{review.input.requirement}</pre>
            <Typography variant="body2">生成モデル: {review.model}</Typography>
            <Typography component="h3">修正の理由</Typography>
            <pre>{review.explanation}</pre>
            <pre aria-label="適用する正確な修正差分">{review.diff}</pre>
          </DialogContent>
          <DialogActions>
            <Button disabled={running} onClick={() => act("reject_repair")}>
              修正を拒否
            </Button>
            <Button
              disabled={running || visible !== review.reference.sha256}
              onClick={() => act("apply_repair")}
            >
              この差分を一度適用して再解析
            </Button>
          </DialogActions>
        </>
      )}
    </Dialog>
  );
}
