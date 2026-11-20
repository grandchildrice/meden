import { useState } from "react";
import { Alert, Button, Stack, TextField, Typography } from "@mui/material";
import type { NativePreviewSnapshot } from "@meden/agent-protocol/native";

export function NativePreview({
  snapshot,
  error,
  running,
  onCapture,
}: {
  snapshot: NativePreviewSnapshot | null;
  error: string | null;
  running: boolean;
  onCapture: (root: string, entry: string) => Promise<boolean>;
}) {
  const [root, setRoot] = useState(".");
  const [entry, setEntry] = useState("index.html");
  return (
    <Stack spacing={2}>
      <Typography variant="h3">アプリのプレビュー</Typography>
      {error && <Alert severity="error">{error}</Alert>}
      <TextField
        label="公開フォルダ（ワークスペース内）"
        size="small"
        value={root}
        onChange={(event) => setRoot(event.target.value)}
      />
      <TextField
        label="開始するHTMLファイル"
        size="small"
        value={entry}
        onChange={(event) => setEntry(event.target.value)}
      />
      <Button
        variant="outlined"
        disabled={running || !root.trim() || !entry.trim()}
        onClick={() => void onCapture(root, entry)}
      >
        ファイルを読み直してプレビュー
      </Button>
      {snapshot ? (
        <>
          <details>
            <summary>プレビューの情報</summary>
            <Typography variant="body2" sx={{ overflowWrap: "anywhere" }}>
              {snapshot.root}/{snapshot.entry} · {snapshot.file_count}ファイル ·{" "}
              {snapshot.byte_count.toLocaleString()} bytes
            </Typography>
            <Typography variant="caption" sx={{ overflowWrap: "anywhere" }}>
              {snapshot.revision}
            </Typography>
            <Typography variant="body2">
              読み込み時点のローカルファイルを表示します。ブラウザ内の操作確認であり、証明結果ではありません。
              ビルドが必要なアプリは出力フォルダを選んでください。サーバー処理・外部サービス・永続ストレージは利用できません。
            </Typography>
          </details>
          {snapshot.errors.length > 0 && (
            <Alert severity="warning">
              読み込めないファイルがあります。出力フォルダと参照先を確認して、読み直してください。
              <ul>
                {snapshot.errors.map((error, index) => (
                  <li key={index}>{error}</li>
                ))}
              </ul>
            </Alert>
          )}
          <iframe
            key={snapshot.url}
            title="ローカルアプリのプレビュー"
            src={snapshot.url}
            sandbox="allow-scripts allow-forms"
            referrerPolicy="no-referrer"
            className="native-preview"
          />
        </>
      ) : (
        <p>フォルダとHTMLファイルを選んで読み込んでください。</p>
      )}
    </Stack>
  );
}
