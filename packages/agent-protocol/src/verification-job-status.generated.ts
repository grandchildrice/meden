// Generated from meden_app::view::verification_jobs::Status. Do not edit.
import { z } from "zod";
export const verificationJobStatusSchema = z.union([
  z.object({ state: z.literal("idle") }).strict(),
  z
    .object({
      state: z.literal("ready"),
      job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
      revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    })
    .strict(),
  z
    .object({ state: z.literal("unavailable"), message: z.string().max(8000) })
    .strict(),
]);
export type VerificationJobStatus = z.infer<typeof verificationJobStatusSchema>;
