import { test } from "node:test";
import { strict as assert } from "node:assert";
import { z } from "zod";
import { validator } from "./source-schema.mjs";
import { readFileSync } from "node:fs";

const compile = (schema) => Function("z", "return " + validator(schema))(z);
test("original positive u64 counters remain exact and malformed decimals never throw from safeParse", () => {
  const schema = compile(
    JSON.parse(
      readFileSync(
        new URL("../../schemas/native-sequence.schema.json", import.meta.url),
        "utf8",
      ),
    ),
  );
  for (const value of ["1", "9007199254740993", "18446744073709551615"])
    assert.equal(schema.parse(value), value);
  for (const value of [
    0,
    1,
    null,
    "",
    "0",
    "01",
    "+1",
    "-1",
    " 1",
    "1 ",
    "1.0",
    "1e1",
    "１２",
    "🧪",
    "18446744073709551616",
    "99999999999999999999",
  ])
    assert.equal(schema.safeParse(value).success, false);
  const nullable = compile({
    type: ["string", "null"],
    format: "meden-positive-u64",
  });
  assert.equal(nullable.parse(null), null);
  assert.equal(nullable.parse("18446744073709551615"), "18446744073709551615");
  assert.equal(nullable.safeParse("18446744073709551616").success, false);
});
test("delivery observations keep exact review sequence, bounded queue and original response codes", () => {
  const owned = (name) =>
    compile(
      JSON.parse(
        readFileSync(
          new URL(
            `../../schemas/native-delivery-${name}.schema.json`,
            import.meta.url,
          ),
          "utf8",
        ),
      ),
    );
  const summary = {
    session: "original-session",
    id: 7,
    source: "payload.bin",
    contract: null,
    target: "http://127.0.0.1:1234/exact",
    bytes: 3,
    payload_sha256: "a".repeat(64),
    state: "reviewed",
  };
  const review = {
    serial: "18446744073709551615",
    summary,
    offset: 0,
    body_hex: "00ff0a",
  };
  assert.deepEqual(owned("review").parse(review), review);
  const outcome = {
    summary: { ...summary, state: "response_received" },
    http_status: 799,
  };
  assert.deepEqual(owned("outcome").parse(outcome), outcome);
  for (const update of [
    { serial: 9007199254740992 },
    { serial: "18446744073709551616" },
    { offset: 16777217 },
    { body_hex: "00Ff" },
    { body_hex: "a" },
  ])
    assert.equal(
      owned("review").safeParse({ ...review, ...update }).success,
      false,
    );
  for (const update of [
    { id: 8 },
    { session: "" },
    { bytes: 16777217 },
    { state: "sent" },
    { contract: "not-a-digest" },
    { approved: true },
  ])
    assert.equal(
      owned("summary").safeParse({ ...summary, ...update }).success,
      false,
    );
  for (const http_status of [99, 1000, 200.5])
    assert.equal(
      owned("outcome").safeParse({ ...outcome, http_status }).success,
      false,
    );
});
test("owned URI format validates URLs and unknown formats fail generation", () => {
  const uri = compile({ type: "string", format: "uri", maxLength: 100 });
  assert.equal(
    uri.parse("http://original.localhost:1234/index.html"),
    "http://original.localhost:1234/index.html",
  );
  for (const input of ["relative.html", "http://", "x".repeat(101)])
    assert.throws(() => uri.parse(input));
  for (const schema of [
    { type: "string", format: "email" },
    { type: "string", format: "uint32" },
    { type: "number", format: "uri" },
  ])
    assert.throws(() => validator(schema));
  assert.equal(
    compile({ type: ["integer", "null"], format: "uint64", minimum: 0 }).parse(
      null,
    ),
    null,
  );
  const nullableUri = compile({ type: ["string", "null"], format: "uri" });
  assert.equal(nullableUri.parse(null), null);
  assert.throws(() => nullableUri.parse("relative"));
});
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
        text: '"literal"',
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
          text: "invalid".repeat(1_048_577),
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
test("candidate output schemas retain required nulls, opaque provenance and lossless sequences", () => {
  const owned = (name) =>
    compile(
      JSON.parse(
        readFileSync(
          new URL(`../../schemas/native-${name}.schema.json`, import.meta.url),
          "utf8",
        ),
      ),
    );
  const record = {
    version: 2,
    sequence: "18446744073709551615",
    id: "analysis-" + "a".repeat(64),
    requirement_id: "requirement-" + "b".repeat(64),
    finding_id: null,
    source_artifact: null,
    engine: "replaceable.candidate.method",
    model: "original",
    processor: "original",
    judge: null,
    model_definition: { diagnostic: [null, true, { original: "λ" }] },
    input: {
      source_revision: "sha256:" + "c".repeat(64),
      target: { file: "statement.smt2", line: 1, name: "claim" },
      requirement: "Original requirement",
    },
    state: "historical",
    assessment: null,
    partial_translation: null,
    error: null,
  };
  const current = owned("analysis-current"),
    history = owned("analysis-history");
  const publication = { revision: "sha256:" + "d".repeat(64), data: record };
  assert.deepEqual(current.parse(publication), publication);
  assert.deepEqual(
    history.parse({ ...publication, data: { history: [record] } }).data.history,
    [record],
  );
  for (const field of [
    "judge",
    "finding_id",
    "source_artifact",
    "assessment",
    "partial_translation",
    "error",
  ]) {
    const incomplete = { ...record };
    delete incomplete[field];
    assert.throws(() => current.parse({ ...publication, data: incomplete }));
  }
  for (const update of [
    { sequence: 9007199254740992 },
    { sequence: "01" },
    { state: "proved" },
    { verified: true },
    { input: { ...record.input, target: { ...record.input.target, line: 0 } } },
  ])
    assert.throws(() =>
      current.parse({ ...publication, data: { ...record, ...update } }),
    );
  assert.throws(() => history.parse(publication));
  assert.throws(() =>
    current.parse({ ...publication, data: { history: [record] } }),
  );
  // Check the actual browser status boundary; the nested model has no separate
  // endpoint or generated module.
  const status = owned("session-state").shape.analyses;
  assert.throws(() =>
    status.parse({
      current: { state: "empty", revision: publication.revision },
      history: { state: "ready", runs: 1, revision: publication.revision },
    }),
  );
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
  assert.throws(() => validator([]));
  const opaque = compile({});
  assert.deepEqual(compile(true).parse({ original: [null, 42] }), {
    original: [null, 42],
  });
  for (const value of [null, true, "literal", 42, [1], { role: "diagnostic" }])
    assert.deepEqual(opaque.parse(value), value);
  assert.deepEqual(
    compile({ description: "Original diagnostic JSON" }).parse({ original: 1 }),
    { original: 1 },
  );
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
