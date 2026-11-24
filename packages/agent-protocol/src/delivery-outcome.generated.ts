// Generated from meden_app::view::delivery::Outcome. Do not edit.
import { z } from "zod";
export const exportOutcomeSchema = z
  .object({
    summary: z
      .object({
        session: z.string().min(1).max(8192),
        id: z.number().int().min(0).max(7),
        source: z.string().max(8192),
        contract: z.union([
          z.string().regex(new RegExp("^[a-f0-9]{64}$")),
          z.null(),
        ]),
        target: z.string().min(1).max(8192),
        bytes: z.number().int().min(0).max(16777216),
        payload_sha256: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
        state: z.union([
          z.literal("pending"),
          z.literal("reviewed"),
          z.literal("rejected"),
          z.literal("attempted"),
          z.literal("response_received"),
          z.literal("delivery_uncertain"),
        ]),
      })
      .strict(),
    http_status: z.union([z.number().int().min(100).max(999), z.null()]),
  })
  .strict();
export type ExportOutcome = z.infer<typeof exportOutcomeSchema>;
