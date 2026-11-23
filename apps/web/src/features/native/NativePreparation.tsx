import { useEffect, useRef } from "react";
import { Alert, Button, LinearProgress } from "@mui/material";
import type {
  NativeState,
  PreparationLogPublication,
} from "@meden/agent-protocol/native";

type Preparation = NativeState["source_preparation"][string];

function duration(milliseconds: number) {
  const seconds = Math.floor(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

export interface LogProps {
  log?: PreparationLogPublication["data"];
  error?: string;
  onRetry: () => void;
}
export function PreparationLog({ log, error, onRetry }: LogProps) {
  const pane = useRef<HTMLPreElement>(null);
  const follow = useRef(true);
  useEffect(() => {
    if (pane.current && follow.current)
      pane.current.scrollTop = pane.current.scrollHeight;
  }, [log?.text]);
  return (
    <>
      {error && (
        <Alert severity="warning">
          Could not load the build log.{" "}
          <Button onClick={onRetry}>Retry log</Button>
        </Alert>
      )}
      <pre
        ref={pane}
        className="preparation-log"
        role="log"
        aria-label="Build log"
        aria-live="off"
        tabIndex={0}
        onScroll={(event) => {
          const pane = event.currentTarget;
          follow.current =
            pane.scrollHeight - pane.scrollTop - pane.clientHeight < 32;
        }}
      >
        {log?.text || "Waiting for compiler output…"}
      </pre>
      {log?.truncated && <small>Showing the latest output.</small>}
    </>
  );
}

export function NativePreparation({
  preparation,
  path,
  onCancel,
  log,
  error,
  onRetry,
}: LogProps & {
  preparation?: Preparation;
  path: string;
  onCancel?: () => void;
}) {
  return (
    <section className="file-preparation" aria-label={"Preparing " + path}>
      <LinearProgress />
      <div className="preparation-heading">
        <strong>{preparation?.phase || "Resolving modules"}</strong>
        <span>{duration(preparation?.elapsed_ms || 0)}</span>
        {onCancel && <Button onClick={onCancel}>Stop preparation</Button>}
      </div>
      {Boolean(preparation?.limit_seconds) && (
        <small>
          Current step: {duration(preparation?.phase_elapsed_ms || 0)} /{" "}
          {duration(preparation!.limit_seconds * 1000)}
        </small>
      )}
      <PreparationLog log={log} error={error} onRetry={onRetry} />
    </section>
  );
}
