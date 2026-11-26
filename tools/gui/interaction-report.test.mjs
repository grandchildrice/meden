import { test } from "node:test";
import { strict as assert } from "node:assert";
import {
  mkdtemp,
  mkdir,
  readFile,
  writeFile,
  rm,
  access,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  interactionTraceWindow,
  interactionInvalidations,
  interactionDispatchCsv,
  interactionSampledDispatch,
} from "./interaction-report.mjs";

const marker = (name, ts, pid = 12, tid = 4) => ({
  name: `meden:zoom:${name}`,
  ts,
  pid,
  tid,
  ph: "I",
});

test("per-dispatch CSV retains repeated inputs in chronological order with their document attribution", () => {
  const sample = {
    document_trace: { process: 12, thread: 4, start_us: 1000 },
    dispatch_events: [
      { type: "wheel", offset_ms: 12, duration_ms: 3 },
      { type: "wheel", offset_ms: 2, duration_ms: 0.5 },
      { type: 'pointer,"move"', offset_ms: 7, duration_ms: 0.25 },
    ],
  };
  assert.equal(
    interactionDispatchCsv({ zoom: sample }),
    '"phase","event","process","thread","window_start_us","offset_ms","duration_ms"\n' +
      '"zoom","wheel",12,4,1000,2,0.5\n' +
      '"zoom","pointer,""move""",12,4,1000,7,0.25\n' +
      '"zoom","wheel",12,4,1000,12,3\n',
  );
  // Sorting the export must not mutate the original capture.
  assert.equal(sample.dispatch_events[0].offset_ms, 12);
  for (const value of [NaN, Infinity, -1]) {
    for (const field of ["offset_ms", "duration_ms"])
      assert.throws(() =>
        interactionDispatchCsv({
          zoom: {
            ...sample,
            dispatch_events: [{ ...sample.dispatch_events[0], [field]: value }],
          },
        }),
      );
  }
  assert.throws(() =>
    interactionDispatchCsv({ zoom: { ...sample, document_trace: undefined } }),
  );
});
const work = (name, ts, dur, pid = 12, tid = 4) => ({
  name,
  ts,
  dur,
  pid,
  tid,
  ph: "X",
});

const cpuProfile = () => ({
  startTime: 100,
  endTime: 150,
  nodes: [1, 2, 3].map((id) => ({
    id,
    callFrame: { functionName: `fn${id}` },
  })),
  samples: [1, 2, 3, 1, 3, 2, 1],
  timeDeltas: [5, 10, -5, 10, 5, 5, 5],
});
const dispatch = (ts, dur, type = "wheel", pid = 12, tid = 4) => ({
  ...work("EventDispatch", ts, dur, pid, tid),
  args: { data: { type } },
});

test("dispatch samples preserve nonmonotonic observations, half-open bounds and inclusive nested handlers", () => {
  const profile = cpuProfile();
  const window = interactionTraceWindow(
    [
      marker("start", 100),
      dispatch(115, 10, "click"),
      dispatch(110, 20),
      dispatch(105, 0),
      dispatch(110, 20, "worker", 12, 9),
      marker("end", 140),
    ],
    "zoom",
  );
  const before = structuredClone({ profile, window });
  const result = interactionSampledDispatch(profile, window);
  assert.equal(result.nonmonotonic_deltas, 1);
  assert.equal(result.profile_samples, 7);
  assert.deepEqual(
    result.events.map((event) => [
      event.type,
      event.offset_ms,
      event.duration_ms,
      event.observed_samples,
    ]),
    [
      ["wheel", 0.005, 0, 0],
      ["wheel", 0.01, 0.02, 4],
      ["click", 0.015, 0.01, 2],
    ],
  );
  const counts = (event) =>
    Object.fromEntries(event.samples.map((s) => [s.node_id, s.count]));
  assert.deepEqual(counts(result.events[1]), { 1: 1, 2: 1, 3: 2 });
  assert.deepEqual(counts(result.events[2]), { 1: 1, 2: 1 });
  assert(!JSON.stringify(result).includes("self_ms"));
  assert.deepEqual({ profile, window }, before);
});

test("empty sampling is an observed absence, not a zero CPU duration", () => {
  const window = interactionTraceWindow(
    [marker("start", 100), dispatch(110, 20), marker("end", 140)],
    "zoom",
  );
  const result = interactionSampledDispatch(
    { ...cpuProfile(), samples: [], timeDeltas: [] },
    window,
  );
  assert.equal(result.events[0].observed_samples, 0);
  assert.deepEqual(result.events[0].samples, []);
  assert(!("cpu_ms" in result.events[0]));
});

test("incomplete profiles, shifted clocks, unknown nodes and foreign dispatches refuse attribution", () => {
  const window = interactionTraceWindow(
    [marker("start", 100), dispatch(110, 20), marker("end", 140)],
    "zoom",
  );
  for (const bad of [
    { samples: undefined },
    { timeDeltas: undefined },
    { samples: [1] },
    { nodes: [...cpuProfile().nodes, cpuProfile().nodes[0]] },
    { nodes: [{ id: NaN, callFrame: {} }] },
    { nodes: [{ id: 1 }] },
    { samples: [99], timeDeltas: [1] },
    { samples: [1], timeDeltas: [NaN] },
    { samples: [1], timeDeltas: [-1] },
    { samples: [1], timeDeltas: [51] },
    { startTime: 101 },
    { endTime: 139 },
    { endTime: Infinity },
  ])
    assert.throws(() =>
      interactionSampledDispatch({ ...cpuProfile(), ...bad }, window),
    );
  for (const event of [
    dispatch(110, -1),
    dispatch(110, 20, "wheel", 12, 9),
    dispatch(110, 20, "wheel", 15),
    dispatch(139, 2),
    dispatch(NaN, 0),
  ]) {
    assert.throws(() =>
      interactionSampledDispatch(cpuProfile(), { ...window, events: [event] }),
    );
  }
});

test("seeded unordered sample points agree with an independent point-count oracle", () => {
  let seed = 0x6cd9;
  const next = () => (seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0);
  for (let run = 0; run < 512; run++) {
    const points = Array.from({ length: 64 }, () => ({
      time: 100 + (next() % 500),
      id: 1 + (next() % 3),
    }));
    let previous = 100;
    const profile = {
      ...cpuProfile(),
      endTime: 650,
      samples: points.map((p) => p.id),
      timeDeltas: points.map((p) => {
        const delta = p.time - previous;
        previous = p.time;
        return delta;
      }),
    };
    const original = Array.from({ length: 8 }, () =>
      dispatch(100 + (next() % 500), next() % 50),
    );
    const window = interactionTraceWindow(
      [marker("start", 100), ...original, marker("end", 650)],
      "zoom",
    );
    const result = interactionSampledDispatch(profile, window);
    for (const [i, event] of original
      .toSorted((a, b) => a.ts - b.ts)
      .entries()) {
      const expected = [1, 2, 3].map(
        (id) =>
          points.filter(
            (p) =>
              p.id === id &&
              p.time >= event.ts &&
              p.time < event.ts + event.dur,
          ).length,
      );
      const actual = result.events[i];
      assert.deepEqual(
        [1, 2, 3].map(
          (id) => actual.samples.find((s) => s.node_id === id)?.count ?? 0,
        ),
        expected,
      );
      assert.equal(
        actual.observed_samples,
        expected.reduce((a, b) => a + b, 0),
      );
    }
  }
});

test("offline export binds original observations, refuses mixed/lossy captures and never replaces evidence", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "meden-dispatch-samples-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const trace = [marker("start", 100), dispatch(110, 20), marker("end", 140)];
  const { events: _, ...document_trace } = interactionTraceWindow(
    trace,
    "zoom",
  );
  const summary = {
    environment: {
      trace_observation: { data_loss_occurred: false },
      source_maps_sha256: {},
    },
    samples: {
      zoom: {
        document_trace,
        dispatch_events: [
          { type: "wheel", offset_ms: 0.01, duration_ms: 0.02 },
        ],
      },
    },
  };
  const original = JSON.stringify(summary);
  await writeFile(join(root, "summary.json"), original);
  await writeFile(
    join(root, "trace.json"),
    JSON.stringify({ traceEvents: trace }),
  );
  await writeFile(join(root, "zoom.cpuprofile"), JSON.stringify(cpuProfile()));
  const run = (out) =>
    spawnSync(
      process.execPath,
      [
        new URL("../perf/dispatch-samples.mjs", import.meta.url).pathname,
        root,
        join(root, out),
      ],
      { encoding: "utf8", timeout: 10000 },
    );
  assert.equal(run("out").status, 0);
  const output = await readFile(join(root, "out/samples.json"), "utf8");
  const report = JSON.parse(output);
  assert.equal(
    report.original_summary_sha256,
    createHash("sha256").update(original).digest("hex"),
  );
  assert.equal(report.samples.zoom.events[0].observed_samples, 4);
  assert.match(
    await readFile(join(root, "out/samples.csv"), "utf8"),
    /leaf_samples/,
  );
  assert.notEqual(run("out").status, 0);
  assert.equal(await readFile(join(root, "out/samples.json"), "utf8"), output);
  for (const [name, mutate] of [
    [
      "empty",
      (s) => {
        s.samples = {};
      },
    ],
    [
      "lost",
      (s) => {
        s.environment.trace_observation.data_loss_occurred = true;
      },
    ],
    [
      "foreign",
      (s) => {
        s.samples.zoom.document_trace.thread = 9;
      },
    ],
    [
      "changed",
      (s) => {
        s.samples.zoom.dispatch_events[0].duration_ms = 0.03;
      },
    ],
    [
      "path",
      (s) => {
        s.samples["../outside"] = s.samples.zoom;
      },
    ],
  ]) {
    const altered = JSON.parse(original);
    mutate(altered);
    await writeFile(join(root, "summary.json"), JSON.stringify(altered));
    assert.notEqual(run(name).status, 0, name);
    await assert.rejects(access(join(root, name)));
  }
});

test("offline export refuses a changed recorded source map before writing attribution", async (t) => {
  const root = await mkdtemp(join(tmpdir(), "meden-dispatch-map-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, "assets"));
  const originalMap = JSON.stringify({
    version: 3,
    names: [],
    sources: ["original.ts"],
    mappings: "AAAA",
  });
  await writeFile(join(root, "assets/bundle.js.map"), originalMap + " ");
  const trace = [marker("start", 100), dispatch(110, 20), marker("end", 140)];
  const { events: _, ...document_trace } = interactionTraceWindow(
    trace,
    "zoom",
  );
  const summary = {
    environment: {
      trace_observation: { data_loss_occurred: false },
      source_map_directory: root,
      source_maps_sha256: {
        "bundle.js": createHash("sha256").update(originalMap).digest("hex"),
      },
    },
    samples: {
      zoom: {
        document_trace,
        dispatch_events: [
          { type: "wheel", offset_ms: 0.01, duration_ms: 0.02 },
        ],
      },
    },
  };
  const profile = cpuProfile();
  for (const node of profile.nodes)
    Object.assign(node.callFrame, {
      url: "http://localhost/assets/bundle.js",
      lineNumber: 0,
      columnNumber: 0,
    });
  await writeFile(join(root, "summary.json"), JSON.stringify(summary));
  await writeFile(
    join(root, "trace.json"),
    JSON.stringify({ traceEvents: trace }),
  );
  await writeFile(join(root, "zoom.cpuprofile"), JSON.stringify(profile));
  const result = spawnSync(
    process.execPath,
    [
      new URL("../perf/dispatch-samples.mjs", import.meta.url).pathname,
      root,
      join(root, "out"),
    ],
    { encoding: "utf8", timeout: 10000 },
  );
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Original source map changed/);
  await assert.rejects(access(join(root, "out")));
});

test("simultaneous worker and other-document tasks never inflate the measured document thread", () => {
  const wheel = work("EventDispatch", 110, 900),
    nested = work("FunctionCall", 111, 800);
  const trace = [
    work("FunctionCall", 50, 5000),
    marker("start", 100),
    work("FunctionCall", 105, 120000, 12, 9),
    wheel,
    work("Paint", 111, 40000, 15, 4),
    nested,
    { ...work("FunctionCall", 120, 50000), ph: "B" },
    marker("end", 1000),
    work("FunctionCall", 1010, 70000),
  ];
  assert.deepEqual(interactionTraceWindow(trace, "zoom"), {
    process: 12,
    thread: 4,
    start_us: 100,
    end_us: 1000,
    events: [wheel, nested],
  });
});

test("missing, duplicated, foreign-thread and malformed markers are unavailable measurements", () => {
  const start = marker("start", 100),
    end = marker("end", 200);
  for (const trace of [
    [],
    [start],
    [end],
    [start, start, end],
    [start, end, end],
    [start, marker("end", 200, 12, 9)],
    [start, marker("end", 200, 14)],
    [start, marker("end", 90)],
    [{ ...start, ts: NaN }, end],
    [
      { ...start, pid: undefined },
      { ...end, pid: undefined },
    ],
    [
      { ...start, tid: "4" },
      { ...end, tid: "4" },
    ],
  ])
    assert.throws(() => interactionTraceWindow(trace, "zoom"));
  assert.deepEqual(interactionTraceWindow([start, end], "zoom").events, []);
});

test("invalidation counts retain instant causes without attributing worker, foreign or unmarked work", () => {
  const instant = (ts, pid = 12, tid = 4) => ({
    name: "StyleRecalcInvalidationTracking",
    ph: "I",
    ts,
    pid,
    tid,
    args: {
      data: {
        reason: "Attribute",
        extraData: "stroke-width",
        nodeName: "line class='map-reference'",
        stackTrace: [{ url: "private diagnostic URL" }],
      },
    },
  });
  const trace = [
    instant(99),
    marker("start", 100),
    instant(100),
    instant(120),
    instant(130, 12, 9),
    instant(140, 15),
    { ...instant(150), ph: "X", dur: 900000 },
    { ...instant(160), name: "UnknownTracking" },
    { ...instant(170), args: {} },
    instant(NaN),
    marker("end", 200),
    instant(200),
    instant(201),
  ];
  const window = interactionTraceWindow(trace, "zoom");
  assert.equal(window.events.length, 1);
  assert.deepEqual(interactionInvalidations(trace, window), {
    observed: 3,
    rows: [
      {
        type: "StyleRecalcInvalidationTracking",
        reason: "Attribute",
        attribute_or_pseudo: "stroke-width",
        node: "line class='map-reference'",
        count: 3,
      },
    ],
  });
  assert.deepEqual(interactionInvalidations([], window), {
    observed: 0,
    rows: [],
  });
});
