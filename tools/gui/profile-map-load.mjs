// Local navigation/geometry measurements. No chat, model approval or file opens.
import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const [url, directory] = process.argv.slice(2);
if (!url || !directory)
  throw Error(
    "Usage: node tools/gui/profile-map-load.mjs GUI_URL NEW_OUTPUT_DIRECTORY",
  );
await mkdir(resolve(directory, ".."), { recursive: true });
await mkdir(directory, { mode: 0o700 }); // Keep earlier measurements intact.
const browser = await chromium.launch({
  executablePath:
    process.env.MEDEN_GUI_CHROME ||
    "/run/current-system/sw/bin/google-chrome-stable",
  args: ["--no-sandbox"],
});
try {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 960 },
    reducedMotion: "no-preference",
  });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Profiler.enable");
  await cdp.send("Profiler.setSamplingInterval", { interval: 1000 });
  await page.addInitScript(() => {
    const p = (window.__medenMapLoad = { tasks: [], ready: null });
    new PerformanceObserver((list) => {
      for (const e of list.getEntries())
        p.tasks.push({ start: e.startTime, duration: e.duration });
    }).observe({ type: "longtask", buffered: true });
    let waiting = false;
    const observer = new MutationObserver(() => {
      const scene = document.querySelector(".continuous-map-scene");
      if (!scene || waiting || !scene.getBoundingClientRect().width) return;
      waiting = true;
      const attached = performance.now();
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          p.ready = {
            attached_ms: attached,
            two_raf_ms: performance.now(),
            layout_revision: scene.getAttribute("data-scene-revision"),
            layout_source: scene.getAttribute("data-layout-source"),
            visible_boxes: scene.querySelectorAll("[data-map-path]").length,
            camera: document
              .querySelector(".project-map-page .react-flow__viewport")
              ?.getAttribute("style"),
          };
          observer.disconnect();
        }),
      );
    });
    observer.observe(document, { childList: true, subtree: true });
  });
  const commands = [],
    errors = [],
    samples = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("request", (r) => {
    if (new URL(r.url()).pathname === "/api/command")
      commands.push(r.postDataJSON());
  });
  for (let visit = 0; visit < 5; visit++) {
    await page.goto("about:blank");
    await cdp.send("Profiler.start");
    await page.goto(url);
    await expect
      .poll(() => page.evaluate(() => Boolean(window.__medenMapLoad?.ready)), {
        timeout: 60000,
      })
      .toBe(true);
    const observation = await page.evaluate(() => ({
      ...window.__medenMapLoad,
      resources: performance
        .getEntriesByType("resource")
        .filter((e) => new URL(e.name).pathname === "/api/inventory")
        .map((e) => ({
          start_ms: e.startTime,
          end_ms: e.responseEnd,
          duration_ms: e.duration,
          encoded_bytes: e.encodedBodySize,
          decoded_bytes: e.decodedBodySize,
        })),
    }));
    const { profile } = await cdp.send("Profiler.stop");
    await writeFile(
      resolve(directory, `visit-${visit}.cpuprofile`),
      JSON.stringify(profile),
    );
    samples.push({ visit, ...observation });
    // Let optional idle persistence complete before the next same-origin visit.
    // The observed layout_source, rather than the visit number, classifies hits.
    await page.waitForTimeout(1000);
  }
  const state = await page.evaluate(async () => {
    const r = await fetch("/api/state", {
      headers: {
        Authorization:
          "Bearer " + sessionStorage.getItem("meden-native-session"),
      },
    });
    if (!r.ok) throw Error("Cannot inspect original host state");
    const s = await r.json();
    return {
      running: s.running,
      authorized: s.authorized,
      project_files: s.overview.total,
    };
  });
  if (
    state.running ||
    state.authorized ||
    errors.length ||
    commands.some((c) => c.type !== "inspect" || c.directory !== true)
  )
    throw Error(
      "Map navigation changed authority, requested work or produced browser errors",
    );
  await writeFile(
    resolve(directory, "summary.json"),
    JSON.stringify(
      {
        captured_at_utc: new Date().toISOString(),
        browser: browser.version(),
        viewport: "1440x960",
        throttling: "none",
        reduced_motion: "no-preference",
        state,
        samples,
        commands,
        errors,
        scope:
          "Five complete navigations in one fresh browser context, native host/project/OS caches retained. First and persisted geometry hits are labelled by the actual DOM attribute; older clients report null. Attached/two-rAF are navigation-relative DOM/frame proxies, not physical first paint. CPU sampling and automation add overhead. Inventory transfer, JSON parsing, input construction, layout/decoding and initial rendering are included, unlike the prepared-geometry overlay unit benchmark. No file inspection, compilation request or model inference is sent.",
      },
      null,
      2,
    ),
  );
  console.log(
    JSON.stringify({
      directory,
      samples: samples.map((s) => ({ visit: s.visit, ...s.ready })),
      project_files: state.project_files,
    }),
  );
} finally {
  await browser.close();
}
