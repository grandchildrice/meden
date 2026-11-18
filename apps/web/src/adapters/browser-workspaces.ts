import { mergeVendingHistory } from "./merge-vending-history";
import { openDB, type DBSchema } from "idb";
import type { WorkspaceRepository } from "@meden/agent-protocol";
import { workspaceSchema, type Workspace } from "@meden/workspace-schema";

interface WorkspaceDatabase extends DBSchema {
  workspaces: { key: string; value: Workspace; indexes: { updatedAt: string } };
}

/** Temporary browser storage, replaceable by the host SQLite adapter. */
export class BrowserWorkspaceRepository implements WorkspaceRepository {
  private db = openDB<WorkspaceDatabase>("meden-workspaces", 3, {
    upgrade(db, oldVersion) {
      if (oldVersion < 1)
        db.createObjectStore("workspaces", { keyPath: "id" }).createIndex(
          "updatedAt",
          "updatedAt",
        );
    },
    blocking: () => {
      void this.db.then((db) => db.close());
    },
  });
  async list(): Promise<Workspace[]> {
    const db = await this.db;
    const transaction = db.transaction("workspaces", "readwrite");
    const store = transaction.objectStore("workspaces");
    const values = await store.index("updatedAt").getAll();
    const workspaces = values
      .reverse()
      .map((value) => mergeVendingHistory(workspaceSchema.parse(value)));
    for (const workspace of workspaces)
      if (
        JSON.stringify(workspace.conversations) !==
        JSON.stringify(
          values.find((value) => value.id === workspace.id)?.conversations,
        )
      )
        await store.put(workspace);
    await transaction.done;
    return workspaces;
  }
  async save(workspace: Workspace): Promise<void> {
    await (
      await this.db
    ).put("workspaces", mergeVendingHistory(workspaceSchema.parse(workspace)));
  }
  async remove(id: string): Promise<void> {
    await (await this.db).delete("workspaces", id);
  }
}
