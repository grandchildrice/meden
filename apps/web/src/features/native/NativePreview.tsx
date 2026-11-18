import { useState } from "react";
import {
  Alert,
  Button,
  Chip,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import type { NativePreviewSnapshot } from "@meden/agent-protocol/native";

export function NativePreview({
  snapshot,
  running,
  onCapture,
}: {
  snapshot: NativePreviewSnapshot | null;
  running: boolean;
  onCapture: (root: string, entry: string) => Promise<boolean>;
}) {
  const [root, setRoot] = useState(".");
  const [entry, setEntry] = useState("index.html");
  return (
    <Stack spacing={2}>
      <Typography variant="h3">アプリのプレビュー</Typography>
      <Typography>
        ローカルのHTML・JavaScript・CSSを開いて操作できます。ビルドが必要なアプリは、指示でビルドしてから出力フォルダを選んでください。
      </Typography>
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
          <Chip label="読み込んだファイルの動作確認・証明なし" />
          <Typography variant="body2" sx={{ overflowWrap: "anywhere" }}>
            {snapshot.root}/{snapshot.entry} · {snapshot.file_count}ファイル ·{" "}
            {snapshot.byte_count.toLocaleString()} bytes
          </Typography>
          <Typography variant="caption" sx={{ overflowWrap: "anywhere" }}>
            {snapshot.revision}
          </Typography>
          <Typography variant="body2">
            表示は読み込み時点のファイルです。新しい指示を送ると閉じます。ブラウザ内の実行に対応し、サーバー処理・外部サービス・永続ストレージは利用できません。
          </Typography>
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
