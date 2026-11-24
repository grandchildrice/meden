import type { WorkspaceOverview } from "@meden/agent-protocol/native";
import type { NavigationFile } from "./project-map-model";

type Entry = WorkspaceOverview["files"][number];
type Candidate = Omit<
  NavigationFile["symbols"][number],
  "semantic" | "main_theorem"
>;
type Retained = {
  file: Omit<NavigationFile, "symbols"> & { symbols: Candidate[] };
  bytes: number;
};
const digest = /^sha256:[a-f0-9]{64}$/;

/** Disposable lexical candidates only, never descriptions, compilation or evidence.
 * Reuse relies on the host's actual indexer/source attribution. Fresh headings
 * must still match every syntax field; current target flags come from them.
 */
export class NavigationCache {
  private readonly values = new Map<string, Retained>();
  private bytes = 0;
  constructor(
    private readonly entries = 64,
    private readonly budget = 64 * 1024 * 1024,
  ) {
    if (
      !Number.isSafeInteger(entries) ||
      entries < 1 ||
      entries > 64 ||
      !Number.isSafeInteger(budget) ||
      budget < 1 ||
      budget > 64 * 1024 * 1024
    )
      throw Error("Invalid navigation retention bounds");
  }
  private key(
    indexer: string,
    file: Pick<Entry, "path" | "sha256" | "indexed">,
  ) {
    return digest.test(indexer) &&
      file.indexed &&
      /^[a-f0-9]{64}$/.test(file.sha256)
      ? JSON.stringify([indexer, file.path, file.sha256])
      : undefined;
  }
  remember(indexer: string, original: NavigationFile) {
    const key = this.key(indexer, original);
    if (!key) return;
    const file: Retained["file"] = {
      path: original.path,
      sha256: original.sha256,
      bytes: original.bytes,
      declarations: original.declarations,
      theorems: original.theorems,
      indexed: original.indexed,
      imports: original.imports.slice(),
      symbols: original.symbols.map((s) => ({
        name: s.name,
        line: s.line,
        kind: s.kind,
        declaration: s.declaration,
        private: s.private,
        references: s.references.slice(),
      })),
    };
    const bytes = new TextEncoder().encode(JSON.stringify(file)).byteLength;
    this.remove(key);
    if (bytes > this.budget) return;
    this.values.set(key, { file, bytes });
    this.bytes += bytes;
    while (this.values.size > this.entries || this.bytes > this.budget)
      this.remove(this.values.keys().next().value!);
  }
  replay(indexer: string, entry: Entry): NavigationFile | undefined {
    const key = this.key(indexer, entry),
      retained = key ? this.values.get(key) : undefined;
    if (!retained) return;
    const file = retained.file;
    if (
      file.bytes !== entry.bytes ||
      file.declarations !== entry.declarations ||
      file.theorems !== entry.theorems ||
      file.imports.length !== entry.imports.length ||
      !file.imports.every((name, i) => name === entry.imports[i]) ||
      file.symbols.length !== entry.headers.length ||
      !file.symbols.every((s, i) => {
        const h = entry.headers[i]!;
        return (
          s.name === h.name &&
          s.line === h.line &&
          s.kind === h.kind &&
          s.declaration === h.declaration &&
          s.private === h.private
        );
      })
    )
      return;
    return {
      path: entry.path,
      sha256: entry.sha256,
      bytes: entry.bytes,
      declarations: entry.declarations,
      theorems: entry.theorems,
      imports: entry.imports.slice(),
      indexed: entry.indexed,
      symbols: entry.headers.map((header, i) => ({
        ...header,
        references: file.symbols[i]!.references.slice(),
        semantic: null,
      })),
    };
  }
  private remove(key: string) {
    const old = this.values.get(key);
    if (old) {
      this.bytes -= old.bytes;
      this.values.delete(key);
    }
  }
}
