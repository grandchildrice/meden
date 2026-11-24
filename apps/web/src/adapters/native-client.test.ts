import { afterEach, expect, test, vi } from "vitest";
import {
  evidencePublicationSchema,
  nativeReferenceSchema,
  nativeStateSchema,
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
