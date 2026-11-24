import { prepareMap } from "./project-map-model";
import { expect, test } from "vitest";
import fc from "fast-check";
import { createHash } from "node:crypto";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { inventoryChunkSchema } from "@meden/agent-protocol/native";
import { referenceEdges, type ReferenceSymbol } from "./project-map-references";
import { projectMap } from "./project-map-model";

// Independent original array/filter algorithm. Kept only as an outcome oracle
// and a measured counterfactual, never as a production fallback.
function scanReferences(symbols: ReadonlyMap<string, ReferenceSymbol>) {
  const names = new Map<string, ReferenceSymbol[]>(),
    local = new Map<string, ReferenceSymbol[]>();
  for (const symbol of symbols.values()) {
    names.set(symbol.name, [...(names.get(symbol.name) || []), symbol]);
    const key = symbol.file + ":" + symbol.name.split(".").at(-1);
    local.set(key, [...(local.get(key) || []), symbol]);
  }
  const edges: Array<{ source: string; target: string }> = [],
    seen = new Set<string>();
  for (const symbol of symbols.values())
    for (const reference of symbol.references) {
      const parts = symbol.name.split(".").slice(0, -1),
        candidates = [
          ...parts.map(
            (_, i) =>
              parts.slice(0, parts.length - i).join(".") + "." + reference,
          ),
          reference,
        ];
      let target: ReferenceSymbol | undefined;
      for (const name of candidates) {
        const visible = (names.get(name) || []).filter(
          (s) => !s.private || s.file === symbol.file,
        );
        if (visible.length === 1) {
          target = visible[0];
          break;
        }
      }
      const fallback = local.get(symbol.file + ":" + reference);
      target ||= fallback?.length === 1 ? fallback[0] : undefined;
      if (
        target &&
        target.id !== symbol.id &&
        !seen.has(symbol.id + "->" + target.id)
      ) {
        seen.add(symbol.id + "->" + target.id);
        edges.push({ source: symbol.id, target: target.id });
      }
    }
  return edges;
}
const input = (rows: Array<Omit<ReferenceSymbol, "id">>) =>
  new Map(rows.map((row, i) => ["s" + i, { id: "s" + i, ...row }]));
const symbol = (
  file: string,
  name: string,
  references: string[] = [],
  hidden = false,
) => ({ file, name, references, private: hidden });

test("nearest visible names and qualified references retain order, cycles and single edges", () => {
  const data = input([
    symbol("A.lean", "N.Inner.start", [
      "hit",
      "N.hit",
      "hidden",
      "missing",
      "N.Inner.start",
      "start",
      "hit",
    ]),
    symbol("A.lean", "N.Inner.hit", ["start"]),
    symbol("B.lean", "N.hit"),
    symbol("B.lean", "N.Inner.hidden", [], true),
  ]);
  expect(referenceEdges(data)).toEqual([
    { source: "s0", target: "s1" },
    { source: "s0", target: "s2" },
    { source: "s1", target: "s0" },
  ]);
});
test("private visibility, global ambiguity and same-file fallback agree without selecting an arbitrary candidate", () => {
  const data = input([
    symbol("A.lean", "N.start", [
      "N.shared",
      "shared",
      "privateOnly",
      "foreign",
    ]),
    symbol("B.lean", "N.shared"),
    symbol("C.lean", "N.shared"),
    symbol("A.lean", "N.shared", [], true),
    symbol("A.lean", "N.privateOnly", [], true),
    symbol("B.lean", "N.foreign", [], true),
  ]);
  expect(referenceEdges(data)).toEqual([
    { source: "s0", target: "s3" },
    { source: "s0", target: "s4" },
  ]);
  data.set("duplicate", {
    id: "duplicate",
    ...symbol("A.lean", "N.privateOnly", [], true),
  });
  expect(referenceEdges(data)).toEqual([{ source: "s0", target: "s3" }]);
});
test("generated scopes preserve every original edge and leave the input unchanged", () => {
  const names = [
    ".x",
    "N..x",
    "x",
    "y",
    "N.x",
    "N.y",
    "N.Inner.x",
    "N.Inner.y",
    "M.x",
    "Ω.x",
    "«a.b»",
    "With:Colon.x",
  ];
  fc.assert(
    fc.property(
      fc.array(
        fc.record({
          file: fc.constantFrom(
            "A.lean",
            "B.lean",
            "Nested/Ω.lean",
            "With:Colon.lean",
          ),
          name: fc.constantFrom(...names),
          private: fc.boolean(),
          references: fc.array(fc.constantFrom(...names, "absent", "Inner.x"), {
            maxLength: 10,
          }),
        }),
        { maxLength: 65 },
      ),
      (rows) => {
        const data = input(rows),
          original = JSON.stringify([...data]);
        expect(referenceEdges(data)).toEqual(scanReferences(data));
        expect(JSON.stringify([...data])).toBe(original);
      },
    ),
    { seed: 20261125, numRuns: Number(process.env.MEDEN_FUZZ_RUNS || 500) },
  );
});

async function compare(
  symbols: ReadonlyMap<string, ReferenceSymbol>,
  output: string,
) {
  const expected = scanReferences(symbols);
  expect(expected.length).toBeGreaterThan(0);
  expect(referenceEdges(symbols)).toEqual(expected);
  const original: number[] = [],
    indexed: number[] = [];
  let consume = 0;
  const measure = (kind: "original" | "indexed") => {
    const start = performance.now(),
      result =
        kind === "original" ||
        process.env.MEDEN_PERF_MUTANT === "scan-references"
          ? scanReferences(symbols)
          : referenceEdges(symbols);
    consume += result.length;
    return performance.now() - start;
  };
  for (let i = 0; i < 9; i++)
    for (const kind of i % 2
      ? (["indexed", "original"] as const)
      : (["original", "indexed"] as const))
      (kind === "original" ? original : indexed).push(measure(kind));
  const median = (xs: number[]) =>
    [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)]!;
  const data = {
    symbols: symbols.size,
    references: [...symbols.values()].reduce(
      (n, s) => n + s.references.length,
      0,
    ),
    edges: expected.length,
    edge_sha256: createHash("sha256")
      .update(JSON.stringify(expected))
      .digest("hex"),
    repetitions: 9,
    original_ms: original,
    indexed_ms: indexed,
    original_median_ms: median(original),
    indexed_median_ms: median(indexed),
    ratio: median(indexed) / median(original),
    maximum_ratio: 0.65,
    consume,
    scope:
      "Same-process original array/filter reference resolution versus unique visibility indices; original ordered edges checked before timing. No browser rendering, native compilation or inference measurement.",
  };
  await mkdir(output, { recursive: true });
  await writeFile(
    output + "/references.json",
    JSON.stringify(data, null, 2) + "\n",
  );
  expect(data.ratio).toBeLessThan(data.maximum_ratio);
}

if (process.env.MEDEN_PERFORMANCE === "1")
  test("reference indices avoid repeated candidate scans while retaining all scoped edges", async () => {
    const rows: Array<Omit<ReferenceSymbol, "id">> = [
      symbol("Shared.lean", "N.common"),
    ];
    for (let file = 0; file < 600; file++) {
      const path = `Group/F${file}.lean`;
      rows.push(
        symbol(path, "N.helper", ["target"], true),
        symbol(path, "N.target", [], true),
      );
      for (let item = 0; item < 10; item++)
        rows.push(
          symbol(
            path,
            `N.value${file}_${item}`,
            Array.from(
              { length: 24 },
              (_, i) => ["helper", "target", "N.common", "absent"][i % 4]!,
            ),
          ),
        );
    }
    await compare(
      input(rows),
      process.env.MEDEN_PERF_OUT || "target/performance",
    );
  }, 60000);

if (process.env.MEDEN_REFERENCE_WORKLOAD)
  test("complete captured inventory retains the original reference graph", async () => {
    const body = await readFile(process.env.MEDEN_REFERENCE_WORKLOAD!, "utf8"),
      publication = JSON.parse(body),
      files = inventoryChunkSchema.shape.data.shape.files.element
        .array()
        .max(250000)
        .parse(publication.data.files);
    const before = JSON.stringify(files),
      scene = projectMap(
        prepareMap({
          files,
          inventory_only: Boolean(publication.data.inventory_only),
        }),
      );
    expect(scene.edges).toEqual(scanReferences(scene.symbols));
    const summary = {
      inventory_only: Boolean(publication.data.inventory_only),
      files: files.map(({ symbols, indexed, ...file }) =>
        indexed
          ? {
              state: "indexed" as const,
              ...file,
              headers: symbols.map(
                ({ references: _, semantic: __, ...header }) => header,
              ),
            }
          : { state: "unindexed" as const, path: file.path, bytes: file.bytes },
      ),
    };
    const initial = projectMap(prepareMap(summary, scene.index.geometry)),
      loaded = projectMap(
        initial.index,
        new Set(),
        new Map(files.map((file) => [file.path, file])),
        initial,
      );
    expect(initial.edges).toEqual([]);
    expect(loaded.edges).toEqual(scene.edges);
    expect([...loaded.symbols]).toEqual([...scene.symbols]);

    await compare(
      scene.symbols,
      process.env.MEDEN_PERF_OUT || "target/performance/actual-references",
    );
    expect(JSON.stringify(files)).toBe(before);
  }, 120000);
