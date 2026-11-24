// Generated from meden_app::view::verification_jobs::Snapshot. Do not edit.
import { z } from "zod";
export const verificationJobPublicationSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z
      .object({
        context: z
          .object({
            job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            reference: z
              .object({
                revision: z.string().regex(new RegExp("^[1-9][0-9]{0,19}$")),
                session: z.string().min(1).max(8192),
                contract: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
              })
              .strict(),
            source_path: z.string().min(1).max(4096),
            name: z.string(),
            backend: z.string(),
            class: z.union([
              z.literal("empirical"),
              z.literal("bounded"),
              z.literal("theorem"),
              z.literal("refinement"),
            ]),
            scope: z.string(),
          })
          .strict(),
        elapsed_ms: z.number().int().min(0),
        phase: z.union([
          z
            .object({
              state: z.literal("running"),
              stage: z.union([
                z.object({ state: z.literal("reading_source") }).strict(),
                z
                  .object({
                    state: z.literal("preparing_checker"),
                    source_sha256: z
                      .string()
                      .regex(new RegExp("^[a-f0-9]{64}$")),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("checking"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("refreshing_source"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("retaining_evidence"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
              ]),
            })
            .strict(),
          z
            .object({
              state: z.literal("admitted"),
              inputs: z
                .object({
                  source_sha256: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
                  prepared_sha256: z
                    .string()
                    .regex(new RegExp("^[a-f0-9]{64}$")),
                })
                .strict(),
              completion_label: z.string(),
            })
            .strict(),
          z
            .object({
              state: z.literal("failed"),
              during: z.union([
                z.object({ state: z.literal("reading_source") }).strict(),
                z
                  .object({
                    state: z.literal("preparing_checker"),
                    source_sha256: z
                      .string()
                      .regex(new RegExp("^[a-f0-9]{64}$")),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("checking"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("refreshing_source"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("retaining_evidence"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
              ]),
              kind: z.union([
                z.literal("unverified"),
                z.literal("unavailable"),
                z.literal("inconclusive"),
              ]),
              message: z.string().max(8000),
            })
            .strict(),
          z
            .object({
              state: z.literal("cancelled"),
              during: z.union([
                z.object({ state: z.literal("reading_source") }).strict(),
                z
                  .object({
                    state: z.literal("preparing_checker"),
                    source_sha256: z
                      .string()
                      .regex(new RegExp("^[a-f0-9]{64}$")),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("checking"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("refreshing_source"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("retaining_evidence"),
                    inputs: z
                      .object({
                        source_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                        prepared_sha256: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
              ]),
              message: z.string().max(8000),
            })
            .strict(),
        ]),
      })
      .strict(),
  })
  .strict();
export type VerificationJobPublication = z.infer<
  typeof verificationJobPublicationSchema
>;
