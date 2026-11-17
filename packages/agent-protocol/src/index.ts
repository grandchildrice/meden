import type { Workspace } from "@meden/workspace-schema";
import { z } from "zod";

/** GUI transport contract only. Admission and approval stay in the Rust controller. */
export const jobSchema = z
  .object({
    id: z.string(),
    workspaceId: z.string(),
    inputRevision: z.number().int().nonnegative(),
    sourceRevision: z.string(),
    kind: z.enum(["agent_turn", "semantic_analysis", "verification"]),
    state: z.enum(["queued", "running", "complete", "failed", "cancelled"]),
  })
  .strict();
export const eventSchema = z.discriminatedUnion("type", [
  z.object({ type: z.literal("job.changed"), job: jobSchema }).strict(),
  z
    .object({
      type: z.literal("message.delta"),
      workspaceId: z.string(),
      conversationId: z.string(),
      messageId: z.string(),
      text: z.string(),
    })
    .strict(),
  z
    .object({ type: z.literal("connection.closed"), reason: z.string() })
    .strict(),
]);
export type AgentEvent = z.infer<typeof eventSchema>;
export type Job = z.infer<typeof jobSchema>;
export type ConnectionState =
  | { status: "unavailable"; reason: string }
  | { status: "connecting" }
  | { status: "connected"; sessionId: string };

export interface AgentClient {
  readonly state: ConnectionState;
  submit(request: {
    workspaceId: string;
    conversationId: string;
    text: string;
    inputRevision: number;
  }): Promise<{ jobId: string }>;
  cancel(jobId: string): Promise<void>;
  subscribe(listener: (event: AgentEvent) => void): () => void;
}
export interface WorkspaceRepository {
  list(): Promise<Workspace[]>;
  save(workspace: Workspace): Promise<void>;
  remove(id: string): Promise<void>;
}
