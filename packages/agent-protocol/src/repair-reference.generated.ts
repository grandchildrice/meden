// Generated from meden_app::analysis::repair::Reference. Do not edit.
import { z } from "zod";
export const repairReferenceSchema = z
  .object({
    id: z.string().regex(new RegExp("^repair-[a-f0-9]{64}$")),
    sha256: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
  })
  .strict();
export type RepairReference = z.infer<typeof repairReferenceSchema>;
