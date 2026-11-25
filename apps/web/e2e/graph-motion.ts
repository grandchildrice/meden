import {
  expect,
  type Locator,
  type Page,
  type TestInfo,
} from "@playwright/test";

/** Observe real camera changes while alternating node/background pointer input.
 * This never sets a viewport or dispatches synthetic DOM wheel events.
 */
export async function zoomAcrossNode(
  page: Page,
  graph: Locator,
  node: Locator,
  info: TestInfo,
) {
  await graph.evaluate((element) => {
    const values: number[] = [];
    const viewports = new Set<Element>();
    let frame = 0,
      truncated = false;
    const sample = () => {
      const viewport = element.querySelector(".react-flow__viewport");
      if (viewport) {
        viewports.add(viewport);
        if (values.length < 1024)
          values.push(new DOMMatrix(getComputedStyle(viewport).transform).a);
        else truncated = true;
      }
      frame = requestAnimationFrame(sample);
    };
    sample();
    let enters = 0;
    const enter = (event: Event) => {
      if (
        event.target instanceof Element &&
        event.target.matches(".root-node > button")
      )
        enters++;
    };
    element.addEventListener("pointerenter", enter, true);
    (window as any).__mixedGraphZoom = () => {
      cancelAnimationFrame(frame);
      element.removeEventListener("pointerenter", enter, true);
      return {
        scales: values,
        node_entries: enters,
        viewport_elements: viewports.size,
        truncated,
      };
    };
  });
  const bounds = (await graph.boundingBox())!;
  const transform = () =>
    graph.evaluate((element) => {
      const viewport = element.querySelector(
        ".react-flow__viewport",
      ) as HTMLElement;
      const matrix = new DOMMatrix(viewport.style.transform);
      return { x: matrix.e, y: matrix.f, zoom: matrix.a };
    });
  type Camera = Awaited<ReturnType<typeof transform>>;
  const probes: Array<{
    before: Camera;
    after: Camera;
    anchor_error_px: number;
  }> = [];
  const wheel = async (x: number, y: number, delta: number) => {
    const before = await transform();
    await page.mouse.move(x, y);
    const flow = (await graph.locator(".react-flow").boundingBox())!;
    await page.mouse.wheel(0, delta);
    await page.evaluate(async () => {
      for (let i = 0; i < 2; i++) await new Promise(requestAnimationFrame);
    });
    const after = await transform();
    const ratio = after.zoom / before.zoom;
    const px = x - flow.x,
      py = y - flow.y;
    probes.push({
      before,
      after,
      anchor_error_px: Math.hypot(
        after.x - (px - (px - before.x) * ratio),
        after.y - (py - (py - before.y) * ratio),
      ),
    });
  };
  try {
    for (const pinch of [false, true]) {
      if (pinch) await page.keyboard.down("Control");
      for (let i = 0; i < 8; i++) {
        const target = (await node.boundingBox())!;
        const delta = pinch ? -2 : -25;
        await wheel(
          target.x + target.width / 2,
          target.y + target.height / 2,
          delta,
        );
        await wheel(bounds.x + bounds.width - 8, bounds.y + 8, delta);
      }
    }
  } finally {
    await page.keyboard.up("Control");
  }
  // Include D3's wheel-end notification and any delayed hover rendering.
  await page.waitForTimeout(350);
  const result: {
    scales: number[];
    node_entries: number;
    viewport_elements: number;
    truncated: boolean;
  } = await page.evaluate(() => (window as any).__mixedGraphZoom());
  await info.attach("interleaved-wheel-and-node-hover", {
    body: JSON.stringify({ ...result, probes }),
    contentType: "application/json",
  });
  expect(result.truncated).toBe(false);
  expect(result.viewport_elements).toBe(1);
  expect(Math.max(...probes.map((p) => p.anchor_error_px))).toBeLessThan(1.5);
  expect(result.node_entries).toBeGreaterThanOrEqual(12);
  expect(result.scales.length).toBeGreaterThan(8);
  expect(result.scales.at(-1)!).toBeGreaterThan(result.scales[0]! + 0.01);
  for (let i = 1; i < result.scales.length; i++)
    expect(result.scales[i]!).toBeGreaterThanOrEqual(
      result.scales[i - 1]! - 0.000001,
    );
  return result;
}
