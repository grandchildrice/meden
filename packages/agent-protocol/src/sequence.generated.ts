// Generated from meden_app::view::sequence::Sequence. Do not edit.
import { z } from "zod";
export const nativeSequenceSchema = z
  .string()
  .max(20)
  .regex(new RegExp("^[1-9][0-9]{0,19}$"))
  .refine(
    (value) =>
      /^[1-9][0-9]{0,19}$/.test(value) &&
      BigInt(value) <= 18446744073709551615n,
    { message: "Expected a canonical positive u64 decimal string" },
  );
export type NativeSequence = z.infer<typeof nativeSequenceSchema>;
