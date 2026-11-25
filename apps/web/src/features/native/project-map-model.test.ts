import { prepareMap, mapArea } from "./project-map-model";
import { expect, test, vi } from "vitest";
import { scan } from "./project-map-oracle";
import type { NavigationInventory } from "./project-map-oracle";
import {
  declarationDetail,
  fade,
  mapHasDeclarations,
  mapAreaBatches,
  mapLocation,
  projectMap,
  visibleMapAreas,
} from "./project-map-model";

test("independent viewport consumers reuse traversal while size and scene changes keep original visibility", () => {
  const input: NavigationInventory = {
    generation: "original",
    excluded: [],
    inventory_only: false,
    files: Array.from({ length: 512 }, (_, i) => ({
      path: `Source/Part${i % 8}/File${i}.lean`,
      sha256: "original",
      bytes: "20",
      declarations: 1,
      theorems: 1,
      indexed: true,
      imports: [],
      symbols: [
        {
          name: `claim${i}`,
          line: 1,
          declaration: "theorem",
          kind: "theorem",
          private: false,
          references: [],
          semantic: null,
          main_theorem: false,
        },
      ],
    })),
  };
  const index = prepareMap(input),
    scene = projectMap(index);
  const a = { x: 0, y: 0, zoom: 0.08 },
    b = { x: -400, y: -300, zoom: 0.12 };
  const size = { width: 900, height: 700 };
  const visits = vi.spyOn(scene.descriptions, "get");
  const first = visibleMapAreas(scene, a, size),
    firstVisits = visits.mock.calls.length;
  expect(firstVisits).toBeGreaterThan(0);
  visibleMapAreas(scene, { ...a }, { ...size });
  visibleMapAreas(scene, { ...a }, { ...size });
  expect(visits.mock.calls.length).toBe(firstVisits);
  visibleMapAreas(scene, b, size);
  const bothVisits = visits.mock.calls.length;
  expect(bothVisits).toBeGreaterThan(firstVisits);
  visibleMapAreas(scene, { ...a }, { ...size });
  expect(visits.mock.calls.length).toBe(bothVisits);
  const resized = visibleMapAreas(scene, a, { width: 300, height: 200 });
  expect(visits.mock.calls.length).toBeGreaterThan(bothVisits);
  visits.mockRestore();
  expect(first).toEqual(scan(scene, a, size));
  expect(resized).toEqual(scan(scene, a, { width: 300, height: 200 }));
  const updated = projectMap(index, new Set(["claim0"]));
  expect(visibleMapAreas(updated, a, size)).toEqual(scan(updated, a, size));
  expect(visibleMapAreas(scene, a, size)).toEqual(scan(scene, a, size));
});

test("branch pruning preserves the visible areas and opacity of a full scan", () => {
  const files: NavigationInventory["files"] = Array.from(
    { length: 1200 },
    (_, i) => ({
      path: `src/Group${i % 12}/Part${i % 7}/File${i}.lean`,
      sha256: "fixture",
      bytes: "100",
      declarations: 1,
      theorems: 1,
      indexed: true,
      imports: [],
      symbols: [
        {
          name: `result${i}`,
          line: 1,
          kind: "theorem",
          declaration: "theorem",
          private: false,
          references: [],
          semantic: null,
          main_theorem: i % 19 === 0,
        },
      ],
    }),
  );
  const scene = projectMap(
    prepareMap({
      files,
      inventory_only: false,
    }),
  );
  for (const zoom of [0.03, 0.08, 0.5, 2, 10, 100])
    for (const [x, y] of [
      [0, 0],
      [-100, -500],
      [-4000, -2400],
      [-12000, -8000],
    ]) {
      const size = { width: 900, height: 700 };
      const left = -x! / zoom,
        top = -y! / zoom;
      // Independent full-scan oracle: preserve every previous visibility rule.
      const opacity = new Map([["", 1]]);
      const expected = [...scene.index.byPath.keys()]
        .map((path) => mapArea(scene, path)!)
        .flatMap((area) => {
          const parent = mapArea(scene, area.parent);
          const alpha = parent
            ? opacity.get(parent.path)! *
              fade(Math.max(parent.width, parent.height) * zoom, 650, 950)
            : 1;
          opacity.set(area.path, alpha);
          if (
            alpha < 0.015 ||
            area.x > left + size.width / zoom ||
            area.y > top + size.height / zoom ||
            area.x + area.width < left ||
            area.y + area.height < top ||
            ((area.width * zoom < 2 || area.height * zoom < 2) &&
              !(area.main && declarationDetail(area, zoom, size) > 0.01))
          )
            return [];
          return [{ area, opacity: alpha }];
        });
      expect(visibleMapAreas(scene, { x: x!, y: y!, zoom }, size)).toEqual(
        expected,
      );
    }
});

test("large-directory batches retain original painter order and file identity across camera changes", () => {
  const scene = projectMap(
    prepareMap({
      inventory_only: true,
      files: Array.from({ length: 400 }, (_, i) => ({
        path: `src/F${i}.lean`,
        sha256: "fixture",
        bytes: "100",
        declarations: 0,
        theorems: 0,
        indexed: false,
        imports: [],
        symbols: [],
      })),
    }),
  );
  const size = { width: 1200, height: 800 },
    identities = new Map<string, string>();
  let revisited = 0;
  for (const camera of [
    { x: 0, y: 0, zoom: 0.1 },
    { x: -700, y: -400, zoom: 0.4 },
    { x: -100, y: -500, zoom: 0.2 },
  ]) {
    const visible = visibleMapAreas(scene, camera, size),
      batches = mapAreaBatches(visible);
    expect(batches.flatMap((batch) => batch.visible)).toEqual(visible);
    expect(new Set(batches.map((batch) => batch.key)).size).toBe(
      batches.length,
    );
    for (const batch of batches) {
      expect(batch.visible.length).toBeLessThanOrEqual(128);
      for (const { area } of batch.visible) {
        const prior = identities.get(area.path);
        if (prior) {
          expect(batch.key).toBe(prior);
          revisited++;
        }
        identities.set(area.path, batch.key);
      }
    }
  }
  expect(identities.size).toBe(400);
  expect(revisited).toBeGreaterThan(0);
});

test("the legend follows visible declarations rather than distant or empty files", () => {
  const scene = projectMap(
    prepareMap({
      inventory_only: false,
      files: ["Empty.lean", "Proof.lean"].map((path, i) => ({
        path,
        sha256: "fixture",
        bytes: "100",
        declarations: i,
        theorems: i,
        imports: [],
        indexed: true,
        symbols: i
          ? [
              {
                name: "result",
                line: 1,
                kind: "theorem",
                declaration: "theorem",
                private: false,
                references: [],
                semantic: null,
                main_theorem: false,
              },
            ]
          : [],
      })),
    }),
  );
  const size = { width: 400, height: 300 };
  for (const path of ["Empty.lean", "Proof.lean"]) {
    const area = mapArea(scene, path)!;
    const camera = {
      x: size.width / 2 - area.x - area.width / 2,
      y: size.height / 2 - area.y - area.height / 2,
      zoom: 1,
    };
    expect(
      mapHasDeclarations(visibleMapAreas(scene, camera, size), 1, size),
    ).toBe(path === "Proof.lean");
  }
  const outside = { x: 200000, y: 200000, zoom: 1 };
  expect(
    mapHasDeclarations(visibleMapAreas(scene, outside, size), 1, size),
  ).toBe(false);
});

test("breadcrumbs enter a containing child only after its opacity exceeds one half", () => {
  const scene = projectMap(
    prepareMap({
      inventory_only: false,
      files: ["A/Inner/First.lean", "A/Second.lean", "B/Third.lean"].map(
        (path) => ({
          path,
          sha256: "fixture",
          bytes: "100",
          declarations: 0,
          theorems: 0,
          imports: [],
          indexed: true,
          symbols: [],
        }),
      ),
    }),
  );
  const parent = mapArea(scene, "A")!,
    child = mapArea(scene, "A/Inner")!;
  const size = { width: 400, height: 300 };
  for (const pixels of [749.999, 750, 750.001]) {
    const zoom = pixels / Math.min(parent.width, parent.height);
    const view = {
      x: size.width / 2 - (child.x + child.width / 2) * zoom,
      y: size.height / 2 - (child.y + child.height / 2) * zoom,
      zoom,
    };
    expect(mapLocation(scene, view, size)).toBe(pixels > 750 ? "A/Inner" : "A");
  }
});
