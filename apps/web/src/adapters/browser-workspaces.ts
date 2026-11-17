import { openDB, type DBSchema } from "idb";
import type { WorkspaceRepository } from "@meden/agent-protocol";
import { workspaceSchema, type Workspace } from "@meden/workspace-schema";

interface WorkspaceDatabase extends DBSchema {
  workspaces: { key: string; value: Workspace; indexes: { updatedAt: string } };
}

/** Temporary browser storage, replaceable by the host SQLite adapter. */
export class BrowserWorkspaceRepository implements WorkspaceRepository {
  private db = openDB<WorkspaceDatabase>("meden-workspaces", 1, {
    upgrade(db) {
      db.createObjectStore("workspaces", { keyPath: "id" }).createIndex(
        "updatedAt",
        "updatedAt",
      );
    },
  });
  async list(): Promise<Workspace[]> {
    const values = await (
      await this.db
    ).getAllFromIndex("workspaces", "updatedAt");
    return values.reverse().map((value) => workspaceSchema.parse(value));
  }
  async save(workspace: Workspace): Promise<void> {
    await (await this.db).put("workspaces", workspaceSchema.parse(workspace));
  }
  async remove(id: string): Promise<void> {
    await (await this.db).delete("workspaces", id);
  }
}
