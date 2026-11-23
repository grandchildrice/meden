// Generated from meden_app::view::SourcePage. Do not edit.
import { z } from "zod";
export const sourcePageSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    files: z.record(z.string(), z.string().max(1048576)),
    roots: z.array(z.string()),
    graph: z
      .union([
        z
          .object({
            revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            declarations: z
              .array(
                z
                  .object({
                    id: z.string().min(1).max(8192),
                    name: z.string().min(1).max(1000),
                    label: z.string().max(1000),
                    kind: z.union([
                      z.literal("theorem"),
                      z.literal("definition"),
                      z.literal("axiom"),
                    ]),
                    isMainTheorem: z.boolean(),
                    statement: z.string().max(100000),
                    source: z
                      .object({
                        file: z.string().min(1).max(4096),
                        line: z
                          .union([z.number().int().min(1), z.null()])
                          .optional(),
                      })
                      .strict(),
                    proofStatus: z.union([
                      z.literal("unknown"),
                      z.literal("unproved"),
                    ]),
                    axioms: z.array(z.string()).max(1000),
                    semantic: z
                      .union([
                        z
                          .object({
                            version: z.number().int().min(0).max(255),
                            origin: z.literal("elaborated"),
                            sentence: z.string(),
                            refutation: z.string(),
                            checker: z
                              .object({
                                engine: z.string(),
                                status: z.enum([
                                  "counterexample",
                                  "predicate_false_example",
                                  "not_found",
                                  "gave_up",
                                  "budget_exhausted",
                                  "unavailable",
                                  "unsupported",
                                  "not_proposition",
                                ]),
                                candidates: z.array(z.string()),
                                diagnostic: z
                                  .union([z.string(), z.null()])
                                  .optional(),
                                seed: z
                                  .union([z.number().int().min(0), z.null()])
                                  .optional(),
                                numInst: z
                                  .union([z.number().int().min(0), z.null()])
                                  .optional(),
                                numRetries: z
                                  .union([z.number().int().min(0), z.null()])
                                  .optional(),
                                maxSize: z
                                  .union([z.number().int().min(0), z.null()])
                                  .optional(),
                                mode: z
                                  .union([z.string(), z.null()])
                                  .optional(),
                              })
                              .strict(),
                            intermediate: z
                              .union([z.string(), z.null()])
                              .optional(),
                            renderer: z
                              .union([z.string(), z.null()])
                              .optional(),
                            rendering_error: z
                              .union([z.string(), z.null()])
                              .optional(),
                          })
                          .strict(),
                        z.null(),
                      ])
                      .optional(),
                  })
                  .strict(),
              )
              .max(5000),
            edges: z
              .array(
                z
                  .object({
                    source: z.string(),
                    target: z.string(),
                    site: z.union([
                      z.literal("type"),
                      z.literal("value"),
                      z.literal("unknown"),
                    ]),
                  })
                  .strict(),
              )
              .max(100000),
          })
          .strict(),
        z.null(),
      ])
      .optional(),
    description: z.string(),
    external: z.array(z.string()),
    compiled: z.boolean(),
    inputs_revision: z
      .union([z.string().regex(new RegExp("^[a-f0-9]{64}$")), z.null()])
      .optional(),
  })
  .strict();
export type SourcePage = z.infer<typeof sourcePageSchema>;
