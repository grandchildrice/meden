// Generated from meden_app::view::evidence::Status. Do not edit.
import { z } from "zod";
export const evidenceStatusSchema = z.discriminatedUnion("state", [
  z
    .object({
      state: z.literal("ready"),
      revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    })
    .strict(),
  z
    .object({ state: z.literal("unavailable"), message: z.string().max(8000) })
    .strict(),
]);
export type EvidenceStatus = z.infer<typeof evidenceStatusSchema>;
