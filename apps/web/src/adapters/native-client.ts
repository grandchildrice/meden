import {
  nativeStateSchema,
  workspaceOverviewSchema,
  historySnapshotSchema,
  type HistorySnapshot,
  type NativeState,
  type NativeCommand,
  type WorkspaceOverview,
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
    const snapshot = historySnapshotSchema.parse(
      await this.request(
        "/api/history?revision=" + encodeURIComponent(revision),
        undefined,
        signal,
      ),
    );
    if (snapshot.revision !== revision)
      throw new Error("History response does not match its requested revision");
    return snapshot;
  }
  async overview(signal?: AbortSignal): Promise<WorkspaceOverview> {
    return workspaceOverviewSchema.parse(
      await this.request("/api/overview", undefined, signal),
    );
  }
  async map(path: string) {
    return workspaceOverviewSchema
      .pick({ revision: true, files: true })
      .parse(await this.request("/api/map?path=" + encodeURIComponent(path)));
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
