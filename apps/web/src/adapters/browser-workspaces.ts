import {
  mergeVendingHistory,
  VENDING_WORKSPACE_ID,
} from "./merge-vending-history";
import { openDB, type DBSchema } from "idb";
import type { WorkspaceRepository } from "@meden/agent-protocol";
import { workspaceSchema, type Workspace } from "@meden/workspace-schema";

interface WorkspaceDatabase extends DBSchema {
  workspaces: { key: string; value: Workspace; indexes: { updatedAt: string } };
}

/** Temporary browser storage, replaceable by the host SQLite adapter. */
export class BrowserWorkspaceRepository implements WorkspaceRepository {
  private db = openDB<WorkspaceDatabase>("meden-workspaces", 2, {
    upgrade(db, oldVersion, _newVersion, transaction) {
      if (oldVersion < 1)
        db.createObjectStore("workspaces", { keyPath: "id" }).createIndex(
          "updatedAt",
          "updatedAt",
        );
      if (oldVersion < 2) {
        const workspaces = transaction.objectStore("workspaces");
        void workspaces
          .get(VENDING_WORKSPACE_ID)
          .then(async (original) => {
            if (!original) return;
            const merged = mergeVendingHistory(workspaceSchema.parse(original));
            await workspaces.put(merged);
          })
          .catch(() => transaction.abort());
      }
    },
    blocking: () => {
      void this.db.then((db) => db.close());
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
