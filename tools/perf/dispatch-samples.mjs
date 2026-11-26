// Offline attribution of an original document-session profile to its dispatches.
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { resolve, basename } from "node:path";
import { createHash } from "node:crypto";
import { strict as assert } from "node:assert";
import { TraceMap, originalPositionFor } from "@jridgewell/trace-mapping";
import {
  interactionTraceWindow,
  interactionSampledDispatch,
} from "../gui/interaction-report.mjs";

const [capture, output, ...extra] = process.argv.slice(2);
if (!capture || !output || extra.length)
  throw Error(
    "Usage: node tools/perf/dispatch-samples.mjs CAPTURE_DIRECTORY NEW_OUTPUT_DIRECTORY",
  );
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");
const original = await readFile(resolve(capture, "summary.json"));
const summary = JSON.parse(original);
if (summary.environment.trace_observation.data_loss_occurred !== false)
  throw Error("Original trace completion is lossy or unavailable");
assert(
  Object.keys(summary.samples).length > 0,
  "Original capture has no operation windows",
);
const traceBytes = await readFile(resolve(capture, "trace.json"));
const trace = JSON.parse(traceBytes).traceEvents;
const maps = new Map(),
  mapDigests = {},
  profiles = [],
  samples = {};
async function locate(frame) {
  let name;
  try {
    name = basename(new URL(frame.url).pathname);
  } catch {
    name = basename((frame.url || "").split(/[?#]/)[0]);
  }
  const fallback = `${name || "(native)"}:${frame.lineNumber ?? -1} · ${frame.functionName || "(anonymous)"}`;
  const expected = summary.environment.source_maps_sha256?.[name];
  if (
    !expected ||
    !name.endsWith(".js") ||
    !Number.isInteger(frame.lineNumber) ||
    frame.lineNumber < 0 ||
    !Number.isInteger(frame.columnNumber) ||
    frame.columnNumber < 0
  )
    return fallback;
  if (!maps.has(name)) {
    const bytes = await readFile(
      resolve(
        summary.environment.source_map_directory,
        "assets",
        name + ".map",
      ),
    );
    assert.equal(sha(bytes), expected, "Original source map changed: " + name);
    maps.set(name, new TraceMap(JSON.parse(bytes)));
    mapDigests[name] = expected;
  }
  const original = originalPositionFor(maps.get(name), {
    line: frame.lineNumber + 1,
    column: frame.columnNumber,
  });
  return original.source
    ? `${original.source}:${original.line} · ${original.name || frame.functionName || "(anonymous)"}`
    : fallback;
}
for (const [phase, sample] of Object.entries(summary.samples)) {
  assert.match(phase, /^[a-z0-9-]+$/, "Invalid original phase name");
  const window = interactionTraceWindow(trace, phase);
  const { events: _, ...attribution } = window;
  assert.deepEqual(
    attribution,
    sample.document_trace,
    "Original document attribution changed",
  );
  const bytes = await readFile(resolve(capture, phase + ".cpuprofile"));
  const observed = interactionSampledDispatch(JSON.parse(bytes), window);
  assert.deepEqual(
    observed.events.map((e) => [e.type, e.offset_ms, e.duration_ms]),
    [...sample.dispatch_events]
      .sort((a, b) => a.offset_ms - b.offset_ms)
      .map((e) => [e.type ?? null, e.offset_ms, e.duration_ms]),
    "Original dispatch observations changed",
  );
  profiles.push({
    phase,
    sha256: sha(bytes),
    samples: observed.profile_samples,
    nonmonotonic_deltas: observed.nonmonotonic_deltas,
  });
  const events = [];
  for (const event of observed.events) {
    const hotspots = new Map();
    for (const point of event.samples) {
      const location = await locate(point.frame);
      const row = hotspots.get(location) || {
        location,
        leaf_samples: 0,
        node_ids: [],
      };
      row.leaf_samples += point.count;
      row.node_ids.push(point.node_id);
      hotspots.set(location, row);
    }
    const { samples: _, ...input } = event;
    events.push({
      ...input,
      hotspots: [...hotspots.values()].sort(
        (a, b) => b.leaf_samples - a.leaf_samples,
      ),
    });
  }
  samples[phase] = { document_trace: attribution, events };
}
const keys = [
  "phase",
  "event",
  "process",
  "thread",
  "offset_ms",
  "duration_ms",
  "observed_samples",
  "location",
  "leaf_samples",
  "share_percent",
];
const rows = Object.entries(samples).flatMap(([phase, sample]) =>
  sample.events.flatMap((event) =>
    (event.hotspots.length
      ? event.hotspots
      : [{ location: null, leaf_samples: 0 }]
    ).map((point) => ({
      phase,
      event: event.type,
      process: sample.document_trace.process,
      thread: sample.document_trace.thread,
      offset_ms: event.offset_ms,
      duration_ms: event.duration_ms,
      observed_samples: event.observed_samples,
      ...point,
      share_percent: event.observed_samples
        ? (100 * point.leaf_samples) / event.observed_samples
        : null,
    })),
  ),
);
const cell = (value) =>
  typeof value === "string"
    ? `"${value.replaceAll('"', '""')}"`
    : value == null
      ? '""'
      : String(value);
const report = {
  original_summary_sha256: sha(original),
  trace_sha256: sha(traceBytes),
  profiles,
  source_maps_sha256: mapDigests,
  methodology:
    "Offline point counts from the original document CDP session. Reconstruct each original sample timestamp/node before sorting, including nonmonotonic deltas. Dispatch intervals are half-open and nested dispatches are inclusive. Counts/shares describe observed leaf samples, not measured CPU durations or exclusive costs. Zero samples does not establish zero work. Trace/profile clock alignment and original CDP-session/file provenance remain capture-adapter assumptions; overlapping bounds alone cannot prove thread origin. Unrecorded source maps are not used. No new browser, application work or inference is invoked.",
  samples,
};
await mkdir(output);
await writeFile(
  resolve(output, "samples.json"),
  JSON.stringify(report, null, 2) + "\n",
);
await writeFile(
  resolve(output, "samples.csv"),
  [
    keys.map(cell).join(","),
    ...rows.map((row) => keys.map((key) => cell(row[key])).join(",")),
  ].join("\n") + "\n",
);
console.log(
  JSON.stringify({
    phases: Object.keys(samples).length,
    dispatches: Object.values(samples).reduce(
      (n, sample) => n + sample.events.length,
      0,
    ),
    output,
  }),
);
