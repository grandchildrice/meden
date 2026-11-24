import { afterEach, expect, test, vi } from "vitest";
import {
  evidencePublicationSchema,
  nativeReferenceSchema,
  nativeStateSchema,
  verificationJobPublicationSchema,
  previewPublicationSchema,
} from "@meden/agent-protocol/native";
import { NativeClient, NativeHttpError } from "./native-client";

const revision = "sha256:" + "a".repeat(64);
const snapshot = {
  revision,
  data: {
    portable: {
      document: null,
      error: null,
      lean_available: false,
      lean: "not prepared",
    },
    messages: [{ speaker: "user", text: "Original history" }],
    logs: [],
  },
};
afterEach(() => vi.unstubAllGlobals());

test("commands validate before transport and retain the exact original review counter and paths", async () => {
  const fetch = vi
    .fn()
    .mockImplementation(() =>
      Promise.resolve(Response.json({ accepted: true }, { status: 202 })),
    );
  vi.stubGlobal("fetch", fetch);
  const client = new NativeClient("host-session");
  const review = {
    type: "approve_processor" as const,
    connection_revision: "18446744073709551615",
    endpoint: "https://original.invalid/v1/chat/completions",
    model: "original-model",
    judge: { endpoint: "https://judge.invalid/v1", model: "original-judge" },
  };
  await client.command(review);
  expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual(review);
  const inspect = {
    type: "inspect" as const,
    path: "Folder/one\nλ.lean",
    directory: false,
  };
  await client.command(inspect);
  expect(JSON.parse(fetch.mock.calls[1][1].body)).toEqual(inspect);
  for (const invalid of [
    { ...review, connection_revision: 1 },
    { ...review, connection_revision: null },
    { ...review, connection_revision: "0" },
    { ...review, connection_revision: "01" },
    { ...review, connection_revision: "18446744073709551616" },
    { ...review, approved: true },
    { type: "instruction", text: "Original intent", execute: "unreviewed" },
    { type: "run_unknown_backend", path: "candidate" },
  ])
    await expect(
      client.command(invalid as Parameters<NativeClient["command"]>[0]),
    ).rejects.toThrow();
  expect(fetch).toHaveBeenCalledTimes(2);
});

test("preview reads bind frame and publication while preserving bounded diagnostics", async () => {
  const frame = "sha256:" + "b".repeat(64);
  const data = {
    frame,
    root: "public",
    entry: "index.html",
    revision: "sha256:" + "c".repeat(64),
    url: "http://original.localhost:1234/index.html",
    file_count: 1,
    byte_count: 17,
    errors: [
      "Missing preview asset: " + "🧪".repeat(500) + "… (path shortened)",
    ],
  };
  const fetch = vi
    .fn()
    .mockImplementation(() =>
      Promise.resolve(Response.json({ revision, data })),
    );
  vi.stubGlobal("fetch", fetch);
  const client = new NativeClient("session");
  expect(await client.preview(revision, frame)).toEqual({ revision, data });
  expect(fetch.mock.calls[0][0]).toBe(
    "/api/preview?revision=" + encodeURIComponent(revision),
  );
  await expect(
    client.preview(revision, "sha256:" + "d".repeat(64)),
  ).rejects.toThrow("requested frame");
  await expect(
    client.preview("sha256:" + "e".repeat(64), frame),
  ).rejects.toThrow("requested revision");
  for (const update of [
    { url: "relative.html" },
    { file_count: 257 },
    { byte_count: 16777217 },
    { errors: Array(21).fill("x") },
    { errors: ["x".repeat(2401)] },
    { verified: true },
    { root: "" },
  ])
    expect(
      previewPublicationSchema.safeParse({
        revision,
        data: { ...data, ...update },
      }).success,
    ).toBe(false);
  expect(nativeStateSchema.shape.preview.parse({ state: "idle" })).toEqual({
    state: "idle",
  });
  expect(
    nativeStateSchema.shape.preview.safeParse({ state: "ready", revision })
      .success,
  ).toBe(false);
  expect(nativeStateSchema.shape.preview.safeParse(data).success).toBe(false);
});

test("verification queries bind the exact job and revision without admitting receipt fields", async () => {
  const job = "sha256:" + "b".repeat(64);
  const inputs = {
    source_sha256: "c".repeat(64),
    prepared_sha256: "d".repeat(64),
  };
  const data = {
    context: {
      job,
      reference: {
        revision: "18446744073709551615",
        session: "host",
        contract: "e".repeat(64),
      },
      source_path: "counter.model.json",
      name: "Finite counter",
      backend: "z3.finite.v1",
      class: "bounded",
      scope: "Original finite scope",
    },
    elapsed_ms: 12,
    phase: { state: "running", stage: { state: "checking", inputs } },
  };
  const fetch = vi
    .fn()
    .mockImplementation(() =>
      Promise.resolve(Response.json({ revision, data })),
    );
  vi.stubGlobal("fetch", fetch);
  const client = new NativeClient("session");
  expect(await client.verificationJob(revision, job)).toEqual({
    revision,
    data,
  });
  expect(fetch.mock.calls[0][0]).toBe(
    "/api/verification-job?revision=" + encodeURIComponent(revision),
  );
  await expect(
    client.verificationJob(revision, "sha256:" + "f".repeat(64)),
  ).rejects.toThrow("does not match its requested job");
  await expect(
    client.verificationJob("sha256:" + "f".repeat(64), job),
  ).rejects.toThrow("does not match its requested revision");
  expect(
    nativeStateSchema.shape.verification_job.parse({ state: "idle" }),
  ).toEqual({ state: "idle" });
  for (const phase of [
    { state: "running", stage: { state: "checking" } },
    { state: "admitted", inputs },
    {
      state: "failed",
      kind: "unverified",
      message: "missing original stage/input",
    },
    { state: "running", stage: { state: "checking", inputs }, receipt: true },
  ]) {
    expect(
      verificationJobPublicationSchema.safeParse({
        revision,
        data: { ...data, phase },
      }).success,
    ).toBe(false);
  }
  for (const observed of [
    { state: "reading_source" },
    { state: "preparing_checker", source_sha256: inputs.source_sha256 },
    { state: "checking", inputs },
  ]) {
    expect(
      verificationJobPublicationSchema.parse({
        revision,
        data: {
          ...data,
          phase: {
            state: "failed",
            during: observed,
            kind: "unavailable",
            message: "Original resource failure",
          },
        },
      }),
    ).toBeTruthy();
  }
});

test("evidence is an exact independent publication with disjoint states and lossless references", async () => {
  const reference = {
    revision: "18446744073709551615",
    session: "original host",
    contract: "b".repeat(64),
  };
  expect(nativeReferenceSchema.parse(reference)).toEqual(reference);
  for (const revision of [0, 9007199254740992, "0", "01", "+1", " 1"]) {
    expect(
      nativeReferenceSchema.safeParse({ ...reference, revision }).success,
    ).toBe(false);
  }
  const contract = {
    reference,
    name: "Finite counter",
    source_path: "counter.model.json",
    interface: "step",
    backend: "z3.finite.v1",
    class: "bounded",
    scope: "256 assignments",
    canonical: "original canonical bytes\nλ",
  };
  const artifact = {
    directory: "original-output",
    source_sha256: "c".repeat(64),
    artifact_sha256: "d".repeat(64),
  };
  const bodies = [
    { state: "empty" },
    { state: "proposed", contract, accepted: null, review: "Original review" },
    {
      state: "accepted",
      contract,
      review: "Original review",
      evidence: {
        state: "unverified",
        retained: null,
        failure: { kind: "inconclusive", message: "Original diagnostic" },
      },
    },
    {
      state: "accepted",
      contract,
      review: "Original review",
      evidence: {
        state: "current",
        artifact,
        completion_label: "CHECKED · complete within finite bounds",
      },
    },
  ];
  const fetch = vi.fn();
  vi.stubGlobal("fetch", fetch);
  const client = new NativeClient("session");
  for (const data of bodies) {
    const body = { revision, data };
    fetch.mockResolvedValue(Response.json(body));
    expect(await client.evidence(revision)).toEqual(body);
  }
  expect(
    fetch.mock.calls.every(
      ([url]) =>
        url === "/api/evidence?revision=" + encodeURIComponent(revision),
    ),
  ).toBe(true);
  fetch.mockImplementation(() =>
    Promise.resolve(Response.json({ revision, data: bodies[3] })),
  );
  await expect(client.evidence("sha256:" + "e".repeat(64))).rejects.toThrow(
    "does not match its requested revision",
  );
  for (const data of [
    { state: "empty", verified: true },
    {
      state: "proposed",
      contract,
      review: "Missing required nullable accepted",
    },
    { ...bodies[1], evidence: bodies[3].evidence },
    { ...bodies[2], evidence: { state: "unverified" } },
    { ...bodies[3], evidence: { ...bodies[3].evidence, grant: true } },
  ])
    expect(
      evidencePublicationSchema.safeParse({ revision, data }).success,
    ).toBe(false);
});

test("inventory and navigation reads bind exact publications and original parent/path", async () => {
  const file = {
    path: "A.lean",
    sha256: "",
    bytes: "18446744073709551615",
    declarations: 0,
    theorems: 0,
    indexed: false,
    imports: [],
    symbols: [],
  };
  const inventory = {
    revision,
    data: {
      generation: "0",
      files: [file],
      excluded: [],
      inventory_only: true,
    },
  };
  const fetch = vi
    .fn()
    .mockImplementation(() => Promise.resolve(Response.json(inventory)));
  vi.stubGlobal("fetch", fetch);
  const client = new NativeClient("session");
  expect(await client.inventory(revision)).toEqual(inventory);
  expect(fetch.mock.calls[0][0]).toBe(
    "/api/inventory?revision=" + encodeURIComponent(revision),
  );
  await expect(client.inventory("sha256:" + "b".repeat(64))).rejects.toThrow(
    "does not match its requested revision",
  );
  const chunk = {
    revision: "sha256:" + "c".repeat(64),
    data: { revision, files: [file] },
  };
  fetch.mockImplementation(() => Promise.resolve(Response.json(chunk)));
  expect(await client.map(revision, "A.lean")).toEqual(chunk.data);
  await expect(client.map(revision, "B.lean")).rejects.toThrow(
    "does not match",
  );
  await expect(
    client.map("sha256:" + "d".repeat(64), "A.lean"),
  ).rejects.toThrow("does not match");
});

test("reader queries bind complete literal text to an exact publication", async () => {
  const body = {
    revision,
    data: {
      path: "notes.txt",
      navigation: "source_only",
      reading: {
        state: "ready",
        content: {
          kind: "text",
          revision: "sha256:" + "b".repeat(64),
          bytes: 21,
          text: 'λ日本語\n"literal"',
        },
      },
    },
  };
  const fetch = vi
    .fn()
    .mockImplementation(() => Promise.resolve(Response.json(body)));
  vi.stubGlobal("fetch", fetch);
  const client = new NativeClient("session");
  expect(await client.reader(revision)).toEqual(body);
  expect(fetch.mock.calls[0][0]).toBe(
    "/api/reader?revision=" + encodeURIComponent(revision),
  );
  await expect(client.reader("sha256:" + "c".repeat(64))).rejects.toThrow(
    "does not match its requested revision",
  );
});

test("history requests bind the exact revision and retain original text", async () => {
  const fetch = vi.fn().mockResolvedValue(Response.json(snapshot));
  vi.stubGlobal("fetch", fetch);
  const abort = new AbortController();
  const result = await new NativeClient("session").history(
    revision,
    abort.signal,
  );
  expect(result).toEqual(snapshot);
  expect(fetch).toHaveBeenCalledExactlyOnceWith(
    "/api/history?revision=" + encodeURIComponent(revision),
    expect.objectContaining({
      signal: abort.signal,
      credentials: "omit",
      cache: "no-store",
      headers: { Authorization: "Bearer session" },
    }),
  );
});

test("a valid envelope for another revision cannot replace requested history", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json(snapshot)));
  await expect(
    new NativeClient("session").history("sha256:" + "b".repeat(64)),
  ).rejects.toThrow("does not match its requested revision");
});

test("stale recovery uses the HTTP status independently of diagnostic text", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValue(
        Response.json({ error: "Unrelated diagnostic" }, { status: 409 }),
      ),
  );
  const error = await new NativeClient("session")
    .history(revision)
    .catch((e) => e);
  expect(error).toBeInstanceOf(NativeHttpError);
  expect(error.status).toBe(409);
});

test("source publication identity includes presentation independently of its input revision", async () => {
  const source = {
    revision,
    data: {
      revision: "sha256:" + "b".repeat(64),
      files: { "Spec.rs": "fn original() {}\n" },
      roots: ["Spec.rs"],
      graph: null,
      description: "Original prepared scope",
      external: [],
      compiled: false,
      inputs_revision: null,
    },
  };
  const fetch = vi.fn().mockResolvedValue(Response.json(source));
  vi.stubGlobal("fetch", fetch);
  expect(await new NativeClient("session").source(revision)).toEqual(source);
  expect(fetch).toHaveBeenCalledWith(
    "/api/source?revision=" + encodeURIComponent(revision),
    expect.anything(),
  );
  fetch.mockResolvedValue(
    Response.json({ ...source, revision: source.data.revision }),
  );
  await expect(new NativeClient("session").source(revision)).rejects.toThrow(
    "does not match",
  );
  fetch.mockResolvedValue(
    Response.json({ ...source, data: { ...source.data, atlas: {} } }),
  );
  await expect(new NativeClient("session").source(revision)).rejects.toThrow();
});

test("preparation logs bind exact publications and outcome variants reject invented success", async () => {
  const job = "sha256:" + "b".repeat(64);
  const body = {
    revision,
    data: {
      job,
      resource: "A.lean",
      text: "λ日本語\ncompile",
      truncated: false,
    },
  };
  const fetch = vi
    .fn()
    .mockImplementation(() => Promise.resolve(Response.json(body)));
  vi.stubGlobal("fetch", fetch);
  expect(await new NativeClient("session").preparationLog(revision)).toEqual(
    body,
  );
  expect(fetch.mock.calls[0][0]).toBe(
    "/api/preparation-log?revision=" + encodeURIComponent(revision),
  );
  await expect(new NativeClient("session").preparationLog(job)).rejects.toThrow(
    "does not match",
  );
  const status = {
    job,
    resource: "A.lean",
    phase: "Compile",
    elapsed_ms: 9,
    phase_elapsed_ms: 4,
    limit_seconds: 10,
    log: { revision },
    outcome: { state: "preparing" },
  };
  expect(
    nativeStateSchema.shape.source_preparation.parse({ "A.lean": status }),
  ).toEqual({
    "A.lean": status,
  });
  expect(
    nativeStateSchema.shape.source_preparation.parse({
      "A.lean": {
        ...status,
        outcome: {
          state: "failed",
          kind: "operation",
          error: "𝕏".repeat(2000),
        },
      },
    }),
  ).toBeTruthy();
  expect(
    nativeStateSchema.shape.source_preparation.safeParse({
      "A.lean": { ...status, outcome: { state: "preparing", cache_hit: true } },
    }).success,
  ).toBe(false);
  expect(
    nativeStateSchema.shape.source_preparation.safeParse({
      "A.lean": {
        ...status,
        outcome: {
          state: "failed",
          kind: "cancelled",
          error: "host stop",
          cache_hit: true,
        },
      },
    }).success,
  ).toBe(false);
});
