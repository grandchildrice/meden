// Generated from meden_app::view::publication::Envelope<inventory::Snapshot>. Do not edit.
import { z } from "zod";
export const inventoryPublicationSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z
      .object({
        generation: z.string().regex(new RegExp("^(0|[1-9][0-9]{0,19})$")),
        files: z
          .array(
            z
              .object({
                path: z.string().min(1).max(4096),
                sha256: z.string().regex(new RegExp("^(?:[a-f0-9]{64})?$")),
                bytes: z.string().regex(new RegExp("^(0|[1-9][0-9]{0,19})$")),
                declarations: z.number().int().min(0).max(1048576),
                theorems: z.number().int().min(0).max(1048576),
                imports: z.array(z.string()),
                symbols: z.array(
                  z
                    .object({
                      name: z.string().min(1).max(1048576),
                      line: z.number().int().min(1).max(1048577),
                      kind: z.union([
                        z.literal("theorem"),
                        z.literal("definition"),
                        z.literal("axiom"),
                      ]),
                      declaration: z.string(),
                      private: z.boolean(),
                      references: z.array(z.string()),
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
                      main_theorem: z.boolean(),
                    })
                    .strict(),
                ),
                indexed: z.boolean(),
              })
              .strict(),
          )
          .max(250000),
        excluded: z.array(z.string()),
        inventory_only: z.boolean(),
      })
      .strict(),
  })
  .strict();
export type InventoryPublication = z.infer<typeof inventoryPublicationSchema>;
