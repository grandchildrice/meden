// Generated from meden_app::view::verification_jobs::Status. Do not edit.
import { z } from "zod";
export const verificationJobStatusSchema = z.discriminatedUnion("state", [
  z.object({ state: z.literal("idle") }).strict(),
  z
    .object({
      state: z.literal("ready"),
      role: z.union([
        z.literal("project"),
        z.literal("recorded_host"),
        z.literal("recorded_model"),
      ]),
      job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
      revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    })
    .strict(),
  z
    .object({
      state: z.literal("unavailable"),
      role: z.union([
        z.union([
          z.literal("project"),
          z.literal("recorded_host"),
          z.literal("recorded_model"),
        ]),
        z.null(),
      ]),
      message: z.string().max(8000),
    })
    .strict(),
]);
export type VerificationJobStatus = z.infer<typeof verificationJobStatusSchema>;
