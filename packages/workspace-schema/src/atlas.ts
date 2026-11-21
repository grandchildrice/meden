import { z } from "zod";
import { explanationSchema } from "./explanation.ts";

/** Atlas data is analysis input, never a native proof or approval receipt. */
export const atlasGraphSchema = z.object({
  metadata: z.object({
    projectName: z.string(),
    leanVersion: z.string().optional(),
    generatedAt: z.string().optional(),
  }),
  nodes: z
    .array(
      z.object({
        id: z.string().min(1).max(1000),
        shortName: z.string(),
        kind: z.string(),
        filePath: z.string(),
        lineStart: z.number().int().positive().nullable().optional(),
        lineEnd: z.number().int().positive().nullable().optional(),
        hasSorry: z.boolean(),
        statement: z.string().max(65536).optional(),
        value: z.string().max(65536).nullable().optional(),
        semantic: explanationSchema.optional(),
        axioms: z.array(z.string()).optional(),
        standardAxioms: z.array(z.string()).optional(),
        externalDependencies: z.array(z.string()).optional(),
        meta: z
          .object({
            summary: z.string().nullable().optional(),
            isMainTheorem: z.boolean().optional(),
            axioms: z.array(z.string()).nullable().optional(),
          })
          .passthrough()
          .optional(),
      }),
    )
    .max(20_000),
  edges: z
    .array(
      z.object({
        source: z.string(),
        target: z.string(),
        kind: z.string().optional(),
      }),
    )
    .max(100_000),
  externalReferences: z.array(z.string()).max(50_000).optional(),
});
export type AtlasGraph = z.infer<typeof atlasGraphSchema>;
