// Generated from meden_app::view::inventory::Snapshot. Do not edit.
import { z } from "zod";
export const inventorySnapshotSchema = z
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
            headers: z.array(
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
                  main_theorem: z.boolean(),
                })
                .strict(),
            ),
            indexed: z.boolean(),
          })
          .strict(),
      )
      .max(250000),
    navigation: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    indexer: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    excluded: z.array(z.string()),
    inventory_only: z.boolean(),
  })
  .strict();
export type InventorySnapshot = z.infer<typeof inventorySnapshotSchema>;
