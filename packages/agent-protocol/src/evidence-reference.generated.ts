// Generated from meden_app::view::evidence::Reference. Do not edit.
import { z } from "zod";
export const nativeReferenceSchema = z
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
  .strict();
export type NativeReference = z.infer<typeof nativeReferenceSchema>;
