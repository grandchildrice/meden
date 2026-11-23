// Generated from meden_app::view::jobs::Index. Do not edit.
import { z } from "zod";
export const preparationIndexSchema = z.record(
  z.string(),
  z
    .object({
      job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
      resource: z.string().min(1).max(4096),
      phase: z.string().min(1).max(512),
      elapsed_ms: z.number().int().min(0),
      phase_elapsed_ms: z.number().int().min(0),
      limit_seconds: z.number().int().min(0),
      outcome: z.union([
        z.object({ state: z.literal("preparing") }).strict(),
        z
          .object({ state: z.literal("ready"), cache_hit: z.boolean() })
          .strict(),
        z
          .object({
            state: z.literal("failed"),
            kind: z.union([
              z.literal("operation"),
              z.literal("cancelled"),
              z.literal("observation"),
            ]),
            error: z.string().max(8000),
          })
          .strict(),
      ]),
      log: z
        .union([
          z
            .object({
              revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            })
            .strict(),
          z.null(),
        ])
        .optional(),
    })
    .strict(),
);
export type PreparationIndex = z.infer<typeof preparationIndexSchema>;
