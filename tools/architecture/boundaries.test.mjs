import { strict as assert } from "node:assert";
import { test } from "node:test";
import { resolve } from "node:path";
import { cargoViolations, programFor, sourceViolations } from "./check.mjs";

const packageRoot = resolve("packages/agent-protocol");
function violations(text) {
  const file = resolve(packageRoot, "src/boundary-probe.ts");
  const program = programFor([file], new Map([[file, text]]));
  return sourceViolations(
    program.getSourceFile(file),
    program.getTypeChecker(),
    packageRoot,
    ["zod", "@meden/workspace-schema"],
  );
}

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
