// Generated from meden_app::view::analyses::Status. Do not edit.
import { z } from "zod";
export const analysesStatusSchema = z
  .object({
    current: z.discriminatedUnion("state", [
      z.object({ state: z.literal("empty") }).strict(),
      z
        .object({
          state: z.literal("ready"),
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
      z
        .object({
          state: z.literal("unavailable"),
          message: z.string().max(8000),
        })
        .strict(),
    ]),
    history: z.discriminatedUnion("state", [
      z
        .object({
          state: z.literal("ready"),
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
          runs: z.number().int().min(0).max(1000),
        })
        .strict(),
      z
        .object({
          state: z.literal("unavailable"),
          message: z.string().max(8000),
        })
        .strict(),
    ]),
  })
  .strict();
export type AnalysesStatus = z.infer<typeof analysesStatusSchema>;
