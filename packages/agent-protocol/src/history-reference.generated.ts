// Generated from meden_app::view::history::Reference. Do not edit.
import { z } from "zod";
export const historyReferenceSchema = z.discriminatedUnion("state", [
  z
    .object({
      state: z.literal("ready"),
      revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
      messages: z.number().int().min(0),
    })
    .strict(),
  z.object({ state: z.literal("unavailable"), message: z.string() }).strict(),
]);
export type HistoryReference = z.infer<typeof historyReferenceSchema>;
