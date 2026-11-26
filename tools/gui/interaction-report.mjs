// Local diagnostic exports only; consume the same observations as summary.json.
// User-timing marks identify the document thread. Worker callbacks can share
// the same timestamp window, but cannot count as synchronous UI-thread work.
export function interactionTraceWindow(trace, phase) {
  const marks = (suffix) =>
    trace.filter((event) => event.name === `meden:${phase}:${suffix}`);
  const starts = marks("start"),
    ends = marks("end");
  if (starts.length !== 1 || ends.length !== 1)
    throw Error(`Missing or ambiguous trace window: ${phase}`);
  const [start] = starts,
    [end] = ends;
  if (
    !Number.isInteger(start.pid) ||
    !Number.isInteger(start.tid) ||
    start.pid !== end.pid ||
    start.tid !== end.tid ||
    !Number.isFinite(start.ts) ||
    !Number.isFinite(end.ts) ||
    end.ts < start.ts
  )
    throw Error(`Invalid document trace window: ${phase}`);
  return {
    process: start.pid,
    thread: start.tid,
    start_us: start.ts,
    end_us: end.ts,
    events: trace.filter(
      (event) =>
        event.pid === start.pid &&
        event.tid === start.tid &&
        event.ts >= start.ts &&
        event.ts <= end.ts &&
        event.ph === "X",
    ),
  };
}

// Invalidation observations are instant records, not duration spans. Keep them
// on the same marked document thread, separately from inclusive event timing.
export function interactionInvalidations(trace, window) {
  const names = new Set([
    "StyleRecalcInvalidationTracking",
    "ScheduleStyleInvalidationTracking",
    "StyleInvalidatorInvalidationTracking",
    "LayoutInvalidationTracking",
  ]);
  const rows = new Map();
  let observed = 0;
  for (const event of trace) {
    if (
      event.pid !== window.process ||
      event.tid !== window.thread ||
      event.ts < window.start_us ||
      event.ts > window.end_us ||
      !Number.isFinite(event.ts) ||
      event.ph !== "I" ||
      !names.has(event.name)
    )
      continue;
    const data = event.args?.data;
    if (!data || typeof data !== "object") continue;
    const string = (value) => (typeof value === "string" ? value : null);
    const row = {
      type: event.name,
      reason: string(data.reason),
      attribute_or_pseudo: string(
        data.extraData ?? data.changedAttribute ?? data.changedPseudo,
      ),
      node: string(data.nodeName),
    };
    const key = JSON.stringify(row);
    const retained = rows.get(key);
    if (retained) retained.count++;
    else rows.set(key, { ...row, count: 1 });
    observed++;
  }
  return {
    observed,
    rows: [...rows.values()].sort((a, b) => b.count - a.count),
  };
}

export function interactionRows(samples) {
  return Object.entries(samples).flatMap(([phase, sample]) =>
    Object.entries(sample.dispatch).map(([event, handler]) => {
      const proxy = sample.input_to_two_animation_frames_by_type[event];
      const timing = sample.event_timing?.[event];
      return {
        phase,
        event,
        handler_count: handler.count,
        handler_p50_ms: handler.p50_ms,
        handler_p95_ms: handler.p95_ms,
        handler_max_ms: handler.max_ms,
        two_raf_count: proxy?.count ?? 0,
        two_raf_p95_ms: proxy?.count ? proxy.p95_ms : null,
        event_timing_count: timing?.queue.count ?? 0,
        queue_p95_ms: timing?.queue.count ? timing.queue.p95_ms : null,
      };
    }),
  );
}

export function interactionCsv(rows) {
  const keys = [
    "phase",
    "event",
    "handler_count",
    "handler_p50_ms",
    "handler_p95_ms",
    "handler_max_ms",
    "two_raf_count",
    "two_raf_p95_ms",
    "event_timing_count",
    "queue_p95_ms",
  ];
  const cell = (value) => JSON.stringify(value ?? "");
  return (
    [
      keys.map(cell).join(","),
      ...rows.map((r) => keys.map((k) => cell(r[k])).join(",")),
    ].join("\n") + "\n"
  );
}

export function interactionDispatchCsv(samples) {
  const keys = [
    "phase",
    "event",
    "process",
    "thread",
    "window_start_us",
    "offset_ms",
    "duration_ms",
  ];
  const rows = Object.entries(samples).flatMap(([phase, sample]) => {
    const window = sample.document_trace;
    if (
      !Number.isInteger(window?.process) ||
      !Number.isInteger(window?.thread) ||
      !Number.isFinite(window?.start_us)
    )
      throw Error(`Missing document attribution for ${phase}`);
    return [...sample.dispatch_events]
      .sort((a, b) => a.offset_ms - b.offset_ms)
      .map((event) => {
        if (
          !Number.isFinite(event.offset_ms) ||
          event.offset_ms < 0 ||
          !Number.isFinite(event.duration_ms) ||
          event.duration_ms < 0
        )
          throw Error(`Invalid dispatch observation for ${phase}`);
        return {
          phase,
          event: typeof event.type === "string" ? event.type : null,
          process: window.process,
          thread: window.thread,
          window_start_us: window.start_us,
          offset_ms: event.offset_ms,
          duration_ms: event.duration_ms,
        };
      });
  });
  const cell = (value) =>
    typeof value === "string"
      ? `"${value.replaceAll('"', '""')}"`
      : value == null
        ? '""'
        : String(value);
  return (
    [
      keys.map(cell).join(","),
      ...rows.map((r) => keys.map((k) => cell(r[k])).join(",")),
    ].join("\n") + "\n"
  );
}

export function interactionMarkdown(rows, samples) {
  const ms = (value) => (value == null ? "unavailable" : value.toFixed(3));
  return [
    "# Local interaction measurements",
    "",
    "Event dispatch is synchronous handler time, including nested handlers. Two-rAF is an input-to-animation-frame proxy, not physical presentation or INP. Event Timing queue observations cover only reported discrete events (16 ms threshold); absence is unavailable, not zero. Continuous wheel/move/resize events do not provide that queue measurement. Automation, CPU sampling and tracing add overhead; these are diagnostic sample percentiles, not field percentiles.",
    "",
    "| Action | Event | Count | Handler p50 (ms) | Handler p95 (ms) | Handler max (ms) | Two-rAF p95 (ms) |",
    "| --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ...rows.map(
      (r) =>
        `| ${r.phase} | ${r.event} | ${r.handler_count} | ${ms(r.handler_p50_ms)} | ${ms(r.handler_p95_ms)} | ${ms(r.handler_max_ms)} | ${ms(r.two_raf_p95_ms)} |`,
    ),
    "",
    "## Rendering and work after handlers",
    "",
    "Trace stages belong only to the document process/thread identified by the window's user-timing marks. Worker and other-process callbacks are excluded. Stages are inclusive and overlap; never sum them as exclusive CPU. Action elapsed times include automation and the deliberate 600 ms settling tail. Browser network and native-stage observations are separate in summary.json.",
    "",
    ...Object.entries(samples).flatMap(([phase, sample]) => [
      `### ${phase}`,
      "",
      `Long tasks: ${sample.long_tasks.count}; longest ${sample.long_tasks.count ? ms(sample.long_tasks.max_ms) + " ms" : "none observed"}. Frame-interval p95: ${ms(sample.frames.p95_ms)} ms.`,
      "",
      "| Stage | Count | p95 (ms) | Total inclusive (ms) |",
      "| --- | ---: | ---: | ---: |",
      ...[
        "UpdateLayoutTree",
        "Layout",
        "PrePaint",
        "Paint",
        "FireAnimationFrame",
        "TimerFire",
      ].map((name) => {
        const r = sample.rendering[name];
        return `| ${name} | ${r.count} | ${r.count ? ms(r.p95_ms) : "unavailable"} | ${ms(r.total_ms)} |`;
      }),
      "",
      "Sampled CPU leaders (self time; idle/program/GC entries are retained):",
      "",
      ...sample.cpu_hotspots
        .slice(0, 5)
        .map((h) => `- ${ms(h.self_ms)} ms: ${h.location}`),
      "",
    ]),
    "",
  ].join("\n");
}
