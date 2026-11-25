import { expect, test } from "vitest";
import fc from "fast-check";
import { OrderedBounds } from "./ordered-bounds";

test("overlapping and repeated rectangles retain exact input order at closed edges and points", () => {
  fc.assert(
    fc.property(
      fc.array(
        fc.record({
          x: fc.integer({ min: -1000, max: 1000 }),
          y: fc.integer({ min: -1000, max: 1000 }),
          width: fc.integer({ min: 1, max: 300 }),
          height: fc.integer({ min: 1, max: 300 }),
        }),
        { maxLength: 300 },
      ),
      fc.integer({ min: -1200, max: 1200 }),
      fc.integer({ min: -1200, max: 1200 }),
      fc.integer({ min: 0, max: 400 }),
      fc.integer({ min: 0, max: 400 }),
      (input, left, top, width, height) => {
        const rectangles = [...input, ...input.slice(0, 2)],
          original = rectangles.map((area) => ({ ...area })),
          index = new OrderedBounds(rectangles);
        for (const bounds of [
          { left, top, right: left + width, bottom: top + height },
          { left, top, right: left, bottom: top },
          ...rectangles.slice(0, 3).map((r) => ({
            left: r.x + r.width,
            top: r.y + r.height,
            right: r.x + r.width,
            bottom: r.y + r.height,
          })),
        ]) {
          const selected: typeof rectangles = [];
          index.visit(bounds, (r) => selected.push(r));
          const expected = rectangles.filter(
            (r) =>
              r.x <= bounds.right &&
              r.y <= bounds.bottom &&
              r.x + r.width >= bounds.left &&
              r.y + r.height >= bounds.top,
          );
          expect(selected).toEqual(expected);
          selected.forEach((item, i) => expect(item).toBe(expected[i]));
        }
        expect(rectangles).toEqual(original);
      },
    ),
    { numRuns: Number(process.env.MEDEN_FUZZ_RUNS || 100), seed: 20261125 },
  );
});

test("a narrow view does not read every hidden item in a dense ordered directory", () => {
  let reads = 0;
  const rectangles = Array.from({ length: 30000 }, (_, i) => ({
    get x() {
      reads++;
      return i % 100;
    },
    y: Math.floor(i / 100),
    width: 0.95,
    height: 0.95,
  }));
  const index = new OrderedBounds(rectangles);
  reads = 0;
  const selected: typeof rectangles = [];
  index.visit({ left: 45.2, top: 45.2, right: 45.4, bottom: 45.4 }, (item) =>
    selected.push(item),
  );
  expect(selected).toEqual([rectangles[4545]]);
  expect(reads).toBeLessThan(300);
});
