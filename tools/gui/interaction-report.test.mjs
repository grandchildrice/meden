import { test } from "node:test";
import { strict as assert } from "node:assert";
import { interactionTraceWindow } from "./interaction-report.mjs";

const marker = (name, ts, pid = 12, tid = 4) => ({
  name: `meden:zoom:${name}`,
  ts,
  pid,
  tid,
  ph: "I",
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
