import { writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { validator } from "./source-schema.mjs";
import { format } from "prettier";
import { z } from "zod";
import { workspaceSchema } from "@meden/workspace-schema";

writeFileSync(
  "schemas/workspace-v1.schema.json",
  JSON.stringify(z.toJSONSchema(workspaceSchema), null, 2) + "\n",
);
for (const [key, rust, symbol, type] of [
  ["source", "SourcePage", "sourcePageSchema", "SourcePage"],
  ["file", "file::FileView", "fileViewSchema", "FileView"],
  ["reader-index", "readers::Index", "readerIndexSchema", "ReaderIndex"],
  [
    "reader-publication",
    "publication::Envelope<file::FileView>",
    "readerPublicationSchema",
    "ReaderPublication",
  ],
  ["source-index", "sources::Index", "sourceIndexSchema", "SourceIndex"],
  [
    "source-publication",
    "publication::Envelope<SourcePage>",
    "sourcePublicationSchema",
    "SourcePublication",
  ],
  ["history", "history::Snapshot", "historySnapshotSchema", "HistorySnapshot"],
  [
    "history-reference",
    "history::Reference",
    "historyReferenceSchema",
    "HistoryReference",
  ],
]) {
  const source = JSON.parse(
    execFileSync(
      "cargo",
      [
        "run",
        "--quiet",
        "--locked",
        "-p",
        "meden-app",
        "--example",
        "export-source-schema",
        "--",
        key!,
      ],
      { encoding: "utf8" },
    ),
  );
  writeFileSync(
    `schemas/native-${key}.schema.json`,
    JSON.stringify(source, null, 2) + "\n",
  );
  writeFileSync(
    `packages/agent-protocol/src/${key}.generated.ts`,
    await format(
      `// Generated from meden_app::view::${rust}. Do not edit.\nimport { z } from "zod";\nexport const ${symbol} = ${validator(source)};\nexport type ${type} = z.infer<typeof ${symbol}>;\n`,
      { parser: "typescript" },
    ),
  );
}
