import { useEffect, useRef, useState } from "react";
import { Button, MenuItem, Select } from "@mui/material";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import type { Declaration } from "@meden/workspace-schema";
export function SourceView({
  files,
  target,
  onBack,
  onNotify,
  showBack = true,
}: {
  showBack?: boolean;
  files: Readonly<Record<string, string>>;
  target?: Declaration;
  onBack: () => void;
  onNotify: (text: string) => void;
}) {
  const [file, setFile] = useState(
    target?.source?.file ||
      ("kernel.rs" in files ? "kernel.rs" : Object.keys(files)[0]) ||
      "",
  );
  const selected = useRef<HTMLSpanElement>(null);
  const viewport = useRef<HTMLPreElement>(null);
  const [tokens, setTokens] =
    useState<{ content: string; color?: string }[][]>();
  useEffect(() => {
    if (target?.source?.file) setFile(target.source.file);
  }, [target]);
  useEffect(() => {
    const row = selected.current,
      pane = viewport.current;
    if (row && pane)
      pane.scrollTop =
        row.offsetTop - pane.offsetTop - pane.clientHeight * 0.35;
  }, [file, target]);
  const source = files[file];
  useEffect(() => {
    let cancelled = false;
    setTokens(undefined);
    if (source !== undefined)
      void import("./highlight")
        .then((module) => module.highlightSource(source, file))
        .then((lines) => {
          if (!cancelled) setTokens(lines);
        })
        .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [file, source]);
  if (source === undefined)
    return <div className="panel-empty">This file is not saved: {file}</div>;
  return (
    <section className="source-view" aria-label="Source code">
      <div className="source-toolbar">
        <Select
          size="small"
          value={file}
          onChange={(event) => setFile(event.target.value)}
          inputProps={{ "aria-label": "Displayed file" }}
        >
          {Object.keys(files).map((name) => (
            <MenuItem key={name} value={name}>
              {name}
            </MenuItem>
          ))}
        </Select>
        <Button
          size="small"
          onClick={() =>
            navigator.clipboard
              .writeText(source)
              .then(() => onNotify("Copied"))
              .catch(() => onNotify("Copy failed"))
          }
        >
          Copy
        </Button>
      </div>
      {target && showBack && (
        <div className="source-location">
          <Button startIcon={<ArrowBackRounded />} onClick={onBack}>
            Back to proof
          </Button>
          <code>
            {target.source?.file}:{target.source?.line}
          </code>
        </div>
      )}
      <pre
        ref={viewport}
        className="source-code numbered-source"
        tabIndex={0}
        aria-label="File contents"
      >
        {source.split("\n").map((line, i) => {
          const active =
            file === target?.source?.file && i + 1 === target.source.line;
          return (
            <span
              key={i}
              className={
                "source-line " + (active ? "selected-source-line" : "")
              }
              ref={active ? selected : undefined}
              aria-current={active ? "location" : undefined}
            >
              <span className="source-line-number" aria-hidden="true">
                {i + 1}
              </span>
              <code>
                {tokens?.[i]?.length
                  ? tokens[i].map((token, index) => (
                      <span
                        className="syntax-token"
                        key={index}
                        style={{ color: token.color }}
                      >
                        {token.content}
                      </span>
                    ))
                  : line || " "}
              </code>
            </span>
          );
        })}
      </pre>
    </section>
  );
}
