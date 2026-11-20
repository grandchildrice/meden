import { useEffect, useRef, useState } from "react";
import { Alert, Button, CircularProgress, Typography } from "@mui/material";
import FolderOutlined from "@mui/icons-material/FolderOutlined";
import DescriptionOutlined from "@mui/icons-material/DescriptionOutlined";
import ChevronRight from "@mui/icons-material/ChevronRight";
import ExpandMore from "@mui/icons-material/ExpandMore";
import type { NativeCommand, NativeState } from "@meden/agent-protocol/native";
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
  const directoryVersion =
    state.overview.revision + ":" + state.messages.length;
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
        return <li key={name}>このフォルダの一部は表示範囲外です。</li>;
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
            aria-label={`${directory ? "フォルダ" : "ファイル"}: ${path}`}
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
    <section className="explorer-tree" aria-label="ファイルエクスプローラー">
      <div className="explorer-title">
        <Typography variant="caption">エクスプローラー</Typography>
        <Button
          disabled={running}
          onClick={() =>
            void onCommand({ type: "inspect", path: ".", directory: true })
          }
        >
          更新
        </Button>
      </div>
      <ul role="tree" aria-label="フォルダとファイル">
        {tree(".", 0)}
      </ul>
    </section>
  );
}

export function NativeCode({
  state,
  path,
  line,
  running,
  onCommand,
}: {
  state: NativeState;
  path?: string;
  line?: number;
  running: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
}) {
  const [files, setFiles] = useState<
    Record<string, NonNullable<NativeState["inspection"]>>
  >({});
  const selected = useRef<HTMLSpanElement>(null);
  const inspection = state.inspection;
  useEffect(() => {
    if (!inspection || inspection.directory) return;
    setFiles((previous) => {
      const next = { ...previous };
      if (inspection.error) delete next[inspection.path];
      else next[inspection.path] = inspection;
      return next;
    });
  }, [JSON.stringify(inspection)]);
  const current = path ? files[path] : undefined;
  useEffect(() => {
    selected.current?.scrollIntoView({ block: "center", inline: "nearest" });
  }, [path, line, current?.text]);
  if (!path)
    return (
      <p className="explorer-empty">左のファイルを選んでコードを開きます。</p>
    );
  if (inspection?.path === path && inspection.error)
    return (
      <Alert severity="error">
        {inspection.error}
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
          再試行
        </Button>
      </Alert>
    );
  return (
    <section className="native-code-editor" aria-label="ソースコード">
      {current ? (
        <>
          <pre aria-label="ファイルの内容" tabIndex={0}>
            {decoded(current.text || "")
              .split("\n")
              .map((text, i) => {
                const number = Number(text.match(/^(\d+): /)?.[1]);
                return (
                  <span
                    key={i}
                    className={`editor-line ${number === line ? "selected-source-line" : ""}`}
                    aria-current={number === line ? "location" : undefined}
                    ref={number === line ? selected : undefined}
                  >
                    {text + "\n"}
                  </span>
                );
              })}
          </pre>
          <div className="explorer-pages">
            <Button
              disabled={running || (current.offset || 1) <= 1}
              onClick={() =>
                void onCommand({
                  type: "inspect",
                  path,
                  directory: false,
                  offset: Math.max(1, (current.offset || 1) - 200),
                })
              }
            >
              前のページ
            </Button>
            <Button
              disabled={running || !current.next_offset}
              onClick={() =>
                void onCommand({
                  type: "inspect",
                  path,
                  directory: false,
                  offset: current.next_offset!,
                })
              }
            >
              次のページ
            </Button>
          </div>
        </>
      ) : (
        <div className="loading">
          <CircularProgress size={22} aria-label="ファイルを読み込み中" />
        </div>
      )}
    </section>
  );
}
