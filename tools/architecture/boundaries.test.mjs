import { strict as assert } from "node:assert";
import { test } from "node:test";
import { resolve } from "node:path";
import {
  cargoViolations,
  compilerEntryViolations,
  programFor,
  sourceViolations,
} from "./check.mjs";

test("compiler entry keeps domain availability independent of native metadata schemas", () => {
  assert.deepEqual(
    compilerEntryViolations(
      "#[derive(Serialize)] pub struct Status { pub available: bool }",
    ),
    [],
  );
});
test("compiler identity guard rejects adding a GUI schema derive to its captured entry", () => {
  assert.match(
    compilerEntryViolations(
      "#[derive(Serialize, schemars::JsonSchema)] #[serde(deny_unknown_fields)] pub struct Status { pub available: bool }",
    ).join(),
    /compiler-captured atlas entry must not own wire schemas/,
  );
});

const packageRoot = resolve("packages/agent-protocol");
function violations(text, basename = "boundary-probe.ts") {
  const file = resolve(packageRoot, "src/" + basename);
  const program = programFor([file], new Map([[file, text]]));
  return sourceViolations(
    program.getSourceFile(file),
    program.getTypeChecker(),
    packageRoot,
    ["zod", "@meden/workspace-schema"],
  );
}

test("native wire barrel re-exports generated definitions and derived history aliases", () => {
  assert.deepEqual(
    violations(
      'import type { HistorySnapshot } from "./history.generated"; export { nativeStateSchema } from "./session-state.generated"; export type { NativeCommand } from "./command.generated"; export type HistoryData = HistorySnapshot["data"];',
      "native.ts",
    ),
    [],
  );
});
for (const source of [
  'import { z } from "zod";',
  'export type NativeCommand = { type: "run" };',
  "export const nativeStateSchema = {};",
  'export { nativeStateSchema } from "./handwritten";',
])
  test(`native wire boundary refuses ${source}`, () =>
    assert.match(
      violations(source, "native.ts").join(),
      /native wire shapes must be generated/,
    ));

test("data ports accept types and deterministic data, including a document field", () => {
  assert.deepEqual(
    violations(
      'import type { Workspace } from "@meden/workspace-schema"; export const view = { document: "text", stamp: new Date(0).toISOString() };',
    ),
    [],
  );
});

for (const source of [
  'import React from "react";',
  'export { readFile } from "node:fs";',
  'void import("node:http");',
  "void import(destination);",
  'type HostFile = import("node:fs").Stats;',
  'const io = require("node:fs");',
  'import "../../../apps/web/src/adapters/native-client";',
  'void fetch("/api/state");',
  "const x = window.location;",
  "export type View = HTMLElement;",
  "const now = Date.now();",
  "const now = new Date();",
  "const id = Math.random();",
])
  test(`data boundary refuses ${source}`, () =>
    assert.notEqual(violations(source).length, 0));

test("the kernel cannot acquire I/O dependencies or a host build script", () => {
  const base = {
    name: "meden-core",
    id: "kernel",
    targets: [],
    dependencies: [
      { name: "serde", kind: null },
      { name: "thiserror", kind: null },
    ],
  };
  assert.deepEqual(cargoViolations([base], [base.id]), []);
  assert.match(
    cargoViolations(
      [
        {
          ...base,
          dependencies: [...base.dependencies, { name: "ureq", kind: null }],
        },
      ],
      [base.id],
    ).join(),
    /kernel dependency/,
  );
  assert.match(
    cargoViolations(
      [{ ...base, targets: [{ kind: ["custom-build"] }] }],
      [base.id],
    ).join(),
    /build script/,
  );
});

test("workspace dependencies point inward", () => {
  const core = {
    name: "meden-core",
    id: "kernel",
    targets: [],
    dependencies: [],
  };
  const cli = {
    name: "meden-cli",
    id: "cli",
    targets: [],
    dependencies: [{ name: core.name, kind: null }],
  };
  assert.deepEqual(cargoViolations([core, cli], [core.id, cli.id]), []);
  assert.match(
    cargoViolations(
      [{ ...core, dependencies: [{ name: cli.name, kind: null }] }, cli],
      [core.id, cli.id],
    ).join(),
    /outward dependency/,
  );
});

test("shared application ports reject model implementations and presentation dependencies", () => {
  const app = { name: "meden-app", id: "app", targets: [], dependencies: [] };
  const transport = {
    name: "meden-llm",
    id: "transport",
    targets: [],
    dependencies: [{ name: "meden-app", kind: null }],
  };
  assert.deepEqual(
    cargoViolations([app, transport], [app.id, transport.id]),
    [],
  );
  const violations = cargoViolations(
    [
      {
        ...app,
        dependencies: [
          { name: "meden-llm", kind: null },
          { name: "ratatui", kind: null },
        ],
      },
      transport,
    ],
    [app.id, transport.id],
  ).join();
  assert.match(violations, /outward dependency/);
  assert.match(violations, /presentation implementation/);
});
