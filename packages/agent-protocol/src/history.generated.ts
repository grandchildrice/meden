// Generated from meden_app::view::history::Snapshot. Do not edit.
import { z } from "zod";
export const historySnapshotSchema = z
  .object({
    revision: z.string().regex(new RegExp("^sha256:[a-f0-9]{64}$")),
    data: z
      .object({
        portable: z
          .object({
            document: z
              .union([
                z
                  .object({
                    version: z.number().int().min(1).max(1),
                    name: z.string(),
                    tasks: z.array(
                      z
                        .object({
                          id: z.string(),
                          title: z.string(),
                          requirement: z.string(),
                          module: z.string(),
                          theorems: z.array(z.string()),
                          next: z.string(),
                        })
                        .strict(),
                    ),
                    events: z.array(
                      z
                        .object({
                          id: z.string(),
                          task: z.string(),
                          time: z.number().int().min(0),
                          note: z.string(),
                          check: z
                            .union([
                              z
                                .object({
                                  engine_sha256: z.string(),
                                  requirement: z.string(),
                                  theorems: z.array(z.string()),
                                  inspection: z.string(),
                                  source_revision: z.string(),
                                  after_revision: z
                                    .union([z.string(), z.null()])
                                    .optional(),
                                  module: z.string(),
                                  success: z.boolean(),
                                  termination: z.string(),
                                  stdout: z.string(),
                                  stderr: z.string(),
                                })
                                .strict(),
                              z.null(),
                            ])
                            .optional(),
                        })
                        .strict(),
                    ),
                    conversations: z.array(
                      z
                        .object({
                          id: z.string(),
                          messages: z.array(
                            z
                              .object({
                                speaker: z.enum(["user", "assistant"]),
                                text: z.string(),
                              })
                              .strict(),
                          ),
                        })
                        .strict(),
                    ),
                  })
                  .strict(),
                z.null(),
              ])
              .optional(),
            error: z.union([z.string(), z.null()]).optional(),
            lean_available: z.boolean(),
            lean: z.string(),
          })
          .strict(),
        messages: z.array(
          z
            .object({
              speaker: z.enum(["user", "assistant"]),
              text: z.string(),
            })
            .strict(),
        ),
        logs: z
          .array(
            z.discriminatedUnion("type", [
              z
                .object({ type: z.literal("notice"), text: z.string() })
                .strict(),
              z
                .object({
                  type: z.literal("call"),
                  id: z.string(),
                  tool: z.string(),
                  text: z.string(),
                })
                .strict(),
              z
                .object({
                  type: z.literal("result"),
                  id: z.string(),
                  tool: z.string(),
                  text: z.string(),
                  error: z.boolean(),
                })
                .strict(),
            ]),
          )
          .max(128),
      })
      .strict(),
  })
  .strict();
export type HistorySnapshot = z.infer<typeof historySnapshotSchema>;
