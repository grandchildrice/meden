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
  pending: "確認待ち",
  reviewed: "内容確認中",
  rejected: "拒否済み",
  attempted: "送信試行済み",
  response_received: "応答受信済み",
  delivery_uncertain: "配送結果不明・再送不可",
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
  const [visibleSerial, setVisibleSerial] = useState<number>();
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
    <Stack spacing={2} aria-label="成果物の配送">
      <Typography variant="h3">成果物の配送</Typography>
      <Typography>
        エージェントが準備した内容と宛先を確認して、1回だけ送信できます。確認だけでは送信されません。
      </Typography>
      {exports.length === 0 && (
        <p>
          配送の提案はありません。必要なら、送るファイルと宛先を指示してください。
        </p>
      )}
      {outcome && (
        <Alert
          severity={
            outcome.summary.state === "delivery_uncertain" ? "warning" : "info"
          }
        >
          配送 #{outcome.summary.id} · {states[outcome.summary.state]}
          {outcome.http_status !== null && <> · HTTP {outcome.http_status}</>}
          {outcome.summary.state === "delivery_uncertain" && (
            <p>
              相手側に届いている可能性があります。送信を繰り返さず、受信側で確認してください。
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
            配送 #{item.id} · {item.source}
          </Typography>
          <Typography sx={{ overflowWrap: "anywhere" }}>
            {item.target}
          </Typography>
          <Chip size="small" label={states[item.state]} />
          <Typography variant="body2">
            {item.bytes.toLocaleString()} bytes ·{" "}
            {item.contract
              ? "検証した契約のパッケージ"
              : "通常のファイル・証明なし"}
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
            配送 #{item.id} の内容と宛先を確認
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
                この配送の内容と宛先を確認
              </Typography>
              <Typography>
                配送 #{review.summary.id} · POST · application/octet-stream
              </Typography>
              <Typography sx={{ overflowWrap: "anywhere" }}>
                送信先: {review.summary.target}
              </Typography>
              <Typography variant="body2" sx={{ overflowWrap: "anywhere" }}>
                固定した内容: {review.summary.bytes.toLocaleString()} bytes ·
                SHA-256 {review.summary.payload_sha256}
              </Typography>
              <Typography variant="body2" sx={{ overflowWrap: "anywhere" }}>
                契約:{" "}
                {review.summary.contract || "なし（通常のファイル・証明なし）"}
              </Typography>
            </DialogTitle>
            <DialogContent dividers>
              <Typography>
                送信する固定済みのバイト列です。準備後に元ファイルが変わっても、この内容と宛先は変わりません。
              </Typography>
              <Typography>
                状態: {states[review.summary.state]} · バイト位置{" "}
                {review.offset}〜{review.offset + review.body_hex.length / 2}
              </Typography>
              <pre
                className="native-delivery-bytes"
                aria-label="送信する正確なバイト列"
              >
                {hexDump(review.body_hex, review.offset) || "（空のファイル）"}
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
                  前の内容
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
                  次の内容
                </Button>
              </Stack>
              {visibleSerial !== review.serial && (
                <Alert severity="info">
                  送信先と内容の見出しをすべて表示できる大きさに画面を広げてください。
                </Alert>
              )}
            </DialogContent>
            <DialogActions sx={{ flexWrap: "wrap" }}>
              <Button disabled={running} onClick={() => act("close_export")}>
                閉じる
              </Button>
              <Button
                disabled={
                  running ||
                  !["pending", "reviewed"].includes(review.summary.state)
                }
                onClick={() => act("reject_export")}
              >
                この配送を拒否
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
                この宛先へ1回送信
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
