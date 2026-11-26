import { test } from "node:test";
import { strict as assert } from "node:assert";
import {
  interactionTraceWindow,
  interactionInvalidations,
  interactionDispatchCsv,
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
