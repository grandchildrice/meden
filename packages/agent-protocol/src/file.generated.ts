// Generated from meden_app::view::file::FileView. Do not edit.
import { z } from "zod";
export const fileViewSchema = z
  .object({
    path: z.string().min(1).max(4096),
    navigation: z.union([z.literal("declarations"), z.literal("source_only")]),
    reading: z.union([
      z
        .object({
          state: z.literal("ready"),
          content: z.union([
            z
              .object({
                kind: z.literal("text"),
                revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
                bytes: z.number().int().min(0).max(1048576),
                text: z.string().max(1048576),
              })
              .strict(),
            z
              .object({
                kind: z.literal("binary"),
                bytes: z.number().int().min(0).max(1048576),
              })
              .strict(),
            z
              .object({
                kind: z.literal("unsupported_encoding"),
                bytes: z.number().int().min(0).max(1048576),
              })
              .strict(),
            z
              .object({
                kind: z.literal("size_limit"),
                bytes: z.string().regex(new RegExp("^(0|[1-9][0-9]{0,19})$")),
              })
              .strict(),
          ]),
        })
        .strict(),
      z
        .object({
          state: z.literal("unavailable"),
          failure: z.union([
            z.literal("operation"),
            z.literal("refused"),
            z.literal("unavailable"),
            z.literal("interrupted"),
            z.literal("cancelled"),
          ]),
          message: z.string().max(8000),
        })
        .strict(),
    ]),
  })
  .strict();
export type FileView = z.infer<typeof fileViewSchema>;
