import { useState } from "react";
import { Alert, Button, Stack, TextField, Typography } from "@mui/material";
import type { PreviewPublication } from "@meden/agent-protocol/native";

export function NativePreview({
  snapshot,
  error,
  loading,
  transferError,
  onRetry,
  running,
  onCapture,
}: {
  snapshot: PreviewPublication["data"] | null;
  error: string | null;
  loading?: boolean;
  transferError?: string;
  onRetry?: () => void;
  running: boolean;
  onCapture: (root: string, entry: string) => Promise<boolean>;
}) {
  const [root, setRoot] = useState(".");
  const [entry, setEntry] = useState("index.html");
  return (
    <Stack spacing={2}>
      <Typography variant="h3">App preview</Typography>
      {error && <Alert severity="error">{error}</Alert>}
      {transferError && (
        <Alert severity="warning">
          Could not load the preview description.
          <details>
            <summary>Details</summary>
            {transferError}
          </details>
          {onRetry && <Button onClick={onRetry}>Retry preview</Button>}
        </Alert>
      )}
      <TextField
        label="Public folder (inside workspace)"
        size="small"
        value={root}
        onChange={(event) => setRoot(event.target.value)}
      />
      <TextField
        label="Entry HTML file"
        size="small"
        value={entry}
        onChange={(event) => setEntry(event.target.value)}
      />
      <Button
        variant="outlined"
        disabled={running || !root.trim() || !entry.trim()}
        onClick={() => void onCapture(root, entry)}
      >
        Reload files and preview
      </Button>
      {snapshot ? (
        <>
          <details>
            <summary>Preview information</summary>
            <Typography variant="body2" sx={{ overflowWrap: "anywhere" }}>
              {snapshot.root}/{snapshot.entry} · {snapshot.file_count} files ·{" "}
              {snapshot.byte_count.toLocaleString("en-US")} bytes
            </Typography>
            <Typography variant="caption" sx={{ overflowWrap: "anywhere" }}>
              {snapshot.revision}
            </Typography>
            <Typography variant="body2">
              Displays local files captured at load time. Browser interaction is
              not proof evidence. For built apps, choose the output folder.
              Server processing, external services and persistent storage are
              unavailable.
            </Typography>
          </details>
          {snapshot.errors.length > 0 && (
            <Alert severity="warning">
              Some files could not be read. Check the output folder and asset
              references, then reload.
              <details>
                <summary>Details</summary>
                <ul>
                  {snapshot.errors.map((error, index) => (
                    <li key={index}>{error}</li>
                  ))}
                </ul>
              </details>
            </Alert>
          )}
          <iframe
            key={snapshot.url}
            title="Local app preview"
            src={snapshot.url}
            sandbox="allow-scripts allow-forms"
            referrerPolicy="no-referrer"
            className="native-preview"
          />
        </>
      ) : loading ? (
        <p role="status">Loading preview…</p>
      ) : (
        <p>Choose a folder and HTML file to load.</p>
      )}
    </Stack>
  );
}
