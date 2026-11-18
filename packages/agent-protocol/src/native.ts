import { z } from "zod";

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
  })
  .strict();
export type SourceSnapshot = z.infer<typeof sourceSnapshotSchema>;
export const meaningInputSchema = z
  .object({
    source_revision: z.string().regex(/^sha256:[a-f0-9]{64}$/),
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
    version: z.literal(1),
    engine: z.literal("meden-claimcheck-roundtrip-v1"),
    model: z.string(),
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
});
export type NativeState = z.infer<typeof nativeStateSchema>;
export type NativeCommand =
  | { type: "approve_processor"; endpoint: string }
  | { type: "revoke_processor" | "cancel" | "refresh_status" }
  | { type: "instruction"; text: string }
  | { type: "source_snapshot"; paths: string[] }
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
