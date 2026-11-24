import { expect, test } from "vitest";
import fc from "fast-check";
import type { NavigationInventory } from "./project-map-oracle";
import {
  prepareMap,
  projectMap,
  mapArea,
  visibleMapAreas,
  mapLocation,
  navigationReady,
  type MapFile,
  type MapScene,
} from "./project-map-model";
import { scan, scanLocation } from "./project-map-oracle";

function fixture(count: number, inventoryOnly = true): NavigationInventory {
  return {
    generation: "0",
    excluded: [],
    inventory_only: inventoryOnly,
    files: Array.from({ length: count }, (_, i) => ({
      path: `src/Group${i % 3}/F${i}.lean`,
      sha256: "original",
      bytes: "10",
      declarations: 0,
      theorems: 0,
      imports: [],
      indexed: false,
      symbols: [],
    })),
  };
}
function observe(file: MapFile, count: number, step: number, hidden = false) {
  return {
    path: file.path,
    imports: "imports" in file ? file.imports : [],
    sha256: "observation-" + step,
    bytes: String(10 + count),
    indexed: true,
    declarations: count,
    theorems: count,
    symbols: Array.from({ length: count }, (_, i) => ({
      name: `N.t${i}`,
      line: step + i + 1,
      kind: "theorem" as const,
      declaration: i % 2 ? "lemma" : "theorem",
      private: hidden,
      references: [`t${(i + 1) % Math.max(1, count)}`],
      semantic: null,
      main_theorem: i === 0 && step % 3 === 0,
    })),
  };
}
function outcome(scene: MapScene) {
  return {
    areas: [...scene.index.byPath.keys()].map((path) => mapArea(scene, path)),
    symbols: [...scene.symbols],
    edges: scene.edges,
    edgesByFile: [...scene.edgesByFile],
    pins: scene.mainPins,
    maxZoom: scene.maxZoom,
  };
}

test("finite file/target update traces match fresh complete projection without changing earlier scenes", () => {
  fc.assert(
    fc.property(
      fc.integer({ min: 1, max: 14 }),
      fc.boolean(),
      fc.boolean(),
      fc.array(
        fc.record({
          file: fc.nat(100),
          count: fc.integer({ min: 0, max: 4 }),
          hidden: fc.boolean(),
          target: fc.boolean(),
          reuse: fc.boolean(),
        }),
        { minLength: 1, maxLength: 35 },
      ),
      (count, inventoryOnly, summarized, updates) => {
        const source = fixture(count, inventoryOnly);
        const original = {
            ...source,
            files: source.files.map((file, i): MapFile => {
              if (!summarized) return file;
              if (i % 3 === 0)
                return {
                  state: "unindexed",
                  path: file.path,
                  bytes: file.bytes,
                };
              const {
                symbols,
                indexed: _,
                ...metadata
              } = observe(file, 2, i, i % 2 === 0);
              return {
                state: "indexed",
                ...metadata,
                headers: symbols.map(
                  ({ references: _, semantic: __, ...header }) => header,
                ),
              };
            }),
          },
          initial = JSON.stringify(original),
          index = prepareMap(original);
        let scene = projectMap(index),
          current = original.files.slice();
        const cumulative = new Map<string, MapFile>();
        for (const [step, event] of updates.entries()) {
          const before = JSON.stringify(outcome(scene)),
            previous = scene,
            i = event.file % count;
          const file = event.reuse
            ? current[i]!
            : observe(current[i]!, event.count, step, event.hidden);
          current = current.map((old, j) => (j === i ? file : old));
          cumulative.set(file.path, file);
          const tracked = new Set(event.target ? ["N.t1"] : []);
          scene = projectMap(index, tracked, cumulative, scene);
          const full = projectMap(
            prepareMap({ ...original, files: current }, index.geometry),
            tracked,
          );
          expect(outcome(scene)).toEqual(outcome(full));
          expect(JSON.stringify(outcome(previous))).toBe(before);
          expect(scene.index).toBe(index);
          expect(scene.index.children).toBe(index.children);
          const view = { x: -250, y: -100, zoom: 0.3 },
            size = { width: 900, height: 700 };
          expect(visibleMapAreas(scene, view, size)).toEqual(
            scan(scene, view, size),
          );
          expect(mapLocation(scene, view, size)).toBe(
            scanLocation(scene, view, size),
          );
        }
        expect(JSON.stringify(original)).toBe(initial);
      },
    ),
    { seed: 20261125, numRuns: Number(process.env.MEDEN_FUZZ_RUNS || 150) },
  );
}, 60000);

test("role-only updates reuse reference edges and a rebase cannot inherit an old observation", () => {
  const original = fixture(2),
    index = prepareMap(original),
    file = observe(original.files[0]!, 2, 1),
    files = new Map([[file.path, file]]);
  const before = projectMap(index, new Set(), files),
    after = projectMap(index, new Set(["N.t1"]), files, before);
  expect(after.edges).toBe(before.edges);
  expect(after.edgesByFile).toBe(before.edgesByFile);
  expect(outcome(after)).toEqual(
    outcome(
      projectMap(
        prepareMap(
          { ...original, files: [file, original.files[1]!] },
          index.geometry,
        ),
        new Set(["N.t1"]),
      ),
    ),
  );
  const rebased = projectMap(
    prepareMap(original, index.geometry),
    new Set(),
    new Map(),
    after,
  );
  expect(rebased.descriptions.size).toBe(0);
  expect(rebased.symbols.size).toBe(0);
  expect(mapArea(rebased, file.path)!.file).toBe(original.files[0]);
});

test("empty/repeated observations and out-of-inventory dependencies do not alter the inventory or leak stale symbols", () => {
  const original = fixture(2),
    index = prepareMap(original),
    first = observe(original.files[0]!, 2, 0);
  const before = projectMap(index, new Set(), new Map([[first.path, first]]));
  expect(
    projectMap(index, new Set(), new Map([[first.path, first]]), before),
  ).toBe(before);
  const empty = observe(first, 0, 1),
    foreign = { ...first, path: "outside/Foreign.lean" };
  const cleared = projectMap(
    index,
    new Set(),
    new Map([
      [empty.path, empty],
      [foreign.path, foreign],
    ]),
    before,
  );
  expect(cleared.symbols.size).toBe(0);
  expect(cleared.edges).toEqual([]);
  expect(cleared.descriptions.size).toBe(1);
  expect(mapArea(cleared, foreign.path)).toBeUndefined();
  expect(before.symbols.size).toBe(2);
  expect(() =>
    projectMap(index, new Set(), new Map([["wrong", first]]), before),
  ).toThrow("does not match its path");
  expect(index.snapshot).toBe(original);
});

test("overview headings retain all roles and positions while complete cached navigation arrives separately", () => {
  const source = fixture(2, false);
  source.files = source.files.map((file, i) => observe(file, 2, i));
  const summary = {
    inventory_only: source.inventory_only,
    files: source.files.map(({ symbols, indexed: _, ...file }) => ({
      state: "indexed" as const,
      ...file,
      headers: symbols.map(
        ({ references: _, semantic: __, ...header }) => header,
      ),
    })),
  };
  const index = prepareMap(summary),
    initial = projectMap(index),
    complete = projectMap(prepareMap(source, index.geometry));
  expect(initial.edges).toEqual([]);
  expect(
    [...initial.symbols.values()].map(
      ({ references: _, semantic: __, ...s }) => s,
    ),
  ).toEqual(
    [...complete.symbols.values()].map(
      ({ references: _, semantic: __, ...s }) => s,
    ),
  );
  expect(navigationReady(mapArea(initial, source.files[0]!.path)!.file!)).toBe(
    false,
  );
  const files = new Map(source.files.map((file) => [file.path, file]));
  const loaded = projectMap(index, new Set(), files, initial);
  expect(outcome(loaded)).toEqual(outcome(complete));
  expect(navigationReady(mapArea(loaded, source.files[0]!.path)!.file!)).toBe(
    true,
  );
  // A completed response may explicitly report an unreadable source.
  expect(navigationReady(fixture(1).files[0]!)).toBe(true);
  expect(initial.edges).toEqual([]);
});
