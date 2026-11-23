import { test } from "node:test";
import { strict as assert } from "node:assert";
import { z } from "zod";
import { validator } from "./source-schema.mjs";

const compile = (schema) => Function("z", "return " + validator(schema))(z);
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
