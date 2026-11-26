// Isolated native host + real public GUI navigation. Never sends chat/inference.
import { spawn } from "node:child_process";
import { once } from "node:events";
import { createWriteStream } from "node:fs";
import { mkdir, open, readFile, readdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createHash } from "node:crypto";
import { finished } from "node:stream/promises";

const [workspace, file, destination = "target/native-profile"] =
  process.argv.slice(2);
if (!workspace || !file)
  throw new Error(
    "Usage: node tools/perf/profile-native.mjs WORKSPACE FILE [OUTPUT_DIRECTORY]",
  );
await mkdir(destination, { recursive: true, mode: 0o700 });
const attach = process.env.MEDEN_PROFILE_PID;
const binary = attach
  ? `/proc/${attach}/exe`
  : resolve(process.env.MEDEN_BIN || "target/profiling/meden");
const port = process.env.MEDEN_PROFILE_PORT || "4324";
const run = async (program, args, options = {}) => {
  const child = spawn(program, args, { stdio: "inherit", ...options });
  const [code, signal] = await once(child, "exit");
  if (code !== 0) throw new Error(`${program} failed: ${code ?? signal}`);
};
// A missing tool/permission is a failed prerequisite, never a successful profile.
await run("bash", [
  "tools/perf/perf.sh",
  "stat",
  "-e",
  "task-clock:u",
  "--",
  "true",
]);
const host = attach
  ? undefined
  : spawn(
      binary,
      [
        "gui",
        "--workspace",
        resolve(workspace),
        "--assets",
        resolve("apps/web/dist"),
        "--port",
        port,
      ],
      {
        env: { ...process.env, MEDEN_ATLAS_BACKEND: "lake" },
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
host?.stderr.pipe(createWriteStream(resolve(destination, "host.stderr.log")));
const hostExit = host ? once(host, "exit") : Promise.resolve();
let recorder;
let recordingExit;
try {
  const url = attach
    ? process.env.MEDEN_GUI_URL
    : await new Promise((accept, reject) => {
        let output = "";
        const timer = setTimeout(
          () => reject(new Error("Native GUI startup timed out")),
          15000,
        );
        host.once("error", reject);
        host.once("exit", () => {
          clearTimeout(timer);
          reject(new Error("Native GUI exited before readiness"));
        });
        host.stdout.on("data", (chunk) => {
          output += chunk;
          const match = output.match(/Meden GUI: (http:\/\/[^\s]+)/);
          if (match) {
            clearTimeout(timer);
            accept(match[1]);
          }
        });
      });
  recorder = spawn(
    "bash",
    [
      "tools/perf/perf.sh",
      "record",
      "-e",
      "cpu-clock:u",
      "-F",
      "99",
      "--call-graph",
      "dwarf,8192",
      "-o",
      resolve(destination, "perf.data"),
      "-p",
      String(attach || host.pid),
    ],
    { stdio: ["ignore", "ignore", "pipe"] },
  );
  recordingExit = once(recorder, "close");
  const recordingLog = createWriteStream(
    resolve(destination, "perf.stderr.log"),
  );
  recorder.stderr.pipe(recordingLog);
  await run(
    process.execPath,
    ["tools/gui/profile-interactions.mjs", url, destination],
    {
      env: { ...process.env, MEDEN_GUI_FILE: file },
    },
  );
  recorder.kill("SIGINT");
  const [recordCode, recordSignal] = await recordingExit;
  await finished(recordingLog);
  recorder = undefined;
  // perf may preserve the requested SIGINT as 130; the report must also decode.
  if (recordCode !== 0 && recordCode !== 130 && recordSignal !== "SIGINT")
    throw new Error("Native sampling failed; inspect perf.stderr.log");
  const count = /\((\d+) samples\)/.exec(
    await readFile(resolve(destination, "perf.stderr.log"), "utf8"),
  );
  if (!count || Number(count[1]) === 0)
    throw new Error("Native capture has no confirmed samples");
  const report = await open(resolve(destination, "perf-report.txt"), "w");
  try {
    await run(
      "bash",
      [
        "tools/perf/perf.sh",
        "report",
        "--stdio",
        "--no-children",
        "--call-graph",
        "none",
        "--column-widths",
        "6,16,40,120",
        "--sort",
        "comm,dso,symbol",
        "--percent-limit",
        "0.5",
        "-i",
        resolve(destination, "perf.data"),
      ],
      { stdio: ["ignore", report.fd, "inherit"] },
    );
  } finally {
    await report.close();
  }
  const files = [
    "perf.data",
    "perf-report.txt",
    "trace.json",
    "trace-status.json",
    "native-metrics.json",
    "preparation.json",
    "events.csv",
    "dispatch-events.csv",
    "events.md",
    "summary.json",
    ...(process.env.MEDEN_GUI_PAINT_OBSERVATIONS === "1"
      ? ["area-paints.json"]
      : []),
    ...(await readdir(destination))
      .filter(
        (name) =>
          name.endsWith(".cpuprofile") || name === "preparation-log.json",
      )
      .sort(),
  ];
  const digests = {};
  for (const name of files)
    digests[name] = createHash("sha256")
      .update(await readFile(resolve(destination, name)))
      .digest("hex");
  await writeFile(
    resolve(destination, "manifest.json"),
    JSON.stringify(
      {
        binary_sha256: createHash("sha256")
          .update(await readFile(binary))
          .digest("hex"),
        samples: Number(count[1]),
        perf_event_paranoid: (
          await readFile("/proc/sys/kernel/perf_event_paranoid", "utf8")
        ).trim(),
        kptr_restrict: (
          await readFile("/proc/sys/kernel/kptr_restrict", "utf8")
        ).trim(),
        event: "cpu-clock:u",
        frequency_hz: 99,
        stack: "dwarf,8192",
        digests,
        limits:
          "Current-user native host and inherited workers only; no kernel stacks. Thread CPU spans exclude worker CPU. File preparation reuses project/OS caches, not a cold installation. Sampling changes timing; browser profiles have their own overhead.",
      },
      null,
      2,
    ),
  );
} finally {
  if (recorder) {
    recorder.kill("SIGINT");
    await recordingExit;
  }
  if (host && host.exitCode === null && host.signalCode === null)
    host.kill("SIGINT");
  await hostExit;
}
