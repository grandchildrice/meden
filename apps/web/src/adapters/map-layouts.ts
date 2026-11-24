import { openDB, type DBSchema, type IDBPDatabase } from "idb";
import {
  mapGeometry,
  packGeometry,
  unpackGeometry,
  type LayoutInput,
  type MapGeometry,
} from "../features/native/project-map-layout";

const MAX_BYTES = 64 * 1024 * 1024;
const MAX_ENTRIES = 8;
interface Entry {
  key: string;
  blob: Blob;
  digest: string;
  bytes: number;
}
interface LayoutDatabase extends DBSchema {
  layouts: { key: number; value: Entry; indexes: { key: string } };
}
export type PreparedMapLayout = {
  key: string;
  revision: string;
  buffer: ArrayBuffer;
  input: LayoutInput;
  geometry: MapGeometry;
  source: "cache" | "computed";
};
async function digest(bytes: Uint8Array<ArrayBuffer>) {
  const hash = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return (
    "sha256:" +
    Array.from(hash, (b) => b.toString(16).padStart(2, "0")).join("")
  );
}
export async function layoutDigest(text: string) {
  return digest(new TextEncoder().encode(text));
}
async function artifactDigest(key: string, buffer: ArrayBuffer) {
  const prefix = new TextEncoder().encode(key),
    bytes = new Uint8Array(prefix.length + buffer.byteLength);
  bytes.set(prefix);
  bytes.set(new Uint8Array(buffer), prefix.length);
  return digest(bytes);
}

/** Disposable geometry only. No source, model approval or evidence is stored.
 * Failed/unavailable storage falls back to the same deterministic computation.
 */
export class BrowserMapLayouts {
  private database?: Promise<IDBPDatabase<LayoutDatabase>>;
  private db() {
    return (this.database ??= openDB<LayoutDatabase>("meden-map-layouts", 1, {
      upgrade(db) {
        db.createObjectStore("layouts", { autoIncrement: true }).createIndex(
          "key",
          "key",
          { unique: true },
        );
      },
      blocking: () => {
        void this.database?.then((db) => db.close());
        this.database = undefined;
      },
    }).catch((error) => {
      this.database = undefined;
      throw error;
    }));
  }
  async prepare(input: LayoutInput): Promise<PreparedMapLayout> {
    const key = await layoutDigest(input.text);
    try {
      const db = await this.db(),
        entry = await db.getFromIndex("layouts", "key", key);
      if (entry) {
        if (
          entry.blob instanceof Blob &&
          entry.bytes === entry.blob.size &&
          entry.bytes <= MAX_BYTES
        ) {
          const buffer = await entry.blob.arrayBuffer();
          const geometry =
            (await artifactDigest(key, buffer)) === entry.digest
              ? unpackGeometry(buffer, input)
              : undefined;
          if (geometry)
            return {
              key,
              revision: entry.digest,
              buffer,
              input,
              geometry,
              source: "cache",
            };
        }
        const id = await db.getKeyFromIndex("layouts", "key", key);
        if (id !== undefined) await db.delete("layouts", id);
      }
    } catch {
      /* A cache failure cannot prevent ordinary map browsing. */
    }
    const geometry = mapGeometry(input),
      buffer = packGeometry(geometry, input);
    return {
      key,
      revision: await artifactDigest(key, buffer),
      buffer,
      input,
      geometry,
      source: "computed",
    };
  }
  /** Run after the computed view is committed, not in the input-to-render path. */
  async retain(prepared: PreparedMapLayout) {
    try {
      const buffer = prepared.buffer,
        bytes = buffer.byteLength;
      if (bytes > MAX_BYTES) return;
      const hash = prepared.revision,
        db = await this.db();
      const tx = db.transaction("layouts", "readwrite"),
        store = tx.objectStore("layouts");
      const old = await store.index("key").getKey(prepared.key);
      if (old !== undefined) await store.delete(old);
      await store.add({
        key: prepared.key,
        blob: new Blob([buffer]),
        digest: hash,
        bytes,
      });
      let size = 0,
        count = 0;
      // Read only small metadata before deleting oldest entries; never getAll
      // blob contents merely to enforce the cache budget.
      let cursor = await store.openCursor();
      const entries: Array<{ key: number; bytes: number }> = [];
      while (cursor) {
        const value = cursor.value;
        if (
          !(value.blob instanceof Blob) ||
          value.bytes !== value.blob.size ||
          value.bytes < 4 ||
          value.bytes > MAX_BYTES
        ) {
          await cursor.delete();
        } else {
          entries.push({ key: cursor.key, bytes: value.bytes });
          size += value.bytes;
          count++;
        }
        cursor = await cursor.continue();
      }
      for (const entry of entries) {
        if (size <= MAX_BYTES && count <= MAX_ENTRIES) break;
        size -= entry.bytes;
        count--;
        await store.delete(entry.key);
      }
      await tx.done;
    } catch {
      /* This derived cache is optional; the prepared view stays usable. */
    }
  }
}
