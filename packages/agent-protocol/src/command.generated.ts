// Generated from meden_app::command::Command. Do not edit.
import { z } from "zod";
export const nativeCommandSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("initialize_work"), name: z.string() }).strict(),
  z
    .object({
      type: z.literal("put_work"),
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
      type: z.literal("note_work"),
      id: z.string(),
      text: z.string(),
      next: z.union([z.string(), z.null()]).optional(),
    })
    .strict(),
  z.object({ type: z.literal("check_work"), id: z.string() }).strict(),
  z.object({ type: z.literal("resume_work"), id: z.string() }).strict(),
  z.object({ type: z.literal("save_work") }).strict(),
  z
    .object({
      type: z.literal("approve_processor"),
      connection_revision: z
        .string()
        .max(20)
        .regex(new RegExp("^[1-9][0-9]{0,19}$"))
        .refine(
          (value) =>
            /^[1-9][0-9]{0,19}$/.test(value) &&
            BigInt(value) <= 18446744073709551615n,
          { message: "Expected a canonical positive u64 decimal string" },
        ),
      endpoint: z.string(),
      model: z.string(),
      judge: z
        .union([
          z.object({ model: z.string(), endpoint: z.string() }).strict(),
          z.null(),
        ])
        .optional(),
    })
    .strict(),
  z
    .object({
      type: z.literal("select_connection"),
      provider: z.string(),
      model: z.string(),
    })
    .strict(),
  z.object({ type: z.literal("list_models"), provider: z.string() }).strict(),
  z.object({ type: z.literal("refresh_workspace") }).strict(),
  z.object({ type: z.literal("refresh_connections") }).strict(),
  z.object({ type: z.literal("revoke_processor") }).strict(),
  z.object({ type: z.literal("instruction"), text: z.string() }).strict(),
  z.object({ type: z.literal("cancel") }).strict(),
  z.object({ type: z.literal("refresh_status") }).strict(),
  z.object({ type: z.literal("prepare_atlas") }).strict(),
  z
    .object({ type: z.literal("atlas_snapshot"), paths: z.array(z.string()) })
    .strict(),
  z
    .object({
      type: z.literal("review_export"),
      reference: z
        .object({
          session: z.string().min(1).max(8192),
          id: z.number().int().min(0).max(7),
        })
        .strict(),
      offset: z.number().int().min(0),
    })
    .strict(),
  z
    .object({
      type: z.literal("send_export"),
      reference: z
        .object({
          session: z.string().min(1).max(8192),
          id: z.number().int().min(0).max(7),
        })
        .strict(),
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
    })
    .strict(),
  z
    .object({
      type: z.literal("reject_export"),
      reference: z
        .object({
          session: z.string().min(1).max(8192),
          id: z.number().int().min(0).max(7),
        })
        .strict(),
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
    })
    .strict(),
  z
    .object({
      type: z.literal("close_export"),
      reference: z
        .object({
          session: z.string().min(1).max(8192),
          id: z.number().int().min(0).max(7),
        })
        .strict(),
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
    })
    .strict(),
  z
    .object({
      type: z.literal("capture_preview"),
      root: z.string(),
      entry: z.string(),
    })
    .strict(),
  z
    .object({ type: z.literal("source_snapshot"), paths: z.array(z.string()) })
    .strict(),
  z
    .object({ type: z.literal("prepare_repair"), finding_id: z.string() })
    .strict(),
  z
    .object({
      type: z.literal("apply_repair"),
      reference: z
        .object({
          id: z.string().regex(new RegExp("^repair-[a-f0-9]{64}$")),
          sha256: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
    })
    .strict(),
  z
    .object({
      type: z.literal("reject_repair"),
      reference: z
        .object({
          id: z.string().regex(new RegExp("^repair-[a-f0-9]{64}$")),
          sha256: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
        })
        .strict(),
    })
    .strict(),
  z.object({ type: z.literal("select_analysis"), id: z.string() }).strict(),
  z
    .object({
      type: z.literal("analyze_meaning"),
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
    })
    .strict(),
  z
    .object({
      type: z.literal("review_contract"),
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
              { message: "Expected a canonical positive u64 decimal string" },
            ),
          session: z.string().min(1).max(8192),
          contract: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
        })
        .strict(),
    })
    .strict(),
  z
    .object({
      type: z.literal("accept_contract"),
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
              { message: "Expected a canonical positive u64 decimal string" },
            ),
          session: z.string().min(1).max(8192),
          contract: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
        })
        .strict(),
    })
    .strict(),
  z
    .object({
      type: z.literal("reject_contract"),
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
              { message: "Expected a canonical positive u64 decimal string" },
            ),
          session: z.string().min(1).max(8192),
          contract: z.string().regex(new RegExp("^[a-f0-9]{64}$")),
        })
        .strict(),
    })
    .strict(),
  z
    .object({
      type: z.literal("inspect"),
      path: z.string(),
      directory: z.boolean(),
    })
    .strict(),
]);
export type NativeCommand = z.infer<typeof nativeCommandSchema>;
