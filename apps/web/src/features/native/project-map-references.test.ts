import { prepareMap } from "./project-map-model";
import { expect, test } from "vitest";
import fc from "fast-check";
import { createHash } from "node:crypto";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { inventoryChunkSchema } from "@meden/agent-protocol/native";
import { ReferenceIndex, type ReferenceSymbol } from "./project-map-references";
import { projectMap } from "./project-map-model";

const referenceEdges = (symbols: ReadonlyMap<string, ReferenceSymbol>) =>
  new ReferenceIndex(symbols).edges(symbols);

import { scanReferences } from "./project-map-oracle";

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

test("retained readings bind shared arrays to the exact file and namespace before self suppression", () => {
  const shared = ["x", "absent", "x"],
    data = input([
      symbol("A.lean", "N.x", shared, true),
      symbol("A.lean", "N.source", shared),
      symbol("A.lean", "M.x", [], true),
      symbol("A.lean", "M.source", shared),
      symbol("B.lean", "N.source", shared),
      symbol("B.lean", "N.x", [], true),
    ]),
    original = JSON.stringify([...data]),
    index = new ReferenceIndex(data);
  for (const current of [data, new Map([...data].reverse()), data])
    expect(index.edges(current)).toEqual(scanReferences(current));
  expect(JSON.stringify([...data])).toBe(original);
});

test("retained resolution preserves empty IDs and global ordered delimiter collisions", () => {
  const rows = [
    { id: "a->b", ...symbol("A.lean", "N.first", ["c"]) },
    { id: "a", ...symbol("B.lean", "N.second", ["bc"]) },
    { id: "c", ...symbol("C.lean", "N.c") },
    { id: "b->c", ...symbol("D.lean", "N.bc") },
    { id: "", ...symbol("E.lean", "N.empty") },
    { id: "user", ...symbol("F.lean", "N.user", ["empty"]) },
  ];
  const data = new Map(rows.map((s) => [s.id, s])),
    index = new ReferenceIndex(data);
  expect(index.edges(data)).toEqual([
    { source: "a->b", target: "c" },
    { source: "user", target: "" },
  ]);
  const reversed = new Map([...data].reverse());
  expect(index.edges(reversed)).toEqual(scanReferences(reversed));
  expect(index.edges(data)).toEqual(scanReferences(data));
});

if (process.env.MEDEN_PERFORMANCE === "1")
  test("small detail arrivals reuse the fixed declaration universe instead of rebuilding global indices", async () => {
    const files = Array.from({ length: 1600 }, (_, file) => ({
      path: `Group/F${file}.lean`,
      sha256: "original",
      bytes: "100",
      indexed: true,
      declarations: 24,
      theorems: 24,
      imports: [],
      symbols: Array.from({ length: 24 }, (_, n) => ({
        name: `N.t${file}_${n}`,
        line: n + 1,
        kind: "theorem" as const,
        declaration: "theorem",
        private: false,
        references: [`N.t${(file + 1) % 1600}_0`, `N.t${file}_0`, "missing"],
        semantic: null,
        main_theorem: false,
      })),
    }));
    const index = prepareMap({
      inventory_only: false,
      files: files.map(({ symbols, indexed: _, ...file }) => ({
        state: "indexed" as const,
        ...file,
        headers: symbols.map(
          ({ references: _, semantic: __, ...header }) => header,
        ),
      })),
    });
    const arrivals = Array.from({ length: 8 }, (_, i) => files[i * 137]!);
    const replay = (
      initial: ReturnType<typeof projectMap>,
      rebuild: boolean,
    ) => {
      let scene = initial;
      const details = new Map<string, (typeof files)[number]>();
      for (const file of arrivals) {
        details.set(file.path, file);
        const previous = rebuild
          ? { ...scene, references: new ReferenceIndex(scene.symbols) }
          : scene;
        scene = projectMap(index, new Set(), details, previous);
      }
      return scene;
    };
    const expected = replay(projectMap(index), true);
    expect(expected.edges.length).toBeGreaterThan(0);
    expect(expected.edges).toEqual(scanReferences(expected.symbols));
    const cached: number[] = [],
      rebuilt: number[] = [];
    let consumed = 0;
    for (let i = 0; i < 9; i++) {
      for (const rebuild of i % 2 ? [false, true] : [true, false]) {
        const initial = projectMap(index),
          start = performance.now();
        const actual = replay(
          initial,
          rebuild ||
            process.env.MEDEN_PERF_MUTANT === "rebuild-reference-index",
        );
        const elapsed = performance.now() - start;
        expect(actual.edges).toEqual(expected.edges);
        expect([...actual.symbols]).toEqual([...expected.symbols]);
        consumed += actual.edges.length;
        (rebuild ? rebuilt : cached).push(elapsed);
      }
    }
    const median = (xs: number[]) =>
      [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)]!;
    const result = {
      symbols: expected.symbols.size,
      files: files.length,
      arrivals: arrivals.length,
      edges: expected.edges.length,
      consumed,
      repetitions: 9,
      cached_ms: cached,
      rebuilt_ms: rebuilt,
      cached_median_ms: median(cached),
      rebuilt_median_ms: median(rebuilt),
      ratio: median(cached) / median(rebuilt),
      maximum_ratio: 0.65,
      scope:
        "Prepared identical headings/geometry, eight detail arrivals; fresh index per series. Interleaved forced global-index reconstruction versus retained immutable candidate universe. Exact final ordered edges/symbols checked; no browser paint, compilation or live inference.",
    };
    const output = process.env.MEDEN_PERF_OUT || "target/performance";
    await mkdir(output, { recursive: true });
    await writeFile(
      output + "/reference-updates.json",
      JSON.stringify(result, null, 2) + "\n",
    );
    expect(result.ratio).toBeLessThan(result.maximum_ratio);
  }, 120000);
