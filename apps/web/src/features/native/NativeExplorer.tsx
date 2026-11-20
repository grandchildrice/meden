import { useEffect, useState } from "react";
import { Alert, Button, Typography } from "@mui/material";
import FolderOutlined from "@mui/icons-material/FolderOutlined";
import DescriptionOutlined from "@mui/icons-material/DescriptionOutlined";
import ChevronRight from "@mui/icons-material/ChevronRight";
import ExpandMore from "@mui/icons-material/ExpandMore";
import type { NativeCommand, NativeState } from "@meden/agent-protocol/native";

function decoded(text: string) {
  try {
    const value: unknown = JSON.parse(text);
    return typeof value === "string" ? value : JSON.stringify(value, null, 2);
  } catch {
    return text;
  }
}
export function NativeExplorer({
  state,
  running,
  onCommand,
  onDependencies,
}: {
  state: NativeState;
  running: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  onDependencies: (path: string) => void;
}) {
  const [directories, setDirectories] = useState<Record<string, string[]>>({});
  const [expanded, setExpanded] = useState(new Set(["."]));
  const [files, setFiles] = useState<
    Record<string, NonNullable<NativeState["inspection"]>>
  >({});
  const [active, setActive] = useState("");
  const inspection = state.inspection;
  useEffect(() => {
    if (!inspection) return;
    if (inspection.error) {
      setFiles((previous) => {
        const next = { ...previous };
        delete next[inspection.path];
        return next;
      });
      setDirectories((previous) => {
        const next = { ...previous };
        delete next[inspection.path];
        return next;
      });
      return;
    }
    if (inspection.directory)
      setDirectories((previous) => ({
        ...previous,
        [inspection.path]: decoded(inspection.text || "")
          .split("\n")
          .filter(Boolean),
      }));
    else {
      setFiles((previous) => ({ ...previous, [inspection.path]: inspection }));
      setActive(inspection.path);
    }
  }, [JSON.stringify(inspection)]);
  useEffect(() => {
    if (!running && !directories["."] && !inspection?.error)
      void onCommand({ type: "inspect", path: ".", directory: true });
  }, [running, Boolean(directories["."]), Boolean(inspection?.error)]);
  const read = (path: string, offset = 1) => {
    setActive(path);
    void onCommand({ type: "inspect", path, directory: false, offset });
  };
  const tree = (parent: string, depth: number) =>
    (directories[parent] || []).map((entry) => {
      const directory = entry.endsWith("/");
      const name = directory ? entry.slice(0, -1) : entry;
      const path = parent === "." ? name : parent + "/" + name;
      const open = expanded.has(path);
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
            onClick={() => {
              if (!directory) {
                read(path);
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
            aria-label={`${directory ? "フォルダ" : "ファイル"}: ${path}`}
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
  const current = files[active];
  return (
    <section className="native-explorer" aria-label="ファイルエクスプローラー">
      <nav className="explorer-tree" aria-label="プロジェクトのファイル">
        <div className="explorer-title">
          <Typography variant="caption">ファイル</Typography>
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
      </nav>
      <div className="explorer-editor">
        <div className="explorer-open-files" aria-label="開いているファイル">
          {Object.keys(files).map((file) => (
            <button
              key={file}
              aria-pressed={active === file}
              onClick={() => read(file)}
              disabled={running}
              title={file}
            >
              {file.split("/").at(-1)}
            </button>
          ))}
        </div>
        {inspection?.error && (
          <Alert severity="error">
            {inspection.path}: {inspection.error}
            <Button
              disabled={running}
              onClick={() =>
                void onCommand({
                  type: "inspect",
                  path: inspection.path,
                  directory: expanded.has(inspection.path),
                })
              }
            >
              再試行
            </Button>
          </Alert>
        )}
        {current ? (
          <>
            <div className="explorer-source-heading">
              <code>{active}</code>
              {/\.(lean|rs)$/.test(active) && (
                <Button
                  disabled={running}
                  onClick={() => onDependencies(active)}
                >
                  依存関係を見る
                </Button>
              )}
            </div>
            <pre aria-label="ファイルの内容">{decoded(current.text || "")}</pre>
            <div className="explorer-pages">
              <Button
                disabled={running || (current.offset || 1) <= 1}
                onClick={() =>
                  read(active, Math.max(1, (current.offset || 1) - 200))
                }
              >
                前のページ
              </Button>
              <span>{current.offset || 1}行目から表示</span>
              <Button
                disabled={running || !current.next_offset}
                onClick={() => read(active, current.next_offset!)}
              >
                次のページ
              </Button>
            </div>
          </>
        ) : (
          <p className="explorer-empty">
            左のフォルダを開き、ファイルを選んでください。
          </p>
        )}
      </div>
    </section>
  );
}
