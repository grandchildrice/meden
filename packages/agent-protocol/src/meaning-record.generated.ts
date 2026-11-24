// Generated from meden_app::view::analyses::Record. Do not edit.
import { z } from "zod";
export const meaningRecordSchema = z
  .object({
    version: z.number().int().min(2).max(2),
    sequence: z.string().regex(new RegExp("^[1-9][0-9]{0,19}$")),
    id: z.string().regex(new RegExp("^analysis-[a-f0-9]{64}$")),
    requirement_id: z.string().regex(new RegExp("^requirement-[a-f0-9]{64}$")),
    finding_id: z.union([
      z.string().regex(new RegExp("^finding-[a-f0-9]{64}$")),
      z.null(),
    ]),
    source_artifact: z.union([
      z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
      z.null(),
    ]),
    engine: z.string(),
    model: z.string(),
    processor: z.string(),
    judge: z.union([
      z.object({ model: z.string(), endpoint: z.string() }).strict(),
      z.null(),
    ]),
    model_definition: z.unknown().optional(),
    judge_definition: z.unknown().optional(),
    input: z
      .object({
        source_revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        graph_revision: z
          .union([
            z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            z.null(),
          ])
          .optional(),
        target: z
          .object({
            file: z.string().min(1).max(4096),
            line: z.number().int().min(1).max(1048577),
            name: z.string().min(1).max(1000),
          })
          .strict(),
        requirement: z.string().min(1).max(16384),
      })
      .strict(),
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
    assessment: z.union([
      z
        .object({
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
          verdict: z.enum(["match", "mismatch", "unclear"]),
        })
        .strict(),
      z.null(),
    ]),
    partial_translation: z.union([
      z
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
      z.null(),
    ]),
    error: z.union([z.string(), z.null()]),
  })
  .strict();
export type MeaningRecord = z.infer<typeof meaningRecordSchema>;
