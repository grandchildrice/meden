import { test, expect, type Page } from "@playwright/test";
import { build } from "vite";
import react from "@vitejs/plugin-react";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createHash } from "node:crypto";

// Exercise the actual shared component with valid revisions of its public input.
// This is an isolated browser test, separate from native publication/restart tests.
let assets: Map<string, string | Uint8Array>;
let bundleHash: string;
test.beforeAll(async () => {
  const original = process.env.MEDEN_GRAPH_PROBE_ASSETS;
  if (original) {
    const manifest: string[] = JSON.parse(
      await readFile(resolve(original, "files.json"), "utf8"),
    );
    assets = new Map(
      await Promise.all(
        manifest.map(
          async (name) =>
            [name, await readFile(resolve(original, name))] as const,
        ),
      ),
    );
  } else {
    const root = resolve("apps/web");
    const entry = "\0graph-camera-probe.js";
    const result = await build({
      configFile: false,
      root,
      logLevel: "silent",
      plugins: [
        react(),
        {
          name: "graph-camera-probe",
          resolveId(id) {
            if (id === entry) return entry;
          },
          load(id) {
            if (id !== entry) return;
            return `import {createElement,useState} from 'react';
            import {createRoot} from 'react-dom/client';
            import {GraphView} from ${JSON.stringify(resolve(root, "src/features/graph/GraphView.tsx"))};
            import '@xyflow/react/dist/style.css';
            import ${JSON.stringify(resolve(root, "src/app/styles.css"))};
            const camera={current:undefined};
            function Probe(){const [workspace,update]=useState({declarations:[],edges:[]});
              window.graphProbe={update,camera,selected:undefined};
              return createElement(GraphView,{workspace,camera,onSelect:d=>window.graphProbe.selected=d});}
            createRoot(document.getElementById('root')).render(createElement(Probe));`;
          },
        },
      ],
      build: {
        write: false,
        cssCodeSplit: false,
        rollupOptions: {
          input: entry,
          output: {
            inlineDynamicImports: true,
            entryFileNames: "probe.js",
            assetFileNames: "[name][extname]",
          },
        },
      },
    });
    if (Array.isArray(result) || !("output" in result))
      throw Error("Expected one probe build");
    assets = new Map(
      result.output.map((item) => [
        item.fileName,
        item.type === "chunk" ? item.code : item.source,
      ]),
    );
    if (process.env.MEDEN_GRAPH_PROBE_RECORD) {
      const directory = resolve(process.env.MEDEN_GRAPH_PROBE_RECORD);
      await mkdir(directory, { recursive: true });
      for (const [name, body] of assets)
        await writeFile(resolve(directory, name), body);
      await writeFile(
        resolve(directory, "files.json"),
        JSON.stringify([...assets.keys()]),
      );
    }
  }
  bundleHash = createHash("sha256")
    .update(assets.get("probe.js")!)
    .digest("hex");
});

const graph = (version: number, extra = 0) => {
  const names = [
    "main",
    "dependency",
    "reference",
    ...Array.from({ length: extra }, (_, i) => "later" + i),
  ];
  return {
    declarations: names.map((name, i) => ({
      id: `revision-${version}:${name}`,
      name,
      label: name,
      kind: "theorem" as const,
      isMainTheorem: i === 0,
      statement: "True",
      proofStatus: "unknown" as const,
      evidenceRefs: [],
      axioms: [],
      source: { file: "Original.lean", line: version * 10 + i },
    })),
    edges: names.slice(1).map((name, i) => ({
      source: `revision-${version}:${i === 1 ? name : "main"}`,
      target: `revision-${version}:${i === 1 ? "main" : name}`,
      site: "unknown" as const,
    })),
  };
};
async function start(page: Page) {
  await page.route("**/graph-camera-probe/**", async (route) => {
    const path = new URL(route.request().url()).pathname.split("/").at(-1)!;
    if (!path)
      return route.fulfill({
        contentType: "text/html",
        body: `<link rel="stylesheet" href="style.css"><div id="root" style="display:flex;width:1100px;height:720px"></div><script type="module" src="probe.js"></script>`,
      });
    const body = assets.get(path);
    if (!body) throw Error("Unknown probe asset " + path);
    return route.fulfill({
      body: typeof body === "string" ? body : Buffer.from(body),
      contentType: path.endsWith(".js")
        ? "text/javascript"
        : path.endsWith(".css")
          ? "text/css"
          : undefined,
    });
  });
  await page.goto("/graph-camera-probe/");
  await page.waitForFunction(() => !!(window as any).graphProbe);
  await page.evaluate(
    (data) => (window as any).graphProbe.update(data),
    graph(1),
  );
  await expect(page.locator(".root-node>button")).toContainText("main");
  await expect(page.locator(".root-node>button")).toBeInViewport({
    ratio: 0.7,
  });
}
const settle = (page: Page) =>
  page.locator(".cone-graph").evaluate(async (element) => {
    let old = "",
      stable = 0;
    for (let i = 0; i < 100; i++) {
      await new Promise(requestAnimationFrame);
      const next =
        element.querySelector(".react-flow__viewport")?.getAttribute("style") ||
        "";
      stable = old === next ? stable + 1 : 0;
      old = next;
      if (stable >= 8) return old;
    }
    throw Error("Camera did not settle");
  });
async function zoom(page: Page) {
  const bounds = (await page.locator(".cone-graph").boundingBox())!;
  const before = await settle(page);
  await page.mouse.move(
    bounds.x + bounds.width / 2,
    bounds.y + bounds.height / 2,
  );
  await page.mouse.wheel(0, -250);
  await page.waitForTimeout(200);
  const after = await settle(page);
  expect(after).not.toBe(before);
  return after;
}

for (const center of ["main", "dependency"])
  test(`shared graph retains the ${center} camera when declaration IDs change`, async ({
    page,
  }, info) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await start(page);
    await settle(page);
    expect(
      await page.evaluate(() => (window as any).graphProbe.camera.current),
    ).toBeUndefined();
    if (center !== "main") {
      await page.locator('[data-id="revision-1:dependency"] button').dblclick();
      await expect(page.locator(".root-node>button")).toContainText(center);
      await settle(page);
    }
    const before = await zoom(page);
    const originalIds = await page
      .locator(".react-flow__node")
      .evaluateAll((nodes) =>
        nodes.map((node) => node.getAttribute("data-id")!).sort(),
      );
    await page.evaluate(
      (data) => (window as any).graphProbe.update(data),
      graph(2),
    );
    await expect(page.locator(".root-node>button")).toContainText(center);
    await expect
      .poll(() =>
        page
          .locator(".react-flow__node")
          .evaluateAll((nodes) =>
            nodes.map((node) => node.getAttribute("data-id")!).sort(),
          ),
      )
      .toEqual(
        originalIds.map((id) => id.replace("revision-1:", "revision-2:")),
      );
    const after = await settle(page);
    await info.attach("revision-camera", {
      contentType: "application/json",
      body: JSON.stringify({
        bundle_sha256: bundleHash,
        center,
        before,
        after,
        original_source_file: "Original.lean",
        new_source_line: graph(2).declarations.find((d) => d.name === center)!
          .source.line,
        scope:
          "Actual shared GraphView with valid ID-reissued inputs; isolated component, not a native publisher, live model or physical presentation measurement.",
      }),
    });
    await expect(page.locator(".root-node>button")).toContainText(center);
    expect(after).toBe(before);
    await page.locator(".root-node>button").click();
    expect(
      await page.evaluate(() => (window as any).graphProbe.selected.source),
    ).toEqual(graph(2).declarations.find((d) => d.name === center)!.source);
    expect(errors).toEqual([]);
  });

test("initial graph fitting is not retained as a user camera across revisions", async ({
  page,
}) => {
  await start(page);
  await settle(page);
  expect(
    await page.evaluate(() => (window as any).graphProbe.camera.current),
  ).toBeUndefined();
  const before = await settle(page);
  await page.evaluate(
    (data) => (window as any).graphProbe.update(data),
    graph(2, 12),
  );
  await expect(
    page.locator('.react-flow__node[data-id^="revision-2:"]'),
  ).toHaveCount(15);
  expect(await settle(page)).not.toBe(before);
  expect(
    await page.evaluate(() => (window as any).graphProbe.camera.current),
  ).toBeUndefined();
});

test("ambiguous source names keep an exact current choice but do not restore onto a new ID", async ({
  page,
}, info) => {
  await start(page);
  const ambiguous = (version: number) => {
    const value = graph(version);
    value.declarations[0]!.name = "shared";
    value.declarations[1]!.name = "shared";
    return value;
  };
  await page.evaluate(
    (value) => (window as any).graphProbe.update(value),
    ambiguous(1),
  );
  await settle(page);
  await page.locator('[data-id="revision-1:dependency"] button').dblclick();
  await expect(page.locator(".root-node>button")).toContainText("dependency");
  const before = await zoom(page);
  await page.evaluate(
    (value) => (window as any).graphProbe.update(value),
    ambiguous(2),
  );
  await expect(page.locator(".root-node>button")).toContainText("main");
  const after = await settle(page);
  expect(after).not.toBe(before);
  await page.locator(".root-node>button").click();
  expect(
    await page.evaluate(() => (window as any).graphProbe.selected.source.line),
  ).toBe(20);
  await info.attach("ambiguous-center", {
    contentType: "application/json",
    body: JSON.stringify({
      bundle_sha256: bundleHash,
      before,
      after,
      current_id_selected_before_update: true,
      ambiguous_rebinding_refused: true,
    }),
  });
  const unambiguous = graph(3, 12);
  unambiguous.declarations[0]!.name = "shared";
  await page.evaluate(
    (value) => (window as any).graphProbe.update(value),
    unambiguous,
  );
  await expect(
    page.locator('.react-flow__node[data-id^="revision-3:"]'),
  ).toHaveCount(15);
  // Neither an ambiguous old gesture nor its automatic refit may become a
  // gesture in this newly unambiguous, larger graph.
  expect(await settle(page)).not.toBe(after);
  expect(
    await page.evaluate(() => (window as any).graphProbe.camera.current),
  ).toBeUndefined();
});

test("a reused ID cannot replace the chosen declaration with a different name", async ({
  page,
}, info) => {
  await start(page);
  await settle(page);
  await page.locator('[data-id="revision-1:dependency"] button').dblclick();
  await expect(page.locator(".root-node>button")).toContainText("dependency");
  const before = await zoom(page),
    value = graph(2);
  const ids = new Map(
    value.declarations.map((d) => [
      d.id,
      "revision-1:" +
        (d.name === "main"
          ? "dependency"
          : d.name === "dependency"
            ? "main"
            : d.name),
    ]),
  );
  for (const declaration of value.declarations)
    declaration.id = ids.get(declaration.id)!;
  for (const edge of value.edges) {
    edge.source = ids.get(edge.source)!;
    edge.target = ids.get(edge.target)!;
  }
  await page.evaluate(
    (value) => (window as any).graphProbe.update(value),
    value,
  );
  await expect(page.locator(".root-node>button")).toContainText("dependency");
  const after = await settle(page);
  expect(after).toBe(before);
  await expect(page.locator(".root-node")).toHaveAttribute(
    "class",
    /root-node/,
  );
  await page.locator(".root-node>button").click();
  expect(
    await page.evaluate(() => (window as any).graphProbe.selected.source.line),
  ).toBe(21);
  await info.attach("reused-id-center", {
    contentType: "application/json",
    body: JSON.stringify({
      bundle_sha256: bundleHash,
      before,
      after,
      reused_id_for_other_name: true,
      selected_current_source_line: 21,
    }),
  });
});

test("an ID update arriving during wheel input never exposes the default camera", async ({
  page,
}, info) => {
  await start(page);
  await settle(page);
  const initial = await page
    .locator(".react-flow__viewport")
    .getAttribute("style");
  await page.evaluate((next) => {
    (window as any).revisionDuringWheel = new Promise((resolve) => {
      document.querySelector(".cone-graph")!.addEventListener(
        "wheel",
        () => {
          requestAnimationFrame(() =>
            requestAnimationFrame(() => {
              const before = document
                .querySelector(".react-flow__viewport")!
                .getAttribute("style");
              (window as any).graphProbe.update(next);
              const frames: Array<string | null> = [];
              const sample = () => {
                frames.push(
                  document
                    .querySelector(".react-flow__viewport")
                    ?.getAttribute("style") ?? null,
                );
                if (frames.length < 20) requestAnimationFrame(sample);
                else resolve({ before, frames });
              };
              sample();
            }),
          );
        },
        { once: true, capture: true },
      );
    });
  }, graph(2));
  const bounds = (await page.locator(".react-flow").boundingBox())!;
  await page.mouse.move(bounds.x + bounds.width - 12, bounds.y + 12);
  await page.mouse.wheel(0, -250);
  const result: { before: string; frames: Array<string | null> } =
    await page.evaluate(() => (window as any).revisionDuringWheel);
  await info.attach("wheel-revision-frames", {
    contentType: "application/json",
    body: JSON.stringify({
      bundle_sha256: bundleHash,
      initial,
      ...result,
      scope:
        "Valid component input updated two animation frames after an actual delivered wheel event; 20 sampled viewport frames, not physical-pixel presentation.",
    }),
  });
  expect(result.before).not.toBe(initial);
  expect(result.frames).toHaveLength(20);
  expect(new Set(result.frames)).toEqual(new Set([result.before]));
  await expect(
    page.locator('.react-flow__node[data-id^="revision-2:"]'),
  ).toHaveCount(3);
  await page.locator(".root-node>button").click();
  expect(
    await page.evaluate(() => (window as any).graphProbe.selected.source.line),
  ).toBe(20);
});
