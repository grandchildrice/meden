// Generated from meden_app::view::inventory::Status. Do not edit.
import { z } from "zod";
export const inventoryStatusSchema = z
  .object({
    state: z.union([
      z.literal("loading"),
      z.literal("ready"),
      z.literal("stale"),
      z.literal("failed"),
    ]),
    scanned: z.number().int().min(0).max(250000),
    total: z.number().int().min(0).max(250000),
    revision: z
      .union([z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")), z.null()])
      .optional(),
    error: z.union([z.string(), z.null()]).optional(),
  })
  .strict();
export type InventoryStatus = z.infer<typeof inventoryStatusSchema>;
