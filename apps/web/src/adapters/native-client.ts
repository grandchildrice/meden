import {
  nativeStateSchema,
  preparationLogSchema,
  type PreparationLogPublication,
  directoryPublicationSchema,
  type DirectoryPublication,
  inventoryPublicationSchema,
  inventoryChunkSchema,
  type InventoryPublication,
  historySnapshotSchema,
  sourcePublicationSchema,
  readerPublicationSchema,
  type ReaderPublication,
  type SourcePublication,
  type HistorySnapshot,
  type NativeState,
  type NativeCommand,
} from "@meden/agent-protocol/native";

/** Typed transport failure; diagnostic text never chooses recovery behavior. */
export class NativeHttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}
/** Same-origin native adapter. Tokens never enter workspace/history exports. */
export class NativeClient {
  private readonly token: string;
  constructor(token: string) {
    this.token = token;
  }
  async state(signal?: AbortSignal): Promise<NativeState> {
    return nativeStateSchema.parse(
      await this.request("/api/state", undefined, signal),
    );
  }
  async command(command: NativeCommand): Promise<void> {
    await this.request("/api/command", command);
  }
  async history(
    revision: string,
    signal?: AbortSignal,
  ): Promise<HistorySnapshot> {
    return this.publication("history", revision, historySnapshotSchema, signal);
  }
  async source(
    revision: string,
    signal?: AbortSignal,
  ): Promise<SourcePublication> {
    return this.publication(
      "source",
      revision,
      sourcePublicationSchema,
      signal,
    );
  }
  async reader(
    revision: string,
    signal?: AbortSignal,
  ): Promise<ReaderPublication> {
    return this.publication(
      "reader",
      revision,
      readerPublicationSchema,
      signal,
    );
  }
  async directory(
    revision: string,
    signal?: AbortSignal,
  ): Promise<DirectoryPublication> {
    return this.publication(
      "directory",
      revision,
      directoryPublicationSchema,
      signal,
    );
  }
  async preparationLog(
    revision: string,
    signal?: AbortSignal,
  ): Promise<PreparationLogPublication> {
    return this.publication(
      "preparation-log",
      revision,
      preparationLogSchema,
      signal,
    );
  }
  private async publication<T extends { revision: string }>(
    role:
      | "history"
      | "source"
      | "reader"
      | "inventory"
      | "directory"
      | "preparation-log",
    revision: string,
    schema: { parse(value: unknown): T },
    signal?: AbortSignal,
  ): Promise<T> {
    const snapshot = schema.parse(
      await this.request(
        "/api/" + role + "?revision=" + encodeURIComponent(revision),
        undefined,
        signal,
      ),
    );
    if (snapshot.revision !== revision)
      throw new Error("View response does not match its requested revision");
    return snapshot;
  }
  async inventory(
    revision: string,
    signal?: AbortSignal,
  ): Promise<InventoryPublication> {
    return this.publication(
      "inventory",
      revision,
      inventoryPublicationSchema,
      signal,
    );
  }
  async map(revision: string, path: string) {
    const chunk = inventoryChunkSchema.parse(
      await this.request(
        "/api/map?inventory=" +
          encodeURIComponent(revision) +
          "&path=" +
          encodeURIComponent(path),
      ),
    );
    if (
      chunk.data.revision !== revision ||
      !chunk.data.files.some((file) => file.path === path)
    )
      throw new Error(
        "Navigation response does not match the requested inventory/file",
      );
    return chunk.data;
  }
  private async request(
    path: string,
    command?: NativeCommand,
    signal?: AbortSignal,
  ): Promise<unknown> {
    const response = await fetch(path, {
      method: command ? "POST" : "GET",
      signal,
      headers: {
        Authorization: `Bearer ${this.token}`,
        ...(command ? { "Content-Type": "application/json" } : {}),
      },
      body: command ? JSON.stringify(command) : undefined,
      credentials: "omit",
      cache: "no-store",
    });
    const value: unknown = await response.json();
    if (!response.ok)
      throw new NativeHttpError(
        response.status,
        typeof value === "object" && value && "error" in value
          ? String(value.error)
          : "Native session unavailable",
      );
    return value;
  }
}
