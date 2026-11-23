import { expect, test } from "vitest";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";
import {
  declarationDetail,
  fade,
  mapHasDeclarations,
  mapLocation,
  projectMap,
  visibleMapAreas,
} from "./project-map-model";

test("branch pruning preserves the visible areas and opacity of a full scan", () => {
  const files: WorkspaceOverview["files"] = Array.from(
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
  const scene = projectMap({
    generation: "0",
    files,
    excluded: [],
    inventory_only: false,
  });
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
      const expected = scene.areas.flatMap((area) => {
        const parent = scene.byPath.get(area.parent);
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

test("the legend follows visible declarations rather than distant or empty files", () => {
  const scene = projectMap({
    generation: "0",
    inventory_only: false,
    excluded: [],
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
  });
  const size = { width: 400, height: 300 };
  for (const path of ["Empty.lean", "Proof.lean"]) {
    const area = scene.byPath.get(path)!;
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
  const scene = projectMap({
    generation: "0",
    inventory_only: false,
    excluded: [],
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
  });
  const parent = scene.byPath.get("A")!,
    child = scene.byPath.get("A/Inner")!;
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
