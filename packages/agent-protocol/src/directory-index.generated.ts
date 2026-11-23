// Generated from meden_app::view::directory::Index. Do not edit.
import { z } from "zod";
export const directoryIndexSchema = z
  .object({
    paths: z.record(
      z.string(),
      z
        .object({
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
    ),
  })
  .strict();
export type DirectoryIndex = z.infer<typeof directoryIndexSchema>;
