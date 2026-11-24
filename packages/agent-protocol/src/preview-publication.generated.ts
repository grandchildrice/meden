// Generated from meden_app::view::preview::Snapshot. Do not edit.
import { z } from "zod";
export const previewPublicationSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z
      .object({
        frame: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        root: z.string().min(1).max(4096),
        entry: z.string().min(1).max(4096),
        revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        url: z.string().min(1).max(16384).url(),
        file_count: z.number().int().min(1).max(256),
        byte_count: z.number().int().min(0).max(16777216),
        errors: z.array(z.string().max(2400)).max(20),
      })
      .strict(),
  })
  .strict();
export type PreviewPublication = z.infer<typeof previewPublicationSchema>;
