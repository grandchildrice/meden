// Generated from meden_app::view::verification_jobs::Snapshot. Do not edit.
import { z } from "zod";
export const verificationJobPublicationSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z.discriminatedUnion("kind", [
      z
        .object({
          kind: z.literal("project"),
          context: z
            .object({
              job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
              subject: z.discriminatedUnion("state", [
                z
                  .object({
                    state: z.literal("proposal"),
                    session: z.string().min(1).max(256),
                    contract: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
                  })
                  .strict(),
                z
                  .object({
                    state: z.literal("accepted"),
                    reference: z
                      .object({
                        revision: z
                          .string()
                          .max(20)
                          .regex(new RegExp("^[1-9][0-9]{0,19}$"))
                          .refine(
                            (value) =>
                              /^[1-9][0-9]{0,19}$/.test(value) &&
                              BigInt(value) <= 18446744073709551615n,
                            {
                              message:
                                "Expected a canonical positive u64 decimal string",
                            },
                          ),
                        session: z.string().min(1).max(8192),
                        contract: z
                          .string()
                          .regex(new RegExp("^[a-f0-9]{64}$")),
                      })
                      .strict(),
                  })
                  .strict(),
              ]),
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
          phase: z.discriminatedUnion("state", [
            z
              .object({
                state: z.literal("proposed"),
                reference: z
                  .object({
                    revision: z
                      .string()
                      .max(20)
                      .regex(new RegExp("^[1-9][0-9]{0,19}$"))
                      .refine(
                        (value) =>
                          /^[1-9][0-9]{0,19}$/.test(value) &&
                          BigInt(value) <= 18446744073709551615n,
                        {
                          message:
                            "Expected a canonical positive u64 decimal string",
                        },
                      ),
                    session: z.string().min(1).max(8192),
                    contract: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
                  })
                  .strict(),
              })
              .strict(),
            z
              .object({
                state: z.literal("running"),
                stage: z.discriminatedUnion("state", [
                  z.object({ state: z.literal("checking_proposal") }).strict(),
                  z.object({ state: z.literal("recording_proposal") }).strict(),
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
                    source_sha256: z
                      .string()
                      .regex(new RegExp("^[a-f0-9]{64}$")),
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
                during: z.discriminatedUnion("state", [
                  z.object({ state: z.literal("checking_proposal") }).strict(),
                  z.object({ state: z.literal("recording_proposal") }).strict(),
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
                during: z.discriminatedUnion("state", [
                  z.object({ state: z.literal("checking_proposal") }).strict(),
                  z.object({ state: z.literal("recording_proposal") }).strict(),
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
      z
        .object({
          kind: z.literal("recorded"),
          context: z
            .object({
              origin: z.enum(["host", "model"]),
              job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
              session: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
              id: z.string().min(1).max(1000),
            })
            .strict(),
          elapsed_ms: z.number().int().min(0),
          phase: z.discriminatedUnion("state", [
            z
              .object({
                state: z.literal("running"),
                stage: z.discriminatedUnion("state", [
                  z.object({ state: z.literal("reading_record") }).strict(),
                  z
                    .object({
                      state: z.literal("preparing"),
                      work: z
                        .object({
                          id: z.string(),
                          title: z.string(),
                          requirement: z.string(),
                          module: z.string(),
                          theorems: z.array(z.string()),
                          next: z.string(),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("capturing"),
                      work: z
                        .object({
                          id: z.string(),
                          title: z.string(),
                          requirement: z.string(),
                          module: z.string(),
                          theorems: z.array(z.string()),
                          next: z.string(),
                        })
                        .strict(),
                      engine_sha256: z.string(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("recording_start"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("building"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("inspecting"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("refreshing"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("persisting"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                ]),
              })
              .strict(),
            z
              .object({
                state: z.literal("historical"),
                inputs: z
                  .object({
                    work: z
                      .object({
                        id: z.string(),
                        title: z.string(),
                        requirement: z.string(),
                        module: z.string(),
                        theorems: z.array(z.string()),
                        next: z.string(),
                      })
                      .strict(),
                    engine_sha256: z
                      .string()
                      .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                    source_revision: z
                      .string()
                      .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                  })
                  .strict(),
                event: z.string().regex(new RegExp("^event_[a-f0-9]{64}$")),
                record_sha256: z
                  .string()
                  .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                result: z.enum([
                  "checked",
                  "rejected",
                  "cancelled",
                  "inconclusive",
                  "unavailable",
                ]),
              })
              .strict(),
            z
              .object({
                state: z.literal("failed"),
                during: z.discriminatedUnion("state", [
                  z.object({ state: z.literal("reading_record") }).strict(),
                  z
                    .object({
                      state: z.literal("preparing"),
                      work: z
                        .object({
                          id: z.string(),
                          title: z.string(),
                          requirement: z.string(),
                          module: z.string(),
                          theorems: z.array(z.string()),
                          next: z.string(),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("capturing"),
                      work: z
                        .object({
                          id: z.string(),
                          title: z.string(),
                          requirement: z.string(),
                          module: z.string(),
                          theorems: z.array(z.string()),
                          next: z.string(),
                        })
                        .strict(),
                      engine_sha256: z.string(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("recording_start"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("building"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("inspecting"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("refreshing"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("persisting"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
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
                during: z.discriminatedUnion("state", [
                  z.object({ state: z.literal("reading_record") }).strict(),
                  z
                    .object({
                      state: z.literal("preparing"),
                      work: z
                        .object({
                          id: z.string(),
                          title: z.string(),
                          requirement: z.string(),
                          module: z.string(),
                          theorems: z.array(z.string()),
                          next: z.string(),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("capturing"),
                      work: z
                        .object({
                          id: z.string(),
                          title: z.string(),
                          requirement: z.string(),
                          module: z.string(),
                          theorems: z.array(z.string()),
                          next: z.string(),
                        })
                        .strict(),
                      engine_sha256: z.string(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("recording_start"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("building"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("inspecting"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("refreshing"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                        })
                        .strict(),
                    })
                    .strict(),
                  z
                    .object({
                      state: z.literal("persisting"),
                      inputs: z
                        .object({
                          work: z
                            .object({
                              id: z.string(),
                              title: z.string(),
                              requirement: z.string(),
                              module: z.string(),
                              theorems: z.array(z.string()),
                              next: z.string(),
                            })
                            .strict(),
                          engine_sha256: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                          source_revision: z
                            .string()
                            .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
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
    ]),
  })
  .strict();
export type VerificationJobPublication = z.infer<
  typeof verificationJobPublicationSchema
>;
