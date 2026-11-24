import { expect, test } from "vitest";
import { NavigationCache } from "./navigation-cache";
import type { NavigationFile } from "./project-map-model";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";

const indexer = "sha256:" + "0".repeat(64);
function file(path = "A.lean"): NavigationFile {
  return {
    path,
    sha256: "a".repeat(64),
    bytes: "100",
    declarations: 1,
    theorems: 1,
    imports: ["B"],
    indexed: true,
    symbols: [
      {
        name: "N.claim",
        line: 2,
        kind: "theorem",
        declaration: "lemma",
        private: false,
        references: ["helper"],
        main_theorem: false,
        semantic: null,
      },
    ],
  };
}
function heading(
  file: NavigationFile,
): Extract<WorkspaceOverview["files"][number], { state: "indexed" }> {
  const { symbols, indexed: _, ...metadata } = file;
  return {
    state: "indexed",
    ...metadata,
    headers: symbols.map(
      ({ references: _, semantic: __, ...header }) => header,
    ),
  };
}

test("source-bound candidates own their data and replay fresh targets without retaining descriptions", () => {
  const cache = new NavigationCache(),
    original = file(),
    entry = heading(original);
  Object.defineProperty(original.symbols[0], "semantic", {
    get() {
      throw Error("Description retained by lexical cache");
    },
  });
  cache.remember(indexer, original);
  original.symbols[0]!.references[0] = "mutated";
  entry.headers[0]!.main_theorem = true;
  const replay = cache.replay(indexer, entry)!;
  expect(replay.symbols[0]!.references).toEqual(["helper"]);
  expect(replay.symbols[0]!.main_theorem).toBe(true);
  expect(replay.symbols[0]!.semantic).toBeNull();
  replay.symbols[0]!.references[0] = "changed again";
  expect(cache.replay(indexer, entry)!.symbols[0]!.references).toEqual([
    "helper",
  ]);
});

test("method/source changes and inconsistent fresh headings cannot reuse old candidates", () => {
  const cache = new NavigationCache(),
    original = file(),
    entry = heading(original);
  cache.remember(indexer, original);
  expect(cache.replay("sha256:" + "1".repeat(64), entry)).toBeUndefined();
  expect(
    cache.replay(indexer, { ...entry, sha256: "b".repeat(64) }),
  ).toBeUndefined();
  expect(
    cache.replay(indexer, {
      state: "unindexed",
      path: entry.path,
      bytes: entry.bytes,
    }),
  ).toBeUndefined();
  for (const changed of [
    { ...entry, bytes: "101" },
    { ...entry, declarations: 2 },
    { ...entry, imports: [] },
    { ...entry, headers: [{ ...entry.headers[0]!, name: "N.changed" }] },
    { ...entry, headers: [{ ...entry.headers[0]!, private: true }] },
    { ...entry, headers: [{ ...entry.headers[0]!, line: 3 }] },
  ])
    expect(cache.replay(indexer, changed)).toBeUndefined();
  expect(cache.replay(indexer, entry)).toBeDefined();
});

test("retention evicts complete candidates by count/serialized bytes and never caches unread input", () => {
  const a = file(),
    b = file("B.lean"),
    cache = new NavigationCache(1);
  cache.remember(indexer, a);
  cache.remember(indexer, b);
  expect(cache.replay(indexer, heading(a))).toBeUndefined();
  expect(cache.replay(indexer, heading(b))).toBeDefined();
  const bounded = new NavigationCache(2, 1000);
  a.symbols[0]!.references = ["x".repeat(400)];
  b.symbols[0]!.references = ["y".repeat(400)];
  bounded.remember(indexer, a);
  bounded.remember(indexer, b);
  expect(bounded.replay(indexer, heading(a))).toBeUndefined();
  expect(bounded.replay(indexer, heading(b))).toBeDefined();
  const tiny = new NavigationCache(1, 1);
  tiny.remember(indexer, a);
  expect(tiny.replay(indexer, heading(a))).toBeUndefined();
  cache.remember(indexer, { ...a, indexed: false });
  expect(cache.replay(indexer, heading(a))).toBeUndefined();
  expect(cache.replay(indexer, heading(b))).toBeDefined();
  for (const count of [0, -1, 65, NaN])
    expect(() => new NavigationCache(count)).toThrow();
});
