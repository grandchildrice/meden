import { useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Button,
  IconButton,
  Tooltip,
  CircularProgress,
  Typography,
} from "@mui/material";
import Refresh from "@mui/icons-material/Refresh";
import {
  ExplanationHint,
  type SourceExplanation,
} from "../source/ExplanationHint";
import FolderOutlined from "@mui/icons-material/FolderOutlined";
import DescriptionOutlined from "@mui/icons-material/DescriptionOutlined";
import ChevronRight from "@mui/icons-material/ChevronRight";
import ExpandMore from "@mui/icons-material/ExpandMore";
import type {
  NativeCommand,
  NativeState,
  FileView,
} from "@meden/agent-protocol/native";
const decoded = (text: string) => {
  try {
    const value: unknown = JSON.parse(text);
    return typeof value === "string" ? value : JSON.stringify(value, null, 2);
  } catch {
    return text;
  }
};

export function NativeExplorer({
  state,
  running,
  onCommand,
  onOpen,
  active,
}: {
  state: NativeState;
  running: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  onOpen: (path: string) => void;
  active?: string;
}) {
  const [directories, setDirectories] = useState<Record<string, string[]>>({});
  const [expanded, setExpanded] = useState(new Set(["."]));
  const loadedRevision = useRef<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  const directoryVersion = JSON.stringify(state.history) + ":" + refresh;
  const inspection = state.inspection;
  useEffect(() => {
    if (inspection?.directory && !inspection.error)
      setDirectories((previous) => ({
        ...previous,
        [inspection.path]: decoded(inspection.text || "")
          .split("\n")
          .filter(Boolean),
      }));
  }, [JSON.stringify(inspection)]);
  useEffect(() => {
    if (
      !running &&
      state.overview.revision &&
      loadedRevision.current !== directoryVersion
    ) {
      loadedRevision.current = directoryVersion;
      void onCommand({ type: "inspect", path: ".", directory: true });
    }
  }, [running, state.overview.revision, directoryVersion]);
  const tree = (parent: string, depth: number): React.ReactNode =>
    (directories[parent] || []).map((entry) => {
      const directory = entry.endsWith("/"),
        name = directory ? entry.slice(0, -1) : entry,
        path = parent === "." ? name : parent + "/" + name,
        open = expanded.has(path);
      if (name === "[more entries omitted]")
        return (
          <li key={name}>
            Some entries in this folder are outside the display limit.
          </li>
        );
      return (
        <li
          key={path}
          role="treeitem"
          aria-expanded={directory ? open : undefined}
          aria-selected={!directory ? path === active : undefined}
        >
          <button
            className={`explorer-entry ${path === active ? "selected" : ""}`}
            style={{ paddingLeft: 8 + depth * 14 }}
            disabled={running}
            title={path}
            aria-label={`${directory ? "Folder" : "File"}: ${path}`}
            onClick={() => {
              if (!directory) {
                onOpen(path);
                return;
              }
              setExpanded((previous) => {
                const next = new Set(previous);
                if (open) next.delete(path);
                else next.add(path);
                return next;
              });
              if (!open)
                void onCommand({ type: "inspect", path, directory: true });
            }}
          >
            {directory ? (
              <>
                {open ? <ExpandMore /> : <ChevronRight />}
                <FolderOutlined />
              </>
            ) : (
              <DescriptionOutlined />
            )}
            <span>{name}</span>
          </button>
          {directory && open && <ul role="group">{tree(path, depth + 1)}</ul>}
        </li>
      );
    });
  return (
    <section className="explorer-tree" aria-label="File explorer">
      <div className="explorer-title">
        <Typography
          className="explorer-workspace-path"
          variant="caption"
          title={state.workspace}
        >
          {state.workspace.replace(/^\/home\/[^/]+/, "~")}
        </Typography>
        <Tooltip title="Refresh files and map">
          <span>
            <IconButton
              aria-label="Refresh files and map"
              size="small"
              disabled={running}
              onClick={() => {
                setRefresh((n) => n + 1);
                void onCommand({ type: "refresh_workspace" });
              }}
            >
              <Refresh fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      </div>
      <ul role="tree" aria-label="Folders and files">
        {tree(".", 0)}
      </ul>
    </section>
  );
}

export function NativeCode({
  source: snapshot,
  observation,
  path,
  line,
  running,
  onCommand,
  explanations = [],
  hoveredLine,
}: {
  source?: import("@meden/agent-protocol/native").SourceSnapshot;
  observation?: FileView;
  path?: string;
  line?: number;
  running: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  explanations?: SourceExplanation[];
  hoveredLine?: number;
}) {
  const selected = useRef<HTMLSpanElement>(null);
  const content =
    observation?.reading.state === "ready"
      ? observation.reading.content
      : undefined;
  const unavailable =
    observation?.reading.state === "unavailable"
      ? observation.reading
      : undefined;
  // A fresh unavailable/non-text observation wins over any older prepared page.
  const source =
    !unavailable && (!content || content.kind === "text") && path
      ? snapshot?.files[path]
      : undefined;
  const [offset, setOffset] = useState(1);
  useEffect(() => setOffset(1), [path]);
  useEffect(() => {
    if (line)
      setOffset((current) =>
        line >= current && line < current + 200
          ? current
          : Math.max(1, line - 5),
      );
  }, [path, line]);
  const current = useMemo(() => {
    if (source === undefined)
      return content?.kind === "text" ? content.text : undefined;
    const lines = source.split("\n");
    const start = Math.max(1, Math.min(offset, lines.length));
    return {
      offset: start,
      line_count: lines.length,
      next_offset: start + 200 <= lines.length ? start + 200 : null,
      text: lines.slice(start - 1, start + 199).join("\n"),
    };
  }, [source, offset, content]);
  const rows = useMemo(
    () =>
      (current?.text || "")
        .split("\n")
        .map((content, i) => ({ number: (current?.offset || 1) + i, content })),
    [current?.offset, current?.text],
  );
  const [tokens, setTokens] =
    useState<Array<Array<{ content: string; color?: string }>>>();
  useEffect(() => {
    let cancelled = false;
    setTokens(undefined);
    if (current && path)
      void import("../source/highlight")
        .then((m) =>
          m.highlightSource(rows.map((row) => row.content).join("\n"), path),
        )
        .then((value) => {
          if (!cancelled) setTokens(value);
        })
        .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [path, current?.text]);

  useEffect(() => {
    selected.current?.scrollIntoView({ block: "center", inline: "nearest" });
  }, [path, line, current?.text]);
  if (!path)
    return (
      <p className="explorer-empty">
        Choose a file on the left to open its source.
      </p>
    );
  if (unavailable)
    return (
      <Alert severity="error">
        <p>This file could not be read.</p>
        <details>
          <summary>Details</summary>
          {unavailable.message}
        </details>
        <Button
          disabled={running}
          onClick={() =>
            void onCommand({
              type: "inspect",
              path,
              directory: false,
              offset: line ? Math.max(1, line - 5) : 1,
            })
          }
        >
          Retry
        </Button>
      </Alert>
    );
  if (content && content.kind !== "text")
    return (
      <p className="file-empty" role="status">
        {content.kind === "binary"
          ? "This is a binary file."
          : content.kind === "unsupported_encoding"
            ? "This file is not supported UTF-8 text."
            : "This file exceeds the 1 MiB reader limit."}
      </p>
    );
  return (
    <section className="native-code-editor" aria-label="Source code">
      {current ? (
        <>
          <pre aria-label="File contents" tabIndex={0}>
            {rows.map((row, i) => (
              <span
                key={i}
                className={
                  "editor-line " +
                  (row.number === line || row.number === hoveredLine
                    ? "selected-source-line"
                    : "")
                }
                aria-current={row.number === line ? "location" : undefined}
                ref={row.number === line ? selected : undefined}
              >
                {row.number !== undefined && (
                  <>
                    <span className="editor-line-number" aria-hidden="true">
                      {row.number}:
                    </span>{" "}
                  </>
                )}
                <span className="editor-semantic-gutter">
                  {explanations
                    .filter((note) => note.line === row.number)
                    .map((note) => (
                      <ExplanationHint
                        key={note.name}
                        note={note}
                        active={hoveredLine === row.number}
                      />
                    ))}
                </span>
                <code>
                  {tokens?.[i]?.length
                    ? tokens[i].map((token, j) => (
                        <span
                          className="syntax-token"
                          key={j}
                          style={{ color: token.color }}
                        >
                          {token.content}
                        </span>
                      ))
                    : row.content}
                </code>
                {"\n"}
              </span>
            ))}
          </pre>
          <div className="explorer-pages">
            <Button
              disabled={
                (running && source === undefined) || (current.offset || 1) <= 1
              }
              onClick={() =>
                source !== undefined
                  ? setOffset(Math.max(1, offset - 200))
                  : void onCommand({
                      type: "read_file_page",
                      path,
                      offset: Math.max(1, (current.offset || 1) - 200),
                    })
              }
            >
              Previous page
            </Button>
            <Button
              disabled={
                (running && source === undefined) || !current.next_offset
              }
              onClick={() =>
                source !== undefined
                  ? setOffset(current.next_offset!)
                  : void onCommand({
                      type: "read_file_page",
                      path,
                      offset: current.next_offset!,
                    })
              }
            >
              Next page
            </Button>
          </div>
        </>
      ) : (
        <div className="loading">
          <CircularProgress size={22} aria-label="Reading file" />
        </div>
      )}
    </section>
  );
}
