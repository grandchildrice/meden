// Generated from meden_app::view::evidence::Snapshot. Do not edit.
import { z } from "zod";
export const evidencePublicationSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z.union([
      z.object({ state: z.literal("empty") }).strict(),
      z
        .object({
          state: z.literal("proposed"),
          contract: z
            .object({
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
              name: z.string(),
              source_path: z.string(),
              interface: z.string(),
              backend: z.string(),
              class: z.union([
                z.literal("empirical"),
                z.literal("bounded"),
                z.literal("theorem"),
                z.literal("refinement"),
              ]),
              scope: z.string(),
              canonical: z.string().max(65536),
            })
            .strict(),
          accepted: z.union([
            z
              .object({
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
                name: z.string(),
                source_path: z.string(),
                interface: z.string(),
                backend: z.string(),
                class: z.union([
                  z.literal("empirical"),
                  z.literal("bounded"),
                  z.literal("theorem"),
                  z.literal("refinement"),
                ]),
                scope: z.string(),
                canonical: z.string().max(65536),
              })
              .strict(),
            z.null(),
          ]),
          review: z.string(),
        })
        .strict(),
      z
        .object({
          state: z.literal("accepted"),
          contract: z
            .object({
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
              name: z.string(),
              source_path: z.string(),
              interface: z.string(),
              backend: z.string(),
              class: z.union([
                z.literal("empirical"),
                z.literal("bounded"),
                z.literal("theorem"),
                z.literal("refinement"),
              ]),
              scope: z.string(),
              canonical: z.string().max(65536),
            })
            .strict(),
          review: z.string(),
          evidence: z.union([
            z
              .object({
                state: z.literal("unverified"),
                retained: z.union([
                  z
                    .object({
                      directory: z.string(),
                      source_sha256: z
                        .string()
                        .regex(new RegExp("^[a-f0-9]{64}$")),
                      artifact_sha256: z
                        .string()
                        .regex(new RegExp("^[a-f0-9]{64}$")),
                    })
                    .strict(),
                  z.null(),
                ]),
                failure: z.union([
                  z
                    .object({
                      kind: z.union([
                        z.literal("unverified"),
                        z.literal("unavailable"),
                        z.literal("inconclusive"),
                      ]),
                      message: z.string(),
                    })
                    .strict(),
                  z.null(),
                ]),
              })
              .strict(),
            z
              .object({
                state: z.literal("current"),
                completion_label: z.string(),
                artifact: z
                  .object({
                    directory: z.string(),
                    source_sha256: z
                      .string()
                      .regex(new RegExp("^[a-f0-9]{64}$")),
                    artifact_sha256: z
                      .string()
                      .regex(new RegExp("^[a-f0-9]{64}$")),
                  })
                  .strict(),
              })
              .strict(),
          ]),
        })
        .strict(),
    ]),
  })
  .strict();
export type EvidencePublication = z.infer<typeof evidencePublicationSchema>;
