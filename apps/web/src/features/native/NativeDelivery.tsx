import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Stack,
  Typography,
} from "@mui/material";
import type {
  NativeCommand,
  ExportOutcome,
  ExportReview,
  ExportSummary,
} from "@meden/agent-protocol/native";

const states: Record<ExportSummary["state"], string> = {
  pending: "Pending review",
  reviewed: "Under review",
  rejected: "Rejected",
  attempted: "Attempted",
  response_received: "Response received",
  delivery_uncertain: "Delivery uncertain; do not resend",
};

export function NativeDelivery({
  exports,
  review,
  outcome,
  running,
  onCommand,
}: {
  exports: ExportSummary[];
  review: ExportReview | null;
  outcome: ExportOutcome | null;
  running: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
}) {
  const [heading, setHeading] = useState<HTMLDivElement | null>(null);
  const [visibleSerial, setVisibleSerial] = useState<ExportReview["serial"]>();
  useEffect(() => {
    setVisibleSerial(undefined);
    const node = heading;
    if (!review || !node) return;
    const serial = review.serial;
    const observer = new IntersectionObserver(
      ([entry]) => {
        setVisibleSerial(
          entry?.isIntersecting && entry.intersectionRatio === 1
            ? serial
            : undefined,
        );
      },
      { threshold: [0, 1] },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [review?.serial, heading]);
  const act = (type: "send_export" | "reject_export" | "close_export") => {
    if (review)
      void onCommand({
        type,
        reference: { session: review.summary.session, id: review.summary.id },
        serial: review.serial,
      });
  };
  return (
    <Stack spacing={2} aria-label="Artifact delivery">
      <Typography variant="h3">Artifact delivery</Typography>
      <Typography>
        Review the prepared content and destination before sending once.
        Reviewing does not send anything.
      </Typography>
      {exports.length === 0 && (
        <p>
          No delivery proposed. Specify a file and destination to prepare one.
        </p>
      )}
      {outcome && (
        <Alert
          severity={
            outcome.summary.state === "delivery_uncertain" ? "warning" : "info"
          }
        >
          Delivery #{outcome.summary.id} · {states[outcome.summary.state]}
          {outcome.http_status !== null && <> · HTTP {outcome.http_status}</>}
          {outcome.summary.state === "delivery_uncertain" && (
            <p>
              The recipient may have received it. Check with the recipient
              instead of sending again.
            </p>
          )}
        </Alert>
      )}
      {exports.map((item) => (
        <Stack
          spacing={1}
          key={`${item.session}:${item.id}`}
          className="native-delivery-entry"
        >
          <Typography component="h3">
            Delivery #{item.id} · {item.source}
          </Typography>
          <Typography sx={{ overflowWrap: "anywhere" }}>
            {item.target}
          </Typography>
          <Chip size="small" label={states[item.state]} />
          <Typography variant="body2">
            {item.bytes.toLocaleString("en-US")} bytes ·{" "}
            {item.contract
              ? "Verified contract package"
              : "Ordinary file; no proof"}
          </Typography>
          <Button
            disabled={running}
            onClick={() =>
              void onCommand({
                type: "review_export",
                reference: { session: item.session, id: item.id },
                offset: 0,
              })
            }
          >
            Delivery #{item.id} : review content and destination
          </Button>
        </Stack>
      ))}
      <Dialog
        open={Boolean(review)}
        onClose={() => {
          if (!running) act("close_export");
        }}
        fullWidth
        maxWidth="md"
        aria-labelledby="native-delivery-title"
      >
        {review && (
          <>
            <DialogTitle
              id="native-delivery-title"
              component="div"
              ref={setHeading}
              className="native-delivery-heading"
            >
              <Typography component="h2" variant="h3">
                Review delivery content and destination
              </Typography>
              <Typography>
                Delivery #{review.summary.id} · POST · application/octet-stream
              </Typography>
              <Typography sx={{ overflowWrap: "anywhere" }}>
                Destination: {review.summary.target}
              </Typography>
              <Typography variant="body2" sx={{ overflowWrap: "anywhere" }}>
                Frozen content: {review.summary.bytes.toLocaleString("en-US")}{" "}
                bytes · SHA-256 {review.summary.payload_sha256}
              </Typography>
              <Typography variant="body2" sx={{ overflowWrap: "anywhere" }}>
                Contract:{" "}
                {review.summary.contract || "None (Ordinary file; no proof)"}
              </Typography>
            </DialogTitle>
            <DialogContent dividers>
              <Typography>
                These are the exact frozen bytes. Later source edits do not
                change this content or destination.
              </Typography>
              <Typography>
                Status: {states[review.summary.state]} · Byte offset{" "}
                {review.offset}〜{review.offset + review.body_hex.length / 2}
              </Typography>
              <pre
                className="native-delivery-bytes"
                aria-label="Exact bytes to send"
              >
                {hexDump(review.body_hex, review.offset) || "(Empty file)"}
              </pre>
              <Stack direction="row" spacing={2}>
                <Button
                  disabled={running || review.offset === 0}
                  onClick={() =>
                    void onCommand({
                      type: "review_export",
                      reference: {
                        session: review.summary.session,
                        id: review.summary.id,
                      },
                      offset: Math.max(0, review.offset - 2048),
                    })
                  }
                >
                  Previous bytes
                </Button>
                <Button
                  disabled={
                    running || review.offset + 2048 >= review.summary.bytes
                  }
                  onClick={() =>
                    void onCommand({
                      type: "review_export",
                      reference: {
                        session: review.summary.session,
                        id: review.summary.id,
                      },
                      offset: review.offset + 2048,
                    })
                  }
                >
                  Next bytes
                </Button>
              </Stack>
              {visibleSerial !== review.serial && (
                <Alert severity="info">
                  Enlarge the window until the complete destination and content
                  heading is visible.
                </Alert>
              )}
            </DialogContent>
            <DialogActions sx={{ flexWrap: "wrap" }}>
              <Button disabled={running} onClick={() => act("close_export")}>
                Close
              </Button>
              <Button
                disabled={
                  running ||
                  !["pending", "reviewed"].includes(review.summary.state)
                }
                onClick={() => act("reject_export")}
              >
                Reject this delivery
              </Button>
              <Button
                variant="contained"
                disabled={
                  running ||
                  visibleSerial !== review.serial ||
                  review.summary.state !== "reviewed"
                }
                onClick={() => act("send_export")}
              >
                Send once to this destination
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Stack>
  );
}

function hexDump(hex: string, offset: number): string {
  const bytes = hex.match(/.{2}/g) || [];
  const rows: string[] = [];
  for (let i = 0; i < bytes.length; i += 16) {
    const row = bytes.slice(i, i + 16);
    rows.push(
      `${(offset + i).toString(16).padStart(8, "0")}  ${row.join(" ").padEnd(47)}  | ${row
        .map((byte) => {
          const value = parseInt(byte, 16);
          return value >= 32 && value <= 126 ? String.fromCharCode(value) : ".";
        })
        .join("")}`,
    );
  }
  return rows.join("\n");
}
