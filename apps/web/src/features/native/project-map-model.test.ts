import { expect, test } from "vitest";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";
import {
  declarationDetail,
  fade,
  projectMap,
  visibleMapAreas,
} from "./project-map-model";

test("branch pruning preserves the visible areas and opacity of a full scan", () => {
  const files: WorkspaceOverview["files"] = Array.from(
    { length: 1200 },
    (_, i) => ({
      path: `src/Group${i % 12}/Part${i % 7}/File${i}.lean`,
      sha256: "fixture",
      bytes: 100,
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
    revision: "fixture",
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
