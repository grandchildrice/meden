import { preparationIndexSchema } from "./preparation-index.generated";
export { preparationLogSchema } from "./preparation-log.generated";
export type { PreparationLogPublication } from "./preparation-log.generated";
import { directoryIndexSchema } from "./directory-index.generated";
export { directoryPublicationSchema } from "./directory-publication.generated";
export type { DirectoryPublication } from "./directory-publication.generated";
import { z } from "zod";
import type { NativeSequence } from "./sequence.generated";
export { nativeSequenceSchema } from "./sequence.generated";
export type { NativeSequence } from "./sequence.generated";
export { exportReferenceSchema } from "./delivery-reference.generated";
import type { ExportReference } from "./delivery-reference.generated";
export type { ExportReference } from "./delivery-reference.generated";
import { exportSummarySchema } from "./delivery-summary.generated";
export { exportSummarySchema } from "./delivery-summary.generated";
export type { ExportSummary } from "./delivery-summary.generated";
import { exportReviewSchema } from "./delivery-review.generated";
export { exportReviewSchema } from "./delivery-review.generated";
export type { ExportReview } from "./delivery-review.generated";
import { exportOutcomeSchema } from "./delivery-outcome.generated";
export { exportOutcomeSchema } from "./delivery-outcome.generated";
export type { ExportOutcome } from "./delivery-outcome.generated";
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
import { verificationJobStatusSchema } from "./verification-job-status.generated";
export { verificationJobPublicationSchema } from "./verification-job-publication.generated";
export type { VerificationJobPublication } from "./verification-job-publication.generated";
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
import { previewStatusSchema } from "./preview-status.generated";
export { previewPublicationSchema } from "./preview-publication.generated";
export type { PreviewPublication } from "./preview-publication.generated";
export { historySnapshotSchema } from "./history.generated";
import type { HistorySnapshot } from "./history.generated";
import { historyReferenceSchema } from "./history-reference.generated";
export type { HistorySnapshot } from "./history.generated";
export type HistoryData = HistorySnapshot["data"];
export type PortableWorkspace = HistoryData["portable"];
export type WorkItem = NonNullable<
  PortableWorkspace["document"]
>["tasks"][number];
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
  verification_job: verificationJobStatusSchema,
  directories: directoryIndexSchema,
  readers: readerIndexSchema,
  sources: sourceIndexSchema,
  analyses: analysesStatusSchema,
  repair_review: repairReviewSchema.nullable(),
  preview: previewStatusSchema,
  exports: z.array(exportSummarySchema).max(8),
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
      serial: NativeSequence;
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
