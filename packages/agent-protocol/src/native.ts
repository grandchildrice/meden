import { z } from "zod";
import { atlasGraphSchema, explanationSchema } from "@meden/workspace-schema";

const sourceSymbolSchema = z
  .object({
    name: z.string(),
    line: z.number().int().positive(),
    kind: z.enum(["definition", "theorem", "axiom"]),
    declaration: z.string(),
    private: z.boolean(),
    references: z.array(z.string()),
    semantic: explanationSchema.nullable(),
    main_theorem: z.boolean(),
  })
  .strict();

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
    files: z.record(z.string(), z.string().max(1_048_576)),
    collection: z
      .object({
        roots: z.array(z.string()),
        external_modules: z.array(z.string()),
        prepared: z
          .string()
          .regex(/^[a-f0-9]{64}$/)
          .optional(),
        modules: z.array(
          z
            .object({ name: z.string(), file: z.string(), root: z.string() })
            .strict(),
        ),
        packages: z.array(
          z
            .object({
              name: z.string(),
              directory: z.string(),
              source_roots: z.array(z.string()),
            })
            .strict(),
        ),
        metadata: z
          .object({
            files: z.record(z.string(), z.string().nullable()),
            directories: z.record(
              z.string(),
              z
                .array(
                  z.object({ name: z.string(), kind: z.string() }).strict(),
                )
                .nullable(),
            ),
          })
          .strict(),
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
const meaningTranslationSchema = z
  .object({
    lemmaName: z.string(),
    naturalLanguage: z.string(),
    preconditions: z.string(),
    postcondition: z.string(),
    scope: z.string(),
    strength: z.enum(["trivial", "weak", "moderate", "strong"]),
    confidence: z.number().min(0).max(1),
  })
  .strict();
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
    judge: z
      .object({ model: z.string(), endpoint: z.string() })
      .strict()
      .nullable()
      .optional(),
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
    partial_translation: meaningTranslationSchema.nullable().optional(),
    assessment: z
      .object({
        verdict: z.enum(["match", "mismatch", "unclear"]),
        translation: meaningTranslationSchema,
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
export const workItemSchema = z
  .object({
    id: z.string(),
    title: z.string(),
    requirement: z.string(),
    module: z.string(),
    theorems: z.array(z.string()),
    next: z.string(),
  })
  .strict();
export type WorkItem = z.infer<typeof workItemSchema>;
export const portableWorkspaceSchema = z
  .object({
    document: z
      .object({
        version: z.literal(1),
        name: z.string(),
        tasks: z.array(workItemSchema),
        events: z.array(
          z
            .object({
              id: z.string(),
              task: z.string(),
              time: z.number().int().nonnegative(),
              note: z.string(),
              check: z
                .object({
                  engine_sha256: z.string(),
                  requirement: z.string(),
                  theorems: z.array(z.string()),
                  inspection: z.string(),
                  source_revision: z.string(),
                  after_revision: z.string().nullable(),
                  module: z.string(),
                  success: z.boolean(),
                  termination: z.string(),
                  stdout: z.string(),
                  stderr: z.string(),
                })
                .strict()
                .nullable(),
            })
            .strict(),
        ),
        conversations: z.array(
          z
            .object({
              id: z.string(),
              messages: z.array(
                z
                  .object({
                    speaker: z.enum(["user", "assistant"]),
                    text: z.string(),
                  })
                  .strict(),
              ),
            })
            .strict(),
        ),
      })
      .strict()
      .nullable(),
    error: z.string().nullable(),
    lean_available: z.boolean(),
    lean: z.string(),
  })
  .strict();
export type PortableWorkspace = z.infer<typeof portableWorkspaceSchema>;
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
export const workspaceOverviewSchema = z
  .object({
    revision: z.string(),
    files: z.array(
      z
        .object({
          path: z.string(),
          sha256: z.string(),
          bytes: z.number(),
          declarations: z.number(),
          theorems: z.number(),
          indexed: z.boolean(),
          imports: z.array(z.string()),
          symbols: z.array(sourceSymbolSchema),
        })
        .strict(),
    ),
    excluded: z.array(z.string()),
    inventory_only: z.boolean(),
  })
  .strict();
export type WorkspaceOverview = z.infer<typeof workspaceOverviewSchema>;
export const nativeStateSchema = z.object({
  source_preparation: z
    .record(
      z.string(),
      z
        .object({
          state: z.enum(["preparing", "ready", "failed"]),
          cache_hit: z.boolean(),
          error: z.string().nullable(),
          phase: z.string(),
          elapsed_ms: z.number().nonnegative(),
          phase_elapsed_ms: z.number().nonnegative(),
          limit_seconds: z.number().nonnegative(),
          log: z.string(),
          log_truncated: z.boolean(),
        })
        .strict(),
    )
    .default({}),
  portable: portableWorkspaceSchema,
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
  overview: z
    .object({
      state: z.enum(["loading", "ready", "failed", "stale"]),
      scanned: z.number(),
      total: z.number(),
      revision: z.string().nullable(),
      error: z.string().nullable(),
    })
    .strict(),
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
    z.union([
      z.object({ type: z.literal("notice"), text: z.string() }).strict(),
      z.object({
        type: z.enum(["call", "result"]),
        id: z.string(),
        tool: z.string(),
        text: z.string(),
        error: z.boolean().optional(),
      }),
    ]),
  ),
  inspection: z
    .object({
      path: z.string(),
      directory: z.boolean().optional(),
      offset: z.number().int().positive().optional(),
      next_offset: z.number().int().positive().nullable().optional(),
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
  | { type: "inspect"; path: string; directory: boolean; offset?: number };

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
  async overview(signal?: AbortSignal): Promise<WorkspaceOverview> {
    return workspaceOverviewSchema.parse(
      await this.request("/api/overview", undefined, signal),
    );
  }
  async map(path: string) {
    return workspaceOverviewSchema
      .pick({ revision: true, files: true })
      .parse(await this.request("/api/map?path=" + encodeURIComponent(path)));
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
