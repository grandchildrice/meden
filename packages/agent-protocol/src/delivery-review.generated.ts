// Generated from meden_app::view::delivery::Review. Do not edit.
import { z } from "zod";
export const exportReviewSchema = z
  .object({
    serial: z
      .string()
      .max(20)
      .regex(new RegExp("^[1-9][0-9]{0,19}$"))
      .refine(
        (value) =>
          /^[1-9][0-9]{0,19}$/.test(value) &&
          BigInt(value) <= 18446744073709551615n,
        { message: "Expected a canonical positive u64 decimal string" },
      ),
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
    offset: z.number().int().min(0).max(16777216),
    body_hex: z.string().max(4096).regex(new RegExp("^(?:[0-9a-f]{2})*$")),
  })
  .strict();
export type ExportReview = z.infer<typeof exportReviewSchema>;
