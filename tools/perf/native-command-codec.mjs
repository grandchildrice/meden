// Pure codec comparison, never inference or a native operation. Both strategies
// validate the same owned branches; the former GUI sent commands without a
// runtime validator, so this is not an old/new GUI latency benchmark.
import { z } from "zod";
import { nativeCommandSchema } from "../../packages/agent-protocol/src/command.generated.ts";
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { dirname } from "node:path";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const destination =
  process.argv[2] || "target/performance/native-command-codec.json";
const inputs = [
  { type: "inspect", path: "Folder/original\nλ.lean", directory: false },
  { type: "instruction", text: "Original intent" },
  { type: "source_snapshot", paths: ["Original.lean"] },
  {
    type: "approve_processor",
    connection_revision: "18446744073709551615",
    endpoint: "https://original.invalid/v1",
    model: "original",
    judge: null,
  },
  {
    type: "send_export",
    reference: { session: "original-host", id: 7 },
    serial: "18446744073709551615",
  },
];
const strategies = {
  tag_lookup: nativeCommandSchema,
  sequential_same_branches: z.union(nativeCommandSchema.options),
};
for (const schema of Object.values(strategies))
  for (const input of inputs) assert.deepEqual(schema.parse(input), input);
const rounds = 9,
  perBatch = 1000,
  rows = [];
for (const input of inputs) {
  for (const schema of Object.values(strategies))
    for (let warmup = 0; warmup < 200; warmup++)
      JSON.stringify(schema.parse(input));
  const samples = { tag_lookup: [], sequential_same_branches: [] };
  for (let round = 0; round < rounds; round++) {
    const order = Object.keys(strategies);
    if (round % 2) order.reverse();
    for (const name of order) {
      const start = performance.now();
      let encodedBytes = 0;
      for (let i = 0; i < perBatch; i++)
        encodedBytes += JSON.stringify(strategies[name].parse(input)).length;
      assert.equal(
        encodedBytes,
        JSON.stringify(strategies[name].parse(input)).length * perBatch,
      );
      samples[name].push(((performance.now() - start) * 1000) / perBatch);
    }
  }
  const summary = Object.fromEntries(
    Object.entries(samples).map(([name, values]) => {
      const sorted = [...values].sort((a, b) => a - b);
      return [
        name,
        {
          batch_mean_us: values,
          median_batch_mean_us: sorted[Math.floor(sorted.length / 2)],
        },
      ];
    }),
  );
  rows.push({ type: input.type, summary });
}
const definitions = [];
for (const path of [
  "packages/agent-protocol/src/command.generated.ts",
  "schemas/native-command.schema.json",
  "tools/perf/native-command-codec.mjs",
]) {
  const bytes = await readFile(path);
  definitions.push({
    path,
    bytes: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  });
}
const report = {
  scope:
    "Generated command validation and JSON encoding of preconstructed objects; not HTTP, queueing, model execution, graph input-to-paint or former unvalidated-client performance.",
  method:
    "Identical owned branches with tag lookup versus sequential union. Nine interleaved batches per strategy/type, 1000 validations/encodings each, 200 warmups. Encoded lengths are consumed and checked. Means per batch; not individual-event p95 or a CI budget.",
  node: process.version,
  branches: nativeCommandSchema.options.length,
  rounds,
  per_batch: perBatch,
  definitions,
  rows,
};
await mkdir(dirname(destination), { recursive: true });
await writeFile(destination, JSON.stringify(report, null, 2) + "\n", {
  flag: "wx",
});
for (const row of rows)
  console.log(
    row.type,
    Object.fromEntries(
      Object.entries(row.summary).map(([name, v]) => [
        name,
        v.median_batch_mean_us,
      ]),
    ),
  );
