import { preparationIndexSchema } from "./preparation-index.generated";
export { preparationLogSchema } from "./preparation-log.generated";
export type { PreparationLogPublication } from "./preparation-log.generated";
import { directoryIndexSchema } from "./directory-index.generated";
export { directoryPublicationSchema } from "./directory-publication.generated";
export type { DirectoryPublication } from "./directory-publication.generated";
import { z } from "zod";
import { readerIndexSchema } from "./reader-index.generated";
export { readerPublicationSchema } from "./reader-publication.generated";
export type { ReaderPublication } from "./reader-publication.generated";
export type { FileView } from "./file.generated";

import { inventoryStatusSchema } from "./inventory-status.generated";
export { inventoryPublicationSchema } from "./inventory-publication.generated";
export { inventoryChunkSchema } from "./inventory-chunk.generated";
export type { InventoryPublication } from "./inventory-publication.generated";
export type { InventorySnapshot as WorkspaceOverview } from "./inventory.generated";

export { nativeReferenceSchema } from "./evidence-reference.generated";
import type { NativeReference } from "./evidence-reference.generated";
export type { NativeReference } from "./evidence-reference.generated";
import { evidenceStatusSchema } from "./evidence-status.generated";
export { evidencePublicationSchema } from "./evidence-publication.generated";
export type { EvidencePublication } from "./evidence-publication.generated";
export type { SourcePage as SourceSnapshot } from "./source.generated";
export { sourcePublicationSchema } from "./source-publication.generated";
export type { SourcePublication } from "./source-publication.generated";
import { sourceIndexSchema } from "./source-index.generated";
import type { MeaningInput } from "./meaning-input.generated";
export { meaningInputSchema } from "./meaning-input.generated";
export type { MeaningInput } from "./meaning-input.generated";
export { meaningRecordSchema } from "./meaning-record.generated";
export type { MeaningRecord } from "./meaning-record.generated";
import { analysesStatusSchema } from "./analyses-status.generated";
export { analysisCurrentSchema } from "./analysis-current.generated";
export type { AnalysisCurrentPublication } from "./analysis-current.generated";
export { analysisHistorySchema } from "./analysis-history.generated";
export type { AnalysisHistoryPublication } from "./analysis-history.generated";
export { repairReferenceSchema } from "./repair-reference.generated";
export type { RepairReference } from "./repair-reference.generated";
import type { RepairReference } from "./repair-reference.generated";
import { repairReviewSchema } from "./repair-review.generated";
export { repairReviewSchema } from "./repair-review.generated";
export type { RepairReview } from "./repair-review.generated";
export const previewSchema = z
  .object({
    root: z.string(),
    entry: z.string(),
    revision: z.string().regex(/^sha256:[a-f0-9]{64}$/),
    url: z.string().url(),
    file_count: z.number().int().nonnegative(),
    byte_count: z.number().int().nonnegative(),
    errors: z.array(z.string()),
  })
  .strict();
export type NativePreviewSnapshot = z.infer<typeof previewSchema>;
export { historySnapshotSchema } from "./history.generated";
import type { HistorySnapshot } from "./history.generated";
import { historyReferenceSchema } from "./history-reference.generated";
export type { HistorySnapshot } from "./history.generated";
export type HistoryData = HistorySnapshot["data"];
export type PortableWorkspace = HistoryData["portable"];
export type WorkItem = NonNullable<
  PortableWorkspace["document"]
>["tasks"][number];
export const exportReferenceSchema = z
  .object({ session: z.string(), id: z.number().int().nonnegative() })
  .strict();
export type ExportReference = z.infer<typeof exportReferenceSchema>;
export const exportSummarySchema = exportReferenceSchema
  .extend({
    source: z.string(),
    contract: z.string().nullable(),
    target: z.string(),
    bytes: z.number().int().nonnegative(),
    payload_sha256: z.string().regex(/^[a-f0-9]{64}$/),
    state: z.enum([
      "pending",
      "reviewed",
      "rejected",
      "attempted",
      "response_received",
      "delivery_uncertain",
    ]),
  })
  .strict();
export type ExportSummary = z.infer<typeof exportSummarySchema>;
export const exportReviewSchema = z
  .object({
    serial: z.number().int().positive(),
    summary: exportSummarySchema,
    offset: z.number().int().nonnegative(),
    body_hex: z
      .string()
      .regex(/^(?:[0-9a-f]{2})*$/)
      .max(4096),
  })
  .strict();
export type ExportReview = z.infer<typeof exportReviewSchema>;
export const exportOutcomeSchema = z
  .object({
    summary: exportSummarySchema,
    http_status: z.number().int().min(100).max(599).nullable(),
  })
  .strict();
export type ExportOutcome = z.infer<typeof exportOutcomeSchema>;
export const nativeStateSchema = z.object({
  source_preparation: preparationIndexSchema,
  history: historyReferenceSchema,
  workspace: z.string(),
  endpoint: z.string(),
  model: z.string(),
  provider: z.string().nullable(),
  connections: z.array(
    z
      .object({
        id: z.string(),
        label: z.string(),
        model: z.string(),
        note: z.string(),
      })
      .strict(),
  ),
  connection_revision: z.number().int().nonnegative(),
  models: z.array(z.string()),
  models_error: z.string().nullable(),
  models_provider: z.string().nullable(),
  overview: inventoryStatusSchema,
  judge: z
    .object({ model: z.string(), endpoint: z.string() })
    .strict()
    .nullable()
    .optional(),
  authorized: z.boolean(),
  approval_available: z.boolean(),
  running: z.boolean(),
  operation: z.enum([
    "conversation",
    "inspection",
    "analysis",
    "preview",
    "workspace",
  ]),
  chat_stopped: z.string().nullable(),
  activity: z.string(),
  partial: z.string(),
  reasoning: z.string(),
  stopped: z.string().nullable(),
  project: evidenceStatusSchema,
  directories: directoryIndexSchema,
  readers: readerIndexSchema,
  sources: sourceIndexSchema,
  analyses: analysesStatusSchema,
  repair_review: repairReviewSchema.nullable(),
  preview: previewSchema.nullable(),
  exports: z.array(exportSummarySchema),
  export_review: exportReviewSchema.nullable(),
  export_outcome: exportOutcomeSchema.nullable(),
  atlas: z
    .object({
      available: z.boolean(),
      prepared: z.boolean(),
      detail: z.string(),
    })
    .strict(),
});
export type NativeState = z.infer<typeof nativeStateSchema>;
export type NativeCommand =
  | { type: "initialize_work"; name: string }
  | { type: "put_work"; work: WorkItem }
  | { type: "note_work"; id: string; text: string; next: string | null }
  | { type: "check_work" | "resume_work"; id: string }
  | { type: "save_work" }
  | {
      type: "approve_processor";
      connection_revision: number;
      endpoint: string;
      model: string;
      judge?: { model: string; endpoint: string } | null;
    }
  | { type: "revoke_processor" | "cancel" | "refresh_status" }
  | { type: "refresh_workspace" }
  | { type: "refresh_connections" }
  | { type: "select_connection"; provider: string; model: string }
  | { type: "list_models"; provider: string }
  | { type: "instruction"; text: string }
  | { type: "source_snapshot"; paths: string[] }
  | { type: "prepare_atlas" }
  | { type: "atlas_snapshot"; paths: string[] }
  | { type: "capture_preview"; root: string; entry: string }
  | { type: "review_export"; reference: ExportReference; offset: number }
  | {
      type: "send_export" | "reject_export" | "close_export";
      reference: ExportReference;
      serial: number;
    }
  | { type: "prepare_repair"; finding_id: string }
  | { type: "apply_repair" | "reject_repair"; reference: RepairReference }
  | { type: "select_analysis"; id: string }
  | { type: "analyze_meaning"; input: MeaningInput }
  | {
      type: "review_contract" | "accept_contract" | "reject_contract";
      reference: NativeReference;
    }
  | { type: "inspect"; path: string; directory: boolean };
