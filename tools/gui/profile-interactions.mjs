// Opt-in local browser measurements; no application telemetry or model requests.
import { chromium, expect } from "@playwright/test";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve, basename } from "node:path";
import { TraceMap, originalPositionFor } from "@jridgewell/trace-mapping";

const [url, destination = "target/gui-profile"] = process.argv.slice(2);
if (!url)
  throw new Error(
    "Usage: node tools/gui/profile-interactions.mjs GUI_URL [OUTPUT_DIRECTORY]",
  );
await mkdir(destination, { recursive: true });
const browser = await chromium.launch({
  executablePath:
    process.env.MEDEN_GUI_CHROME ||
    "/run/current-system/sw/bin/google-chrome-stable",
  args: ["--no-sandbox"],
});
const motion = process.env.MEDEN_GUI_MOTION || "reduce";
if (!["reduce", "no-preference"].includes(motion))
  throw new Error("MEDEN_GUI_MOTION must be reduce or no-preference");
const samples = {};
const maps = new Map();
const quantiles = (values) => {
  const sorted = values.filter(Number.isFinite).sort((a, b) => a - b);
  const at = (q) =>
    sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * q))] || 0;
  return {
    count: sorted.length,
    p50_ms: at(0.5),
    p95_ms: at(0.95),
    max_ms: at(1),
    total_ms: sorted.reduce((a, b) => a + b, 0),
  };
};
async function locate(frame) {
  const name = basename(frame.url || "");
  if (!name.endsWith(".js"))
    return frame.functionName || frame.url || "(anonymous)";
  if (!maps.has(name)) {
    try {
      maps.set(
        name,
        new TraceMap(
          JSON.parse(
            await readFile(
              resolve("apps/web/dist/assets", name + ".map"),
              "utf8",
            ),
          ),
        ),
      );
    } catch {
      maps.set(name, null);
    }
  }
  const map = maps.get(name);
  const original =
    map &&
    originalPositionFor(map, {
      line: frame.lineNumber + 1,
      column: frame.columnNumber,
    });
  return original?.source
    ? `${original.source}:${original.line} · ${original.name || frame.functionName || "(anonymous)"}`
    : `${name}:${frame.lineNumber + 1} · ${frame.functionName || "(anonymous)"}`;
}
try {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 960 },
    reducedMotion: motion,
  });
  page.setDefaultTimeout(15000);
  await page.addInitScript(() => {
    window.__medenProfile = {
      phase: "setup",
      frames: [],
      inputs: [],
      tasks: [],
      shifts: [],
      last: 0,
    };
    const p = window.__medenProfile;
    const frame = (now) => {
      if (p.last) p.frames.push({ phase: p.phase, ms: now - p.last });
      p.last = now;
      requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
    for (const type of [
      "wheel",
      "pointerdown",
      "pointerup",
      "pointerover",
      "pointerout",
      "click",
    ]) {
      document.addEventListener(
        type,
        () => {
          const phase = p.phase,
            start = performance.now();
          requestAnimationFrame(() =>
            requestAnimationFrame(() =>
              p.inputs.push({ phase, type, ms: performance.now() - start }),
            ),
          );
        },
        { capture: true, passive: true },
      );
    }
    new PerformanceObserver((list) => {
      for (const e of list.getEntries())
        p.tasks.push({ phase: p.phase, ms: e.duration });
    }).observe({ type: "longtask", buffered: true });
    new PerformanceObserver((list) => {
      for (const e of list.getEntries())
        p.shifts.push({
          phase: p.phase,
          value: e.value,
          recentInput: e.hadRecentInput,
        });
    }).observe({ type: "layout-shift", buffered: true });
  });
  let phase = "setup";
  const requests = [];
  const requestPhases = new WeakMap();
  page.on("request", (request) => requestPhases.set(request, phase));
  page.on("requestfinished", async (request) => {
    if (!request.url().includes("/api/")) return;
    try {
      const sizes = await request.sizes();
      requests.push({
        phase: requestPhases.get(request),
        path: new URL(request.url()).pathname,
        method: request.method(),
        bytes: sizes.responseBodySize,
        ...request.timing(),
      });
    } catch {
      // A closing page may discard the request metadata; never fetch bodies.
    }
  });
  await page.goto(url);
  const state = () =>
    page.evaluate(async () =>
      (
        await fetch("/api/state", {
          headers: {
            Authorization:
              "Bearer " + sessionStorage.getItem("meden-native-session"),
          },
        })
      ).json(),
    );
  let native = await state();
  const file =
    process.env.MEDEN_GUI_FILE ||
    native.source_snapshot?.roots[0] ||
    Object.keys(native.source_snapshot?.files || {})[0];
  if (!file)
    throw new Error(
      "Select MEDEN_GUI_FILE or open a file first; the profiler needs a real compiled graph.",
    );
  const map = page.getByRole("region", { name: "Project map", exact: true });
  const parts = file.split("/");
  for (let i = 1; i < parts.length; i++) {
    await page
      .getByRole("button", {
        name: "Folder: " + parts.slice(0, i).join("/"),
        exact: true,
      })
      .click();
  }
  console.log(
    JSON.stringify({
      setup: "Preparing the selected file, including cache validation",
      file,
    }),
  );
  const setupStart = performance.now();
  await Promise.all([
    page.waitForResponse(
      (r) =>
        r.url().endsWith("/api/command") &&
        r.request().postDataJSON()?.path === file,
    ),
    page.getByRole("button", { name: "File: " + file, exact: true }).click(),
  ]);
  await expect
    .poll(
      async () => {
        const s = await state();
        return (
          !s.running &&
          s.source_preparation[file]?.state === "ready" &&
          s.source_snapshot?.files[file] !== undefined
        );
      },
      { timeout: 600000 },
    )
    .toBe(true);
  const setupElapsed = performance.now() - setupStart;
  native = await state();
  const graph = page.locator(".ide-file-page:not([hidden]) .cone-graph");
  await expect(graph).toBeVisible();
  const viewport = graph.locator(".react-flow__viewport");
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Profiler.enable");
  await cdp.send("Profiler.setSamplingInterval", { interval: 1000 });
  const trace = [];
  cdp.on("Tracing.dataCollected", ({ value }) => trace.push(...value));
  await cdp.send("Tracing.start", {
    categories: "devtools.timeline,blink.user_timing,v8.execute",
    options: "record-as-much-as-possible",
  });
  const waitFrames = (count) =>
    page.evaluate(async (n) => {
      for (let i = 0; i < n; i++) await new Promise(requestAnimationFrame);
    }, count);
  const measure = async (label, action) => {
    await cdp.send("Profiler.start");
    phase = label;
    await page.evaluate((label) => {
      window.__medenProfile.phase = label;
      window.__medenProfile.last = 0;
      performance.mark("meden:" + label + ":start");
    }, label);
    const before = performance.now();
    await action();
    // Include D3 wheel-end (150 ms), delayed callbacks and their
    // rendering tail; profiler start/stop stay outside the measured window.
    await page.waitForTimeout(600);
    await waitFrames(8);
    const elapsed = performance.now() - before;
    await page.evaluate((label) => {
      performance.mark("meden:" + label + ":end");
      window.__medenProfile.phase = "between";
    }, label);
    phase = "between";
    const { profile } = await cdp.send("Profiler.stop");
    await writeFile(
      resolve(destination, label + ".cpuprofile"),
      JSON.stringify(profile),
    );
    const nodes = new Map(profile.nodes.map((n) => [n.id, n]));
    const self = new Map();
    for (let i = 0; i < (profile.samples || []).length; i++) {
      const id = profile.samples[i],
        ms = (profile.timeDeltas[i] || 0) / 1000;
      self.set(id, (self.get(id) || 0) + ms);
    }
    const hotspots = [];
    for (const [id, ms] of [...self].sort((a, b) => b[1] - a[1]).slice(0, 20)) {
      hotspots.push({
        self_ms: ms,
        location: await locate(nodes.get(id).callFrame),
      });
    }
    samples[label] = { elapsed_ms: elapsed, cpu_hotspots: hotspots };
    console.log(JSON.stringify({ measured: label, elapsed_ms: elapsed }));
    phase = "between";
    await page.evaluate(() => {
      window.__medenProfile.phase = "between";
    });
  };
  const graphBox = await graph.boundingBox();
  const visibleNode = () =>
    graph.locator(".radar-node > button").evaluateAll((buttons) => {
      const canvas = buttons[0]?.closest(".cone-graph").getBoundingClientRect();
      for (const b of buttons) {
        const r = b.getBoundingClientRect();
        const x =
          Math.max(r.left, canvas.left + 48) +
          Math.min(r.width, canvas.width - 48) / 2;
        const y =
          Math.max(r.top, canvas.top + 20) +
          Math.min(r.height, canvas.height - 40) / 2;
        if (
          x < canvas.right &&
          y < canvas.bottom &&
          document.elementFromPoint(x, y)?.closest("button") === b
        )
          return { x, y };
      }
      throw new Error("No graph node can be reached by the pointer");
    });
  await measure("graph-idle-poll", () => waitFrames(180));
  // Separate late setup/layout work from steady status polling. The first idle
  // window can include an overview response begun during file preparation.
  await measure("graph-idle-settled", () => waitFrames(180));
  await measure("graph-wheel", async () => {
    await page.mouse.move(
      graphBox.x + graphBox.width / 2,
      graphBox.y + graphBox.height / 2,
    );
    for (let i = 0; i < 24; i++) await page.mouse.wheel(0, i < 12 ? -35 : 35);
  });
  await measure("graph-hover", async () => {
    for (let i = 0; i < 20; i++) {
      const { x, y } = await visibleNode();
      await page.mouse.move(x, y);
      await page.mouse.move(graphBox.x + graphBox.width - 8, graphBox.y + 8);
    }
  });
  await measure("graph-pan", async () => {
    await page.mouse.move(
      graphBox.x + graphBox.width - 40,
      graphBox.y + graphBox.height / 2,
    );
    await page.mouse.down();
    await page.mouse.move(
      graphBox.x + graphBox.width - 140,
      graphBox.y + graphBox.height / 2 + 50,
      { steps: 30 },
    );
    await page.mouse.up();
  });
  await measure("graph-resize", async () => {
    for (let i = 0; i < 8; i++)
      await page.setViewportSize({ width: i % 2 ? 1440 : 1380, height: 960 });
  });
  await measure("tab-switch", async () => {
    for (let i = 0; i < 12; i++) {
      await page.getByRole("tab", { name: "Project map", exact: true }).click();
      await page.getByRole("tab", { name: parts.at(-1), exact: true }).click();
    }
  });
  await measure("graph-node-select", async () => {
    await graph.getByRole("button", { name: "Fit View", exact: true }).click();
    for (let i = 0; i < 8; i++) {
      const { x, y } = await visibleNode();
      await page.mouse.click(x, y);
    }
  });
  await page.getByRole("tab", { name: "Project map", exact: true }).click();
  const mapBox = await map.boundingBox();
  await measure("project-map-wheel", async () => {
    await page.mouse.move(
      mapBox.x + mapBox.width / 2,
      mapBox.y + mapBox.height / 2,
    );
    for (let i = 0; i < 24; i++) await page.mouse.wheel(0, i < 12 ? -35 : 35);
  });
  await measure("project-map-idle", () => waitFrames(180));
  const done = new Promise((resolve) =>
    cdp.once("Tracing.tracingComplete", resolve),
  );
  await cdp.send("Tracing.end");
  await done;
  const metrics = await page.evaluate(() => window.__medenProfile);
  for (const [label, sample] of Object.entries(samples)) {
    sample.frames = quantiles(
      metrics.frames.filter((e) => e.phase === label).map((e) => e.ms),
    );
    sample.input_to_two_animation_frames = quantiles(
      metrics.inputs.filter((e) => e.phase === label).map((e) => e.ms),
    );
    sample.long_tasks = quantiles(
      metrics.tasks.filter((e) => e.phase === label).map((e) => e.ms),
    );
    sample.requests = requests.filter((r) => r.phase === label);
    const start = trace.find((e) => e.name === "meden:" + label + ":start")?.ts;
    const end = trace.find((e) => e.name === "meden:" + label + ":end")?.ts;
    const events = trace.filter(
      (e) => e.ts >= start && e.ts <= end && e.ph === "X",
    );
    sample.dispatch = Object.fromEntries(
      [
        ...new Set(
          events
            .filter((e) => e.name === "EventDispatch")
            .map((e) => e.args?.data?.type),
        ),
      ].map((type) => [
        type,
        quantiles(
          events
            .filter(
              (e) => e.name === "EventDispatch" && e.args?.data?.type === type,
            )
            .map((e) => e.dur / 1000),
        ),
      ]),
    );
    sample.rendering = Object.fromEntries(
      [
        "UpdateLayoutTree",
        "Layout",
        "PrePaint",
        "Paint",
        "FunctionCall",
        "FireAnimationFrame",
        "TimerFire",
      ].map((name) => [
        name,
        quantiles(
          events.filter((e) => e.name === name).map((e) => e.dur / 1000),
        ),
      ]),
    );
  }
  const nativeMetrics = await page.evaluate(async () => {
    const response = await fetch("/api/metrics", {
      headers: {
        Authorization:
          "Bearer " + sessionStorage.getItem("meden-native-session"),
      },
    });
    return response.ok ? response.json() : null;
  });
  await writeFile(
    resolve(destination, "native-metrics.json"),
    JSON.stringify(nativeMetrics, null, 2),
  );
  const report = {
    environment: {
      browser: browser.version(),
      viewport: "1440x960",
      throttling: "none",
      reduced_motion: motion,
      gesture_tail_ms: 600,
      file,
      declarations: native.source_snapshot.graph.declarations.length,
      project_files: native.overview.total,
      setup_preparation_ms: setupElapsed,
      setup_cache_hit: native.source_preparation[file]?.cache_hit ?? null,
    },
    methodology:
      "Real GUI pointer/wheel/click events with CDP dispatch spans and sampled CPU profiles. Input-to-two-rAF is a presentation proxy, not measured physical presentation or INP. Nested trace spans overlap and must not be summed as exclusive CPU time. Profiling and automation add overhead; one controlled desktop run is not a field percentile.",
    samples,
    layout_shifts: metrics.shifts,
  };
  await writeFile(
    resolve(destination, "summary.json"),
    JSON.stringify(report, null, 2),
  );
  await writeFile(
    resolve(destination, "trace.json"),
    JSON.stringify({ traceEvents: trace }, (_, v) =>
      typeof v === "string"
        ? v.replace(/#session=[a-f0-9]+/g, "#session=REDACTED")
        : v,
    ),
  );
  await page.screenshot({ path: resolve(destination, "project-map.png") });
  console.log(
    JSON.stringify({ destination, file, phases: Object.keys(samples) }),
  );
} finally {
  await browser.close();
}
