import type { Workspace } from "@meden/workspace-schema";

/** Storage port; implementations own browser or host I/O. */
export interface WorkspaceRepository {
  list(): Promise<Workspace[]>;
  save(workspace: Workspace): Promise<void>;
  remove(id: string): Promise<void>;
}
