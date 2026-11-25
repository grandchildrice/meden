import { test, expect } from "@playwright/test";
import { installAreaPaintObserver } from "../../../tools/gui/area-paint-observer.mjs";

test("opt-in area observations distinguish command/font changes and expose truncation", async ({
  page,
}) => {
  await page.goto("about:blank");
  await page.setContent(
    '<canvas class="map-area-canvas" width="4" height="4"></canvas><canvas id="other"></canvas>',
  );
  await page.evaluate(installAreaPaintObserver);
  await page.evaluate(() => {
    const surface = document
      .querySelector("canvas")!
      .transferControlToOffscreen();
    const context = surface.getContext("2d")!;
    (window as any).__observationSurface = surface;
    const draw = (color: string) => {
      context.clearRect(0, 0, 4, 4);
      context.beginPath();
      context.roundRect(0, 0, 4, 4, 1);
      context.fillStyle = color;
      context.fill();
      context.strokeStyle = "#fff";
      context.lineWidth = 0.6;
      context.stroke();
    };
    draw("#123456");
    draw("#123456");
    draw("#456789");
    document.fonts.dispatchEvent(new Event("loadingdone"));
    draw("#456789");
    const other = document
      .querySelector<HTMLCanvasElement>("#other")!
      .transferControlToOffscreen()
      .getContext("2d")!;
    other.clearRect(0, 0, 4, 4);
    other.roundRect(0, 0, 4, 4, 1);
    other.fill();
  });
  const observations = await page.evaluate(
    () => (window as any).__medenAreaPaints,
  );
  expect(observations.truncated).toBe(false);
  expect(observations.frames).toHaveLength(4);
  expect(observations.frames.map((r: any) => r.areas)).toEqual([1, 1, 1, 1]);
  expect(observations.frames.map((r: any) => r.identical_to_previous)).toEqual([
    false,
    true,
    false,
    false,
  ]);
  await page.evaluate(() => {
    const surface = (window as any).__observationSurface as OffscreenCanvas;
    const context = surface.getContext("2d")!;
    for (let i = 0; i < 10002; i++) context.clearRect(0, 0, 4, 4);
  });
  const bounded = await page.evaluate(() => (window as any).__medenAreaPaints);
  expect(bounded.truncated).toBe(true);
  expect(bounded.frames).toHaveLength(10000);
});
