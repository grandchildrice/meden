import { z } from "zod";
import { atlasGraphSchema } from "@meden/workspace-schema";

export const nativeReferenceSchema = z
  .object({
    revision: z.number().int().nonnegative(),
    session: z.string(),
    contract: z.string(),
  })
  .strict();
export type NativeReference = z.infer<typeof nativeReferenceSchema>;
export const sourceSnapshotSchema = z
  .object({
    revision: z.string().regex(/^sha256:[a-f0-9]{64}$/),
    files: z.record(z.string(), z.string().max(65_536)),
    collection: z
      .object({
        roots: z.array(z.string()),
        external_modules: z.array(z.string()),
      })
      .strict()
      .optional(),
    atlas: z
      .object({
        revision: z.string().regex(/^sha256:[a-f0-9]{64}$/),
        upstream: z.string(),
        resources_sha256: z.string(),
        graph: atlasGraphSchema,
      })
      .strict()
      .optional(),
  })
  .strict();
export type SourceSnapshot = z.infer<typeof sourceSnapshotSchema>;
export const meaningInputSchema = z
  .object({
    source_revision: z.string().regex(/^sha256:[a-f0-9]{64}$/),
    graph_revision: z
      .string()
      .regex(/^sha256:[a-f0-9]{64}$/)
      .optional(),
    target: z
      .object({
        file: z.string().max(4096),
        line: z.number().int().positive(),
        name: z.string().max(1000),
      })
      .strict(),
    requirement: z.string().min(1).max(16_384),
  })
  .strict();
export type MeaningInput = z.infer<typeof meaningInputSchema>;
export const meaningRecordSchema = z
  .object({
    version: z.literal(2),
    sequence: z.number().int().positive(),
    id: z.string().regex(/^analysis-[a-f0-9]{64}$/),
    requirement_id: z.string().regex(/^requirement-[a-f0-9]{64}$/),
    source_artifact: z
      .string()
      .regex(/^sha256:[a-f0-9]{64}$/)
      .nullable(),
    finding_id: z
      .string()
      .regex(/^finding-[a-f0-9]{64}$/)
      .nullable(),
    engine: z.literal("meden-claimcheck-roundtrip-v1"),
    model: z.string(),
    processor: z.string(),
    input: meaningInputSchema,
    state: z.enum([
      "observing",
      "translating",
      "comparing",
      "complete",
      "failed",
      "cancelled",
      "stale",
      "historical",
    ]),
    error: z.string().nullable(),
    assessment: z
      .object({
        verdict: z.enum(["match", "mismatch", "unclear"]),
        translation: z
          .object({
            lemmaName: z.string(),
            naturalLanguage: z.string(),
            preconditions: z.string(),
            postcondition: z.string(),
            scope: z.string(),
            strength: z.enum(["trivial", "weak", "moderate", "strong"]),
            confidence: z.number().min(0).max(1),
          })
          .strict(),
        comparison: z
          .object({
            lemmaName: z.string(),
            match: z.boolean(),
            discrepancy: z.string(),
            weakeningType: z.enum([
              "none",
              "tautology",
              "weakened-postcondition",
              "narrowed-scope",
              "missing-case",
              "wrong-property",
            ]),
            explanation: z.string(),
          })
          .strict(),
      })
      .strict()
      .nullable(),
  })
  .strict();
export type MeaningRecord = z.infer<typeof meaningRecordSchema>;
export const repairReferenceSchema = z
  .object({
    id: z.string().regex(/^repair-[a-f0-9]{64}$/),
    sha256: z.string().regex(/^sha256:[a-f0-9]{64}$/),
  })
  .strict();
export const repairReviewSchema = z
  .object({
    reference: repairReferenceSchema,
    model: z.string(),
    processor: z.string(),
    finding_id: z.string(),
    requirement_id: z.string(),
    input: meaningInputSchema,
    file: z.string(),
    accepted_reference: nativeReferenceSchema.nullable(),
    explanation: z.string(),
    diff: z.string(),
  })
  .strict();
export type RepairReview = z.infer<typeof repairReviewSchema>;
export type RepairReference = z.infer<typeof repairReferenceSchema>;

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
  workspace: z.string(),
  endpoint: z.string(),
  model: z.string(),
  authorized: z.boolean(),
  approval_available: z.boolean(),
  running: z.boolean(),
  activity: z.string(),
  partial: z.string(),
  reasoning: z.string(),
  stopped: z.string().nullable(),
  messages: z.array(
    z.object({ speaker: z.enum(["user", "assistant"]), text: z.string() }),
  ),
  project: z
    .object({
      reference: nativeReferenceSchema,
      accepted: z.boolean(),
      proposed: z.boolean(),
      verified: z.boolean(),
      review: z.string(),
      source_path: z.string(),
    })
    .passthrough(),
  logs: z.array(
    z.object({
      type: z.enum(["call", "result"]),
      id: z.string(),
      tool: z.string(),
      text: z.string(),
      error: z.boolean().optional(),
    }),
  ),
  inspection: z
    .object({
      path: z.string(),
      directory: z.boolean().optional(),
      text: z.string().optional(),
      error: z.string().optional(),
    })
    .nullable(),
  source_snapshot: sourceSnapshotSchema.nullable(),
  analysis: meaningRecordSchema.nullable(),
  analysis_history: z.array(meaningRecordSchema).max(1000),
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
  | { type: "approve_processor"; endpoint: string }
  | { type: "revoke_processor" | "cancel" | "refresh_status" }
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

/** Same-origin native adapter. Tokens never enter workspace/history exports. */
export class NativeClient {
  private readonly token: string;
  constructor(token: string) {
    this.token = token;
  }
  async state(signal?: AbortSignal): Promise<NativeState> {
    return nativeStateSchema.parse(
      await this.request("/api/state", undefined, signal),
    );
  }
  async command(command: NativeCommand): Promise<void> {
    await this.request("/api/command", command);
  }
  private async request(
    path: string,
    command?: NativeCommand,
    signal?: AbortSignal,
  ): Promise<unknown> {
    const response = await fetch(path, {
      method: command ? "POST" : "GET",
      signal,
      headers: {
        Authorization: `Bearer ${this.token}`,
        ...(command ? { "Content-Type": "application/json" } : {}),
      },
      body: command ? JSON.stringify(command) : undefined,
      credentials: "omit",
      cache: "no-store",
    });
    const value: unknown = await response.json();
    if (!response.ok)
      throw new Error(
        typeof value === "object" && value && "error" in value
          ? String(value.error)
          : "Native session unavailable",
      );
    return value;
  }
}
