import fc from "fast-check";
import { mkdir, writeFile } from "node:fs/promises";
import { cpus, platform, release } from "node:os";
import { expect, test } from "vitest";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";
import {
  declarationDetail,
  fade,
  projectMap,
  visibleMapAreas,
  type MapScene,
} from "./project-map-model";

// Deliberately scan every area, without the production parent/children index.
export function scan(
  scene: MapScene,
  view: { x: number; y: number; zoom: number },
  size: { width: number; height: number },
) {
  const opacity = new Map([["", 1]]);
  const left = -view.x / view.zoom,
    top = -view.y / view.zoom;
  return scene.areas.flatMap((area) => {
    const parent = scene.byPath.get(area.parent);
    const alpha = parent
      ? opacity.get(parent.path)! *
        fade(Math.max(parent.width, parent.height) * view.zoom, 650, 950)
      : 1;
    opacity.set(area.path, alpha);
    if (
      alpha < 0.015 ||
      area.x > left + size.width / view.zoom ||
      area.y > top + size.height / view.zoom ||
      area.x + area.width < left ||
      area.y + area.height < top ||
      ((area.width * view.zoom < 2 || area.height * view.zoom < 2) &&
        !(area.main && declarationDetail(area, view.zoom, size) > 0.01))
    )
      return [];
    return [{ area, opacity: alpha }];
  });
}

export function corpus(
  rows: Array<{ folders: number[]; count: number; main: boolean }>,
): WorkspaceOverview {
  return {
    revision: "generated-corpus-v1",
    excluded: [],
    inventory_only: false,
    files: rows.map((row, index) => ({
      path: [...row.folders.map((n) => `D${n}`), `F${index}.lean`].join("/"),
      sha256: "fixture",
      bytes: 100,
      declarations: row.count,
      theorems: row.count,
      indexed: true,
      imports: [],
      symbols: Array.from({ length: row.count }, (_, i) => ({
        name: `t${index}_${i}`,
        line: i + 1,
        kind: "theorem",
        declaration: "theorem",
        private: false,
        references: i ? [`t${index}_0`] : [`t${index}_${row.count - 1}`],
        semantic: null,
        main_theorem: row.main && i === 0,
      })),
    })),
  };
}

const row = fc.record({
  folders: fc.array(fc.integer({ min: 0, max: 9 }), { maxLength: 5 }),
  count: fc.integer({ min: 0, max: 8 }),
  main: fc.boolean(),
});
const view = fc.record({
  x: fc.integer({ min: -50000, max: 10000 }),
  y: fc.integer({ min: -50000, max: 10000 }),
  zoom: fc.double({ min: 0.001, max: 500, noNaN: true }),
});
const size = fc.record({
  width: fc.integer({ min: 1, max: 2500 }),
  height: fc.integer({ min: 1, max: 2000 }),
});
const seed = Number(process.env.MEDEN_FUZZ_SEED || 20261123);
const runs = Number(process.env.MEDEN_FUZZ_RUNS || 150);
if (
  !Number.isSafeInteger(seed) ||
  !Number.isSafeInteger(runs) ||
  runs < 1 ||
  runs > 100000
)
  throw new Error("Invalid fuzz seed/run count");

test("generated camera traces agree with a full scan and never change world coordinates", () => {
  fc.assert(
    fc.property(
      fc.array(row, { maxLength: 90 }),
      fc.array(fc.record({ view, size }), { minLength: 1, maxLength: 25 }),
      (rows, moves) => {
        const input = corpus(rows),
          scene = projectMap(input);
        const coordinates = scene.areas.map(({ x, y, width, height }) => ({
          x,
          y,
          width,
          height,
        }));
        for (const { view, size } of moves) {
          expect(visibleMapAreas(scene, view, size)).toEqual(
            scan(scene, view, size),
          );
          expect(
            scene.areas.map(({ x, y, width, height }) => ({
              x,
              y,
              width,
              height,
            })),
          ).toEqual(coordinates);
        }
        expect(projectMap(input).areas).toEqual(scene.areas);
      },
    ),
    { seed, numRuns: runs, path: process.env.MEDEN_FUZZ_PATH, verbose: true },
  );
});

if (process.env.MEDEN_PERFORMANCE === "1") {
  test("large-map visibility stays below 35% of the full-scan baseline", async () => {
    const input = corpus(
      Array.from({ length: 175187 }, (_, i) => ({
        folders: [i % 24, Math.floor(i / 24) % 12],
        count: 0,
        main: false,
      })),
    );
    const start = performance.now();
    const scene = projectMap(input);
    const layout = performance.now() - start;
    const camera = { x: 0, y: 0, zoom: 0.08 },
      size = { width: 900, height: 700 };
    const optimized: number[] = [],
      reference: number[] = [];
    // Negative control: verify that the budget rejects the retired algorithm.
    const candidate =
      process.env.MEDEN_PERF_MUTANT === "full-scan" ? scan : visibleMapAreas;
    const measure = (operation: () => unknown) => {
      const start = performance.now();
      for (let n = 0; n < 10; n++) operation();
      return performance.now() - start;
    };
    // Warm both implementations, then alternate their order to reduce bias.
    expect(visibleMapAreas(scene, camera, size)).toEqual(
      scan(scene, camera, size),
    );
    for (let i = 0; i < 9; i++) {
      const fast = () =>
        optimized.push(measure(() => candidate(scene, camera, size)));
      const slow = () =>
        reference.push(measure(() => scan(scene, camera, size)));
      if (i % 2) {
        fast();
        slow();
      } else {
        slow();
        fast();
      }
    }
    const median = (values: number[]) =>
      [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]!;
    const ratio = median(optimized) / median(reference);
    const destination = process.env.MEDEN_PERF_OUT || "target/performance";
    await mkdir(destination, { recursive: true });
    await writeFile(
      `${destination}/map.json`,
      JSON.stringify(
        {
          workload: {
            version: 1,
            files: input.files.length,
            camera,
            size,
            repetitions: 9,
            calls_per_sample: 10,
          },
          environment: {
            node: process.version,
            os: platform(),
            kernel: release(),
            cpu: cpus()[0]?.model,
          },
          layout_ms: layout,
          optimized_ms: optimized,
          full_scan_ms: reference,
          median_ratio: ratio,
          maximum_ratio: 0.35,
          scope:
            "Same-process comparative algorithm gate, not browser paint/INP or a native compilation budget. OS caches are warm; nine alternating paired measurements. Absolute timings are diagnostic only.",
        },
        null,
        2,
      ),
    );
    expect(ratio).toBeLessThan(0.35);
  }, 60000);
}
