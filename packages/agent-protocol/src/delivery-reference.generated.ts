// Generated from meden_app::view::delivery::Reference. Do not edit.
import { z } from "zod";
export const exportReferenceSchema = z
  .object({
    session: z.string().min(1).max(8192),
    id: z.number().int().min(0).max(7),
  })
  .strict();
export type ExportReference = z.infer<typeof exportReferenceSchema>;
