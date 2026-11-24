import { expect, test, vi } from "vitest";
import fc from "fast-check";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";
import {
  layoutInput,
  mapGeometry,
  packGeometry,
  unpackGeometry,
  MAP_LAYOUT,
  comparePath,
} from "./project-map-layout";
import { projectMap } from "./project-map-model";
import { BrowserMapLayouts } from "../../adapters/map-layouts";

function fixture(paths: string[]): WorkspaceOverview {
  return {
    generation: "1",
    excluded: [],
    inventory_only: true,
    files: paths.map((path) => ({
      path,
      sha256: "",
      bytes: "1",
      declarations: 0,
      theorems: 0,
      imports: [],
      symbols: [],
      indexed: false,
    })),
  };
}
test("layout version 1 retains its exact weighted Unicode world-coordinate fixture", () => {
  const data = fixture([
    "src/ä.lean",
    "src/A.lean",
    "src/a.lean",
    "tests/Ω.lean",
    "src/deep/🧪.lean",
  ]);
  data.inventory_only = false;
  data.files.forEach((file, i) => {
    file.declarations = i + 1;
  });
  const input = layoutInput(data);
  // Deliberate geometry/ordering changes require a new artifact version.
  expect(MAP_LAYOUT.version).toBe(1);
  expect(
    createHash("sha256")
      .update(input.text)
      .update(new Uint8Array(packGeometry(mapGeometry(input), input)))
      .digest("hex"),
  ).toBe("c0e635c295551ce7eada2235d260c288a8277b95a820cb41165f268d74f06a8d");
});
test("layout identity and world coordinates ignore input enumeration, locale, source text and lazy labels", () => {
  const data = fixture([
    "src/ä.lean",
    "src/Z.lean",
    "src/A.lean",
    "src/a.lean",
    "src/🧪.lean",
  ]);
  const input = layoutInput(data),
    geometry = mapGeometry(input);
  const locale = vi
    .spyOn(String.prototype, "localeCompare")
    .mockImplementation(() => {
      throw Error("Ambient locale used");
    });
  try {
    const other = {
      ...data,
      generation: "999",
      files: [...data.files].reverse().map((file) => ({
        ...file,
        sha256: "changed",
        bytes: "99",
        declarations: 10,
        theorems: 5,
      })),
    };
    expect(layoutInput(other).text).toBe(input.text);
    expect(mapGeometry(layoutInput(other))).toEqual(geometry);
    expect(packGeometry(geometry, input)).toEqual(
      packGeometry(mapGeometry(layoutInput(other)), layoutInput(other)),
    );
    expect(layoutInput({ ...other, inventory_only: false }).text).not.toBe(
      input.text,
    );
    expect(layoutInput({ ...data, files: data.files.slice(1) }).text).not.toBe(
      input.text,
    );
  } finally {
    locale.mockRestore();
  }
});
test("packed coordinates round trip complete generated trees without mutating their original inventory", () => {
  fc.assert(
    fc.property(
      fc.uniqueArray(
        fc.array(fc.integer({ min: 0, max: 9 }), {
          minLength: 1,
          maxLength: 7,
        }),
        { maxLength: 60, selector: (row) => row.join("/") },
      ),
      fc.boolean(),
      (rows, inventoryOnly) => {
        const data = fixture(
          rows.map((row, i) => row.join("/") + `/F${i}.lean`),
        );
        data.inventory_only = inventoryOnly;
        data.files.forEach((file, i) => {
          file.declarations = i % 7;
        });
        const before = JSON.stringify(data),
          input = layoutInput(data);
        const geometry = mapGeometry(input),
          packed = packGeometry(geometry, input);
        // Independent prefix enumeration checks the optimized sorted trie,
        // including exact dictionary order and all aggregate weights/counts.
        const expected = new Map<
          string,
          {
            index: number;
            path: string;
            parent: string;
            files: number;
            weight: number;
            leaf: boolean;
          }
        >();
        for (const file of [...data.files].sort((a, b) =>
          comparePath(a.path, b.path),
        )) {
          const parts = file.path.split("/");
          const weight = inventoryOnly ? 1 : Math.max(1, file.declarations);
          for (let depth = 1; depth <= parts.length; depth++) {
            const path = parts.slice(0, depth).join("/"),
              found = expected.get(path);
            if (found) {
              found.files++;
              found.weight += weight;
            } else
              expected.set(path, {
                index: expected.size,
                path,
                parent: parts.slice(0, depth - 1).join("/"),
                files: 1,
                weight,
                leaf: depth === parts.length,
              });
          }
        }
        expect(
          [...input.groups.values()].map(({ children: _, ...group }) => group),
        ).toEqual([...expected.values()]);
        expect(unpackGeometry(packed, input)).toEqual(geometry);
        expect(JSON.stringify(data)).toBe(before);
        expect(
          geometry.areas.filter((a) =>
            data.files.some((f) => f.path === a.path),
          ),
        ).toHaveLength(data.files.length);
      },
    ),
    { seed: 20261124, numRuns: 150 },
  );
});
test("foreign, incomplete, duplicate, non-finite and out-of-bounds cached coordinates cannot become a prepared map", () => {
  const input = layoutInput(
      fixture(["A/One.lean", "A/Inner/Two.lean", "B/Three.lean"]),
    ),
    geometry = mapGeometry(input),
    original = packGeometry(geometry, input);
  for (const fault of [
    "magic",
    "index",
    "duplicate",
    "parent",
    "nan",
    "outside",
    "zero",
  ]) {
    const buffer = original.slice(0),
      view = new DataView(buffer);
    if (fault === "magic") view.setUint32(0, 0, true);
    if (fault === "index") view.setUint32(4, 0xffffffff, true);
    if (fault === "duplicate")
      view.setUint32(40, view.getUint32(4, true), true);
    if (fault === "parent")
      view.setUint32(4, input.groups.get("A/Inner/Two.lean")!.index, true);
    if (fault === "nan") view.setFloat64(8, NaN, true);
    if (fault === "outside") view.setFloat64(8, -100, true);
    if (fault === "zero") view.setFloat64(24, 0, true);
    expect(unpackGeometry(buffer, input), fault).toBeUndefined();
  }
  expect(unpackGeometry(original.slice(0, -1), input)).toBeUndefined();
  expect(() => packGeometry({ ...geometry, input: "foreign" }, input)).toThrow(
    "Foreign map geometry",
  );
  expect(packGeometry(unpackGeometry(original, input)!, input)).toEqual(
    original,
  );
});
test("unsupported depth, file conflicts and bad weights refuse the whole layout while leaving source data intact", () => {
  for (const data of [
    fixture(["A.lean", "A.lean/Child.lean"]),
    fixture(["../outside.lean"]),
    fixture(["A.lean", "A.lean"]),
    fixture(["D/".repeat(MAP_LAYOUT.maximumDepth) + "File.lean"]),
  ]) {
    const before = JSON.stringify(data);
    expect(() => layoutInput(data)).toThrow();
    expect(JSON.stringify(data)).toBe(before);
  }
});
test("updating a prepared declaration overlay agrees with a complete rebuild and retains original boxes", () => {
  const original = fixture(["A/First.lean", "A/Second.lean", "B/Third.lean"]),
    input = layoutInput(original),
    geometry = mapGeometry(input),
    before = projectMap(original, new Set(), geometry);
  const changed = {
    ...original,
    files: original.files.map((f, i) =>
      i
        ? f
        : {
            ...f,
            indexed: true,
            sha256: "observed",
            declarations: 2,
            theorems: 2,
            symbols: [
              {
                name: "N.second",
                line: 8,
                kind: "theorem" as const,
                declaration: "lemma",
                private: false,
                references: ["first"],
                semantic: null,
                main_theorem: false,
              },
              {
                name: "N.first",
                line: 1,
                kind: "theorem" as const,
                declaration: "theorem",
                private: false,
                references: ["second"],
                semantic: null,
                main_theorem: true,
              },
            ],
          },
    ),
  };
  const tracked = new Set(["N.second"]),
    updated = projectMap(changed, tracked, geometry, before),
    full = projectMap(changed, tracked);
  expect(updated.areas).toEqual(full.areas);
  expect(updated.edges).toEqual(full.edges);
  expect(updated.mainPins).toEqual(full.mainPins);
  expect([...updated.children]).toEqual([...full.children]);
  expect([...updated.symbols]).toEqual([...full.symbols]);
  expect(updated.byPath.get("A/Second.lean")).toBe(
    before.byPath.get("A/Second.lean"),
  );
  expect(before.symbols.size).toBe(0);
  expect(updated.geometry).toBe(geometry);
});
test("the declared implementation version matches the actually installed layout engine", async () => {
  const pkg = JSON.parse(
    await readFile("node_modules/d3-hierarchy/package.json", "utf8"),
  );
  expect(MAP_LAYOUT.implementation).toBe(
    `d3-hierarchy@${pkg.version}/treemapBinary`,
  );
});
test("unavailable browser storage computes the same disposable layout without inference or hidden fallback data", async () => {
  const input = layoutInput(fixture(["A.lean", "B.lean"])),
    store = new BrowserMapLayouts();
  const first = await store.prepare(input),
    again = await store.prepare(input);
  expect(first.source).toBe("computed");
  expect(first.revision).toBe(again.revision);
  expect(first.geometry).toEqual(mapGeometry(input));
  await expect(store.retain(first)).resolves.toBeUndefined();
});

if (process.env.MEDEN_PERFORMANCE === "1")
  test("prepared large-map overlays avoid repeating full layout without changing the resulting scene", async () => {
    const data = fixture(
      Array.from(
        { length: 175187 },
        (_, i) => `D${i % 24}/Part${Math.floor(i / 24) % 12}/F${i}.lean`,
      ),
    );
    const inputStart = performance.now(),
      input = layoutInput(data),
      inputMs = performance.now() - inputStart;
    const geometryStart = performance.now(),
      geometry = mapGeometry(input),
      geometryMs = performance.now() - geometryStart;
    const sceneStart = performance.now(),
      before = projectMap(data, new Set(), geometry),
      sceneMs = performance.now() - sceneStart;
    const changed = {
      ...data,
      files: data.files.map((f, i) =>
        i === 80000
          ? {
              ...f,
              indexed: true,
              sha256: "new",
              declarations: 1,
              theorems: 1,
              symbols: [
                {
                  name: "new_target",
                  line: 1,
                  kind: "theorem" as const,
                  declaration: "theorem",
                  private: false,
                  references: [],
                  semantic: null,
                  main_theorem: true,
                },
              ],
            }
          : f,
      ),
    };
    const reference = projectMap(changed),
      candidate = projectMap(changed, new Set(), geometry, before);
    expect(candidate.areas).toEqual(reference.areas);
    expect(candidate.mainPins).toEqual(reference.mainPins);
    const full: number[] = [],
      reuse: number[] = [];
    let consume = 0;
    const measure = (kind: "full" | "reuse") => {
      const start = performance.now(),
        scene =
          kind === "full" || process.env.MEDEN_PERF_MUTANT === "full-layout"
            ? projectMap(changed)
            : projectMap(changed, new Set(), geometry, before);
      consume += scene.areas.length + scene.symbols.size;
      return performance.now() - start;
    };
    for (let i = 0; i < 9; i++)
      for (const kind of i % 2
        ? (["reuse", "full"] as const)
        : (["full", "reuse"] as const))
        (kind === "full" ? full : reuse).push(measure(kind));
    const median = (xs: number[]) =>
        [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)]!,
      ratio = median(reuse) / median(full);
    const packed = packGeometry(geometry, input),
      start = performance.now();
    const restored = unpackGeometry(packed, input);
    const unpack_ms = performance.now() - start;
    expect(restored).toEqual(geometry);
    const output = process.env.MEDEN_PERF_OUT || "target/performance";
    await mkdir(output, { recursive: true });
    await writeFile(
      output + "/map-overlay.json",
      JSON.stringify(
        {
          files: data.files.length,
          repetitions: 9,
          full_ms: full,
          reuse_ms: reuse,
          ratio,
          maximum_ratio: 0.5,
          packed_bytes: packed.byteLength,
          unpack_ms,
          first_input_ms: inputMs,
          first_geometry_ms: geometryMs,
          first_scene_ms: sceneMs,
          consume,
          scope:
            "Same process, paired current-algorithm recomputation versus prepared geometry overlay; no browser paint or native compilation measurement.",
        },
        null,
        2,
      ),
    );
    expect(ratio).toBeLessThan(0.5);
  }, 60000);
