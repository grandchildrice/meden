// Generated from meden_app::view::session::State. Do not edit.
import { z } from "zod";
export const nativeStateSchema = z
  .object({
    history: z.discriminatedUnion("state", [
      z
        .object({
          state: z.literal("ready"),
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
          messages: z.number().int().min(0),
        })
        .strict(),
      z
        .object({ state: z.literal("unavailable"), message: z.string() })
        .strict(),
    ]),
    workspace: z.string(),
    endpoint: z.string(),
    model: z.string(),
    provider: z.union([z.string(), z.null()]),
    connections: z.array(
      z
        .object({
          id: z.string(),
          label: z.string(),
          model: z.string(),
          note: z.string(),
        })
        .strict(),
    ),
    connection_revision: z.union([
      z
        .string()
        .max(20)
        .regex(new RegExp("^[1-9][0-9]{0,19}$"))
        .refine(
          (value) =>
            /^[1-9][0-9]{0,19}$/.test(value) &&
            BigInt(value) <= 18446744073709551615n,
          { message: "Expected a canonical positive u64 decimal string" },
        ),
      z.null(),
    ]),
    models: z.array(z.string()),
    models_error: z.union([z.string(), z.null()]),
    models_provider: z.union([z.string(), z.null()]),
    overview: z
      .object({
        state: z.union([
          z.literal("loading"),
          z.literal("ready"),
          z.literal("stale"),
          z.literal("failed"),
        ]),
        scanned: z.number().int().min(0).max(250000),
        total: z.number().int().min(0).max(250000),
        revision: z.union([
          z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
          z.null(),
        ]),
        error: z.union([z.string(), z.null()]),
      })
      .strict(),
    judge: z.union([
      z.object({ model: z.string(), endpoint: z.string() }).strict(),
      z.null(),
    ]),
    authorized: z.boolean(),
    approval_available: z.boolean(),
    running: z.boolean(),
    operation: z.enum([
      "conversation",
      "inspection",
      "analysis",
      "preview",
      "workspace",
    ]),
    chat_stopped: z.union([z.string(), z.null()]),
    activity: z.string(),
    partial: z.string(),
    reasoning: z.string(),
    project: z.discriminatedUnion("state", [
      z
        .object({
          state: z.literal("ready"),
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
      z
        .object({
          state: z.literal("unavailable"),
          message: z.string().max(8000),
        })
        .strict(),
    ]),
    verification_job: z.discriminatedUnion("state", [
      z.object({ state: z.literal("idle") }).strict(),
      z
        .object({
          state: z.literal("ready"),
          job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
      z
        .object({
          state: z.literal("unavailable"),
          message: z.string().max(8000),
        })
        .strict(),
    ]),
    stopped: z.union([z.string(), z.null()]),
    directories: z
      .object({
        paths: z.record(
          z.string(),
          z
            .object({
              revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            })
            .strict(),
        ),
      })
      .strict(),
    readers: z
      .object({
        files: z.record(
          z.string(),
          z
            .object({
              revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            })
            .strict(),
        ),
      })
      .strict(),
    sources: z
      .object({
        current: z.union([
          z
            .object({
              revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            })
            .strict(),
          z.null(),
        ]),
        files: z.record(
          z.string(),
          z
            .object({
              revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            })
            .strict(),
        ),
      })
      .strict(),
    source_preparation: z.record(
      z.string(),
      z
        .object({
          job: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
          resource: z.string().min(1).max(4096),
          phase: z.string().min(1).max(512),
          elapsed_ms: z.number().int().min(0),
          phase_elapsed_ms: z.number().int().min(0),
          limit_seconds: z.number().int().min(0),
          outcome: z.discriminatedUnion("state", [
            z.object({ state: z.literal("preparing") }).strict(),
            z
              .object({ state: z.literal("ready"), cache_hit: z.boolean() })
              .strict(),
            z
              .object({
                state: z.literal("failed"),
                kind: z.union([
                  z.literal("operation"),
                  z.literal("cancelled"),
                  z.literal("observation"),
                ]),
                error: z.string().max(8000),
              })
              .strict(),
          ]),
          log: z.union([
            z
              .object({
                revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
              })
              .strict(),
            z.null(),
          ]),
        })
        .strict(),
    ),
    analyses: z
      .object({
        current: z.discriminatedUnion("state", [
          z.object({ state: z.literal("empty") }).strict(),
          z
            .object({
              state: z.literal("ready"),
              revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            })
            .strict(),
          z
            .object({
              state: z.literal("unavailable"),
              message: z.string().max(8000),
            })
            .strict(),
        ]),
        history: z.discriminatedUnion("state", [
          z
            .object({
              state: z.literal("ready"),
              revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
              runs: z.number().int().min(0).max(1000),
            })
            .strict(),
          z
            .object({
              state: z.literal("unavailable"),
              message: z.string().max(8000),
            })
            .strict(),
        ]),
      })
      .strict(),
    repair_review: z.union([
      z
        .object({
          reference: z
            .object({
              id: z.string().regex(new RegExp("^repair-[a-f0-9]{64}$")),
              sha256: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
            })
            .strict(),
          model: z.string(),
          processor: z.string(),
          model_definition: z.unknown().optional(),
          finding_id: z.string(),
          requirement_id: z.string(),
          input: z
            .object({
              source_revision: z
                .string()
                .regex(new RegExp("^sha256:[a-f0-9]{64}$")),
              graph_revision: z
                .union([
                  z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                  z.null(),
                ])
                .optional(),
              target: z
                .object({
                  file: z.string().min(1).max(4096),
                  line: z.number().int().min(1).max(1048577),
                  name: z.string().min(1).max(1000),
                })
                .strict(),
              requirement: z.string().min(1).max(16384),
            })
            .strict(),
          file: z.string(),
          accepted_reference: z.union([
            z
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
            z.null(),
          ]),
          explanation: z.string(),
          diff: z.string(),
        })
        .strict(),
      z.null(),
    ]),
    preview: z.discriminatedUnion("state", [
      z.object({ state: z.literal("idle") }).strict(),
      z
        .object({
          state: z.literal("ready"),
          frame: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
          revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
      z
        .object({
          state: z.literal("unavailable"),
          message: z.string().max(8000),
        })
        .strict(),
    ]),
    exports: z
      .array(
        z
          .object({
            session: z.string().min(1).max(8192),
            id: z.number().int().min(0).max(7),
            source: z.string().max(8192),
            contract: z.union([
              z.string().regex(new RegExp("^[a-f0-9]{64}$")),
              z.null(),
            ]),
            target: z.string().min(1).max(8192),
            bytes: z.number().int().min(0).max(16777216),
            payload_sha256: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
            state: z.union([
              z.literal("pending"),
              z.literal("reviewed"),
              z.literal("rejected"),
              z.literal("attempted"),
              z.literal("response_received"),
              z.literal("delivery_uncertain"),
            ]),
          })
          .strict(),
      )
      .max(8),
    export_review: z.union([
      z
        .object({
          serial: z
            .string()
            .max(20)
            .regex(new RegExp("^[1-9][0-9]{0,19}$"))
            .refine(
              (value) =>
                /^[1-9][0-9]{0,19}$/.test(value) &&
                BigInt(value) <= 18446744073709551615n,
              { message: "Expected a canonical positive u64 decimal string" },
            ),
          summary: z
            .object({
              session: z.string().min(1).max(8192),
              id: z.number().int().min(0).max(7),
              source: z.string().max(8192),
              contract: z.union([
                z.string().regex(new RegExp("^[a-f0-9]{64}$")),
                z.null(),
              ]),
              target: z.string().min(1).max(8192),
              bytes: z.number().int().min(0).max(16777216),
              payload_sha256: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
              state: z.union([
                z.literal("pending"),
                z.literal("reviewed"),
                z.literal("rejected"),
                z.literal("attempted"),
                z.literal("response_received"),
                z.literal("delivery_uncertain"),
              ]),
            })
            .strict(),
          offset: z.number().int().min(0).max(16777216),
          body_hex: z
            .string()
            .max(4096)
            .regex(new RegExp("^(?:[0-9a-f]{2})*$")),
        })
        .strict(),
      z.null(),
    ]),
    export_outcome: z.union([
      z
        .object({
          summary: z
            .object({
              session: z.string().min(1).max(8192),
              id: z.number().int().min(0).max(7),
              source: z.string().max(8192),
              contract: z.union([
                z.string().regex(new RegExp("^[a-f0-9]{64}$")),
                z.null(),
              ]),
              target: z.string().min(1).max(8192),
              bytes: z.number().int().min(0).max(16777216),
              payload_sha256: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
              state: z.union([
                z.literal("pending"),
                z.literal("reviewed"),
                z.literal("rejected"),
                z.literal("attempted"),
                z.literal("response_received"),
                z.literal("delivery_uncertain"),
              ]),
            })
            .strict(),
          http_status: z.union([z.number().int().min(100).max(999), z.null()]),
        })
        .strict(),
      z.null(),
    ]),
    atlas: z
      .object({
        available: z.boolean(),
        prepared: z.boolean(),
        detail: z.string(),
      })
      .strict(),
  })
  .strict();
export type NativeState = z.infer<typeof nativeStateSchema>;
