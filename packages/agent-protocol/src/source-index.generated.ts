// Generated from meden_app::view::sources::Index. Do not edit.
import { z } from "zod";
export const sourceIndexSchema = z
  .object({
    current: z
      .union([
        z
          .object({
            revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
          })
          .strict(),
        z.null(),
      ])
      .optional(),
    files: z.record(
      z.string(),
      z
        .object({
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
    ),
  })
  .strict();
export type SourceIndex = z.infer<typeof sourceIndexSchema>;
