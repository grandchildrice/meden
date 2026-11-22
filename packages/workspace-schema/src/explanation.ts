import { z } from "zod";

/** A build/source observation, never a proof or review-completion receipt. */
export const explanationSchema = z
  .object({
    version: z.literal(1),
    origin: z.literal("elaborated"),
    sentence: z.string().min(1).max(65536),
    refutation: z.string().max(65536),
    intermediate: z.string().max(65536).optional(),
    renderer: z.string().max(256).optional(),
    rendering_error: z.string().max(65536).optional(),
    checker: z
      .object({
        engine: z.string().min(1).max(128),
        status: z.enum([
          "not_proposition",
          "unsupported",
          "counterexample",
          "predicate_false_example",
          "not_found",
          "gave_up",
          "budget_exhausted",
          "unavailable",
        ]),
        candidates: z.array(z.string().max(8192)).max(64),
        diagnostic: z.string().max(65536).optional(),
        seed: z.number().int().nonnegative().optional(),
        numInst: z.number().int().nonnegative().optional(),
        numRetries: z.number().int().nonnegative().optional(),
        maxSize: z.number().int().nonnegative().optional(),
        mode: z.literal("predicate").optional(),
      })
      .strict(),
  })
  .strict();
export type Explanation = z.infer<typeof explanationSchema>;
