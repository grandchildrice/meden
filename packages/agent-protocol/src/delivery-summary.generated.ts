// Generated from meden_app::view::delivery::Summary. Do not edit.
import { z } from "zod";
export const exportSummarySchema = z
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
  .strict();
export type ExportSummary = z.infer<typeof exportSummarySchema>;
