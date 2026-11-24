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
for (const [key, rust, symbol, type, owner = "view"] of [
  [
    "verification-job-status",
    "verification_jobs::Status",
    "verificationJobStatusSchema",
    "VerificationJobStatus",
  ],
  [
    "verification-job-publication",
    "verification_jobs::Snapshot",
    "verificationJobPublicationSchema",
    "VerificationJobPublication",
  ],
  [
    "evidence-status",
    "evidence::Status",
    "evidenceStatusSchema",
    "EvidenceStatus",
  ],
  [
    "evidence-publication",
    "evidence::Snapshot",
    "evidencePublicationSchema",
    "EvidencePublication",
  ],
  [
    "evidence-reference",
    "evidence::Reference",
    "nativeReferenceSchema",
    "NativeReference",
  ],
  ["repair-review", "repairs::Data", "repairReviewSchema", "RepairReview"],
  [
    "repair-reference",
    "repair::Reference",
    "repairReferenceSchema",
    "RepairReference",
    "analysis",
  ],
  [
    "analyses-status",
    "analyses::Status",
    "analysesStatusSchema",
    "AnalysesStatus",
  ],
  [
    "analysis-current",
    "analyses::CurrentPublication",
    "analysisCurrentSchema",
    "AnalysisCurrentPublication",
  ],
  [
    "analysis-history",
    "analyses::HistoryPublication",
    "analysisHistorySchema",
    "AnalysisHistoryPublication",
  ],
  [
    "meaning-record",
    "analyses::Record",
    "meaningRecordSchema",
    "MeaningRecord",
  ],
  ["meaning-input", "Input", "meaningInputSchema", "MeaningInput", "semantic"],
  [
    "preparation-index",
    "jobs::Index",
    "preparationIndexSchema",
    "PreparationIndex",
  ],
  [
    "preparation-log",
    "publication::Envelope<jobs::Log>",
    "preparationLogSchema",
    "PreparationLogPublication",
  ],
  [
    "directory-index",
    "directory::Index",
    "directoryIndexSchema",
    "DirectoryIndex",
  ],
  [
    "directory-publication",
    "publication::Envelope<directory::Directory>",
    "directoryPublicationSchema",
    "DirectoryPublication",
  ],
  [
    "inventory",
    "inventory::Snapshot",
    "inventorySnapshotSchema",
    "InventorySnapshot",
  ],
  [
    "inventory-status",
    "inventory::Status",
    "inventoryStatusSchema",
    "InventoryStatus",
  ],
  [
    "inventory-publication",
    "publication::Envelope<inventory::Snapshot>",
    "inventoryPublicationSchema",
    "InventoryPublication",
  ],
  [
    "inventory-chunk",
    "publication::Envelope<inventory::Chunk>",
    "inventoryChunkSchema",
    "InventoryChunk",
  ],
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
      `// Generated from meden_app::${owner}::${rust}. Do not edit.\nimport { z } from "zod";\nexport const ${symbol} = ${validator(source)};\nexport type ${type} = z.infer<typeof ${symbol}>;\n`,
      { parser: "typescript" },
    ),
  );
}
