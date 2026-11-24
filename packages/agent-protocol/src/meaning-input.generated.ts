// Generated from meden_app::semantic::Input. Do not edit.
import { z } from "zod";
export const meaningInputSchema = z
  .object({
    source_revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    graph_revision: z
      .union([z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")), z.null()])
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
  .strict();
export type MeaningInput = z.infer<typeof meaningInputSchema>;
