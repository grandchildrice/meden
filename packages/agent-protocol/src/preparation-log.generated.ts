// Generated from meden_app::view::publication::Envelope<jobs::Log>. Do not edit.
import { z } from "zod";
export const preparationLogSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z
      .object({
        job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        resource: z.string().min(1).max(4096),
        text: z.string().max(65536),
        truncated: z.boolean(),
      })
      .strict(),
  })
  .strict();
export type PreparationLogPublication = z.infer<typeof preparationLogSchema>;
