// Generated from meden_app::view::repairs::Data. Do not edit.
import { z } from "zod";
export const repairReviewSchema = z
  .object({
    reference: z
      .object({
        id: z.string().regex(new RegExp("^repair-[a-f0-9]{64}$")),
        sha256: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
      })
      .strict(),
    model: z.string(),
    processor: z.string(),
    model_definition: z.unknown().optional(),
    finding_id: z.string(),
    requirement_id: z.string(),
    input: z
      .object({
        source_revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        graph_revision: z
          .union([
            z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            z.null(),
          ])
          .optional(),
        target: z
          .object({
            file: z.string().min(1).max(4096),
            line: z.number().int().min(1).max(1048577),
            name: z.string().min(1).max(1000),
          })
          .strict(),
        requirement: z.string().min(1).max(16384),
      })
      .strict(),
    file: z.string(),
    accepted_reference: z.union([
      z
        .object({
          revision: z
            .string()
            .max(20)
            .regex(new RegExp("^[1-9][0-9]{0,19}$"))
            .refine(
              (value) =>
                /^[1-9][0-9]{0,19}$/.test(value) &&
                BigInt(value) <= 18446744073709551615n,
              { message: "Expected a canonical positive u64 decimal string" },
            ),
          session: z.string().min(1).max(8192),
          contract: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
        })
        .strict(),
      z.null(),
    ]),
    explanation: z.string(),
    diff: z.string(),
  })
  .strict();
export type RepairReview = z.infer<typeof repairReviewSchema>;
