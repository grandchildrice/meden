import { test } from "node:test";
import { strict as assert } from "node:assert";
import { z } from "zod";
import { validator } from "./source-schema.mjs";
import { readFileSync } from "node:fs";

const compile = (schema) => Function("z", "return " + validator(schema))(z);
test("actual generated file DTO keeps availability disjoint and bounded", () => {
  const schema = compile(
    JSON.parse(
      readFileSync(
        new URL("../../schemas/native-file.schema.json", import.meta.url),
        "utf8",
      ),
    ),
  );
  const file = {
    path: "unsupported.smt2",
    navigation: "source_only",
    reading: {
      state: "ready",
      content: {
        kind: "text",
        revision: "sha256:" + "a".repeat(64),
        bytes: 8,
        text: {
          offset: 1,
          line_count: 1,
          next_offset: null,
          text: '"literal"',
        },
      },
    },
  };
  assert.deepEqual(schema.parse(file), file);
  for (const update of [
    { path: "" },
    { navigation: "verified" },
    {
      reading: {
        state: "unavailable",
        failure: "operation",
        message: "missing",
        content: { kind: "binary", bytes: 1 },
      },
    },
    {
      reading: {
        state: "ready",
        content: { kind: "binary", bytes: 1, text: "partial" },
      },
    },
    {
      reading: {
        state: "ready",
        content: {
          ...file.reading.content,
          text: {
            offset: 0,
            line_count: 1,
            next_offset: null,
            text: "invalid",
          },
        },
      },
    },
  ])
    assert.throws(() => schema.parse({ ...file, ...update }));
});
test("owned nullable numeric constraints do not apply numeric methods to null", () => {
  const schema = compile({
    type: ["integer", "null"],
    minimum: 1,
    maximum: 40,
  });
  assert.equal(schema.parse(null), null);
  assert.equal(schema.parse(40), 40);
  assert.throws(() => schema.parse(0));
  assert.throws(() => schema.parse(41));
  const text = compile({ type: ["string", "null"], pattern: "^name$" });
  assert.equal(text.parse(null), null);
  assert.equal(text.parse("name"), "name");
  assert.throws(() => text.parse("other"));
});
test("owned object/record/reference bounds survive generation", () => {
  const schema = compile({
    type: "object",
    additionalProperties: false,
    required: ["values"],
    properties: {
      values: { type: "array", maxItems: 2, items: { $ref: "#/$defs/Name" } },
    },
    $defs: { Name: { type: "string", minLength: 1, maxLength: 4 } },
  });
  assert.deepEqual(schema.parse({ values: ["name"] }), { values: ["name"] });
  for (const input of [
    {},
    { values: [""] },
    { values: ["longer"] },
    { values: ["a", "b", "c"] },
    { values: [], extra: true },
  ])
    assert.throws(() => schema.parse(input));
});
test("unknown DTO constraints/references fail generation", () => {
  assert.throws(() => validator({ type: "string", allOf: [] }));
  assert.throws(() => validator({ $ref: "#/$defs/Missing" }));
  assert.throws(() =>
    validator({
      $ref: "#/$defs/Name",
      maxLength: 4,
      $defs: { Name: { type: "string" } },
    }),
  );
  assert.throws(() => validator({ type: "object", properties: {} }));
  for (const schema of [
    { const: "name", type: "integer" },
    { enum: ["name"], type: "boolean" },
    { oneOf: [] },
    { const: "name", maxLength: 2 },
    { enum: ["name"], pattern: "^other$" },
    { anyOf: [{ type: "string" }], allOf: [] },
    { oneOf: [{ type: "string" }, { type: "string" }] },
    {
      type: "object",
      additionalProperties: { type: "string" },
      maxProperties: 1,
    },
  ])
    assert.throws(() => validator(schema));
});

test("exclusive object unions require a disjoint required constant tag", () => {
  const branch = (tag) => ({
    type: "object",
    additionalProperties: false,
    required: ["kind", "bytes"],
    properties: {
      kind: { const: tag, type: "string" },
      bytes: { type: "integer", minimum: 0, maximum: 10 },
    },
  });
  const schema = compile({
    oneOf: [branch("binary"), branch("unsupported_encoding")],
  });
  assert.deepEqual(schema.parse({ kind: "binary", bytes: 10 }), {
    kind: "binary",
    bytes: 10,
  });
  for (const value of [
    { kind: "text", bytes: 1 },
    { kind: "binary", bytes: 11 },
    { kind: "binary", bytes: 0, text: "partial" },
    { bytes: 1 },
  ])
    assert.throws(() => schema.parse(value));
  for (const variants of [
    [branch("same"), branch("same")],
    [branch("binary"), { ...branch("encoding"), required: ["bytes"] }],
    [
      branch("binary"),
      {
        ...branch("encoding"),
        properties: { kind: { type: "string" }, bytes: { type: "integer" } },
      },
    ],
  ])
    assert.throws(() => validator({ oneOf: variants }));
});
