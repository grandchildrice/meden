// Generated from meden_app::view::publication::Envelope<directory::Directory>. Do not edit.
import { z } from "zod";
export const directoryPublicationSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z
      .object({
        path: z.string().min(1).max(4096),
        reading: z.discriminatedUnion("state", [
          z
            .object({
              state: z.literal("ready"),
              entries: z
                .array(
                  z
                    .object({
                      name: z.string().min(1).max(255),
                      kind: z.union([
                        z.literal("file"),
                        z.literal("directory"),
                        z.literal("symlink"),
                        z.literal("other"),
                      ]),
                    })
                    .strict(),
                )
                .max(100000),
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
      .strict(),
  })
  .strict();
export type DirectoryPublication = z.infer<typeof directoryPublicationSchema>;
