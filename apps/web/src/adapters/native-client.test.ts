import { afterEach, expect, test, vi } from "vitest";
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
