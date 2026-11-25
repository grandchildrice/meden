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
import { scan, scanLocation, scanReferences } from "./project-map-oracle";

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

test("dense-directory details and tracked targets keep exact visibility through original and replaced indices", () => {
  fc.assert(
    fc.property(
      fc.integer({ min: 65, max: 170 }),
      fc.array(
        fc.record({
          file: fc.nat(300),
          declarations: fc.integer({ min: 0, max: 5 }),
          tracked: fc.boolean(),
          replace: fc.boolean(),
        }),
        { minLength: 1, maxLength: 8 },
      ),
      (count, changes) => {
        const input = fixture(count);
        input.files = input.files.map((file, i) => ({
          ...file,
          path: `Flat/F${i}.lean`,
        }));
        let index = prepareMap(input),
          scene = projectMap(index);
        const world = JSON.stringify(index.geometry);
        const files = new Map<string, MapFile>(),
          size = { width: 900, height: 700 };
        changes.forEach((change, step) => {
          const before = scene,
            selected = input.files[change.file % count]!,
            observation = observe(selected, change.declarations, step),
            tracked = change.tracked ? new Set(["N.t0"]) : new Set<string>();
          files.set(selected.path, observation);
          if (change.replace) index = prepareMap(input);
          scene = projectMap(index, tracked, files, scene);
          const area = mapArea(scene, selected.path)!,
            zoom = Math.min(size.width / area.width, size.height / area.height),
            cameras = [
              { x: 0, y: 0, zoom: 0.08 },
              {
                x: size.width / 2 - (area.x + area.width / 2) * zoom,
                y: size.height / 2 - (area.y + area.height / 2) * zoom,
                zoom,
              },
              { x: -area.x * zoom, y: -area.y * zoom, zoom },
            ];
          for (const camera of cameras) {
            expect(visibleMapAreas(scene, camera, size)).toEqual(
              scan(scene, camera, size),
            );
            expect(mapLocation(scene, camera, size)).toBe(
              scanLocation(scene, camera, size),
            );
            expect(visibleMapAreas(before, camera, size)).toEqual(
              scan(before, camera, size),
            );
          }
          expect(JSON.stringify(index.geometry)).toBe(world);
        });
      },
    ),
    { seed: 20261125, numRuns: Number(process.env.MEDEN_FUZZ_RUNS || 100) },
  );
}, 60000);

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
          sameIdentity: fc.boolean(),
          rename: fc.boolean(),
          references: fc.array(
            fc.constantFrom(
              "t0",
              "t1",
              "N.t0",
              "N.t1",
              "M.x",
              "Ω.x",
              "missing",
            ),
            { maxLength: 8 },
          ),
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
          const old = current[i]!;
          const oldSymbols =
            "headers" in old
              ? old.headers
              : "symbols" in old
                ? old.symbols
                : [];
          const observation = observe(old, event.count, step, event.hidden);
          const updated = event.sameIdentity
            ? {
                ...observation,
                declarations: oldSymbols.length,
                theorems: oldSymbols.length,
                symbols: oldSymbols.map((symbol) => ({
                  ...symbol,
                  references: event.references,
                  semantic: null,
                })),
              }
            : {
                ...observation,
                symbols: observation.symbols.map((symbol, n) => ({
                  ...symbol,
                  name: event.rename
                    ? ["M.x", "Ω.x", "N.t0", "N.t1"][n % 4]!
                    : symbol.name,
                  references: event.references,
                })),
              };
          const file = event.reuse ? old : updated;
          current = current.map((old, j) => (j === i ? file : old));
          cumulative.set(file.path, file);
          const tracked = new Set(event.target ? ["N.t1"] : []);
          scene = projectMap(index, tracked, cumulative, scene);
          const full = projectMap(
            prepareMap({ ...original, files: current }, index.geometry),
            tracked,
          );
          expect(outcome(scene)).toEqual(outcome(full));
          expect(scene.edges).toEqual(scanReferences(scene.symbols));
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

test("details reuse the same target universe, while additions, ambiguity, privacy and renames invalidate earlier readings", () => {
  const source = fixture(3, false);
  source.files = source.files.map((file, i) => ({
    ...observe(file, 1, 0),
    symbols: [
      {
        name: i === 0 ? "N.user" : i === 1 ? "N.target" : "N.other",
        line: 1,
        kind: "theorem" as const,
        declaration: "theorem",
        private: false,
        references: i === 0 ? ["target", "missing"] : [],
        semantic: null,
        main_theorem: false,
      },
    ],
  }));
  const summary = {
    inventory_only: false,
    files: source.files.map(({ symbols, indexed: _, ...file }) => ({
      state: "indexed" as const,
      ...file,
      headers: symbols.map(
        ({ references: _, semantic: __, ...header }) => header,
      ),
    })),
  };
  const index = prepareMap(summary),
    files = new Map<string, MapFile>();
  let scene = projectMap(index);
  const initialIndex = scene.references;
  for (const file of source.files) {
    files.set(file.path, file);
    const next = projectMap(index, new Set(), files, scene);
    expect(next.references).toBe(initialIndex);
    expect(next.edges).toEqual(scanReferences(next.symbols));
    scene = next;
  }
  expect(scene.edges.length).toBeGreaterThan(0);
  for (const [name, hidden] of [
    ["N.target", false],
    ["N.target", true],
    ["N.missing", false],
    ["N.other", false],
  ] as const) {
    const previous = scene,
      retained = JSON.stringify([...scene.symbols]),
      original = source.files[2]!,
      file = {
        ...original,
        symbols: [{ ...original.symbols[0]!, name, private: hidden }],
      };
    files.set(file.path, file);
    scene = projectMap(index, new Set(), files, scene);
    expect(scene.references).not.toBe(previous.references);
    expect(scene.edges).toEqual(scanReferences(scene.symbols));
    expect(JSON.stringify([...previous.symbols])).toBe(retained);
    const targets = new Set(["N.user"]),
      changedRole = projectMap(index, targets, files, scene);
    expect(changedRole.references).toBe(scene.references);
    expect(changedRole.edges).toBe(scene.edges);
    scene = changedRole;
  }
});
