// Generated from meden_app::view::readers::Index. Do not edit.
import { z } from "zod";
export const readerIndexSchema = z
  .object({
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
export type ReaderIndex = z.infer<typeof readerIndexSchema>;
