import {
  nativeStateSchema,
  previewPublicationSchema,
  type PreviewPublication,
  evidencePublicationSchema,
  type EvidencePublication,
  verificationJobPublicationSchema,
  type VerificationJobPublication,
  analysisCurrentSchema,
  analysisHistorySchema,
  type AnalysisCurrentPublication,
  type AnalysisHistoryPublication,
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
  async evidence(
    revision: string,
    signal?: AbortSignal,
  ): Promise<EvidencePublication> {
    return this.publication(
      "evidence",
      revision,
      evidencePublicationSchema,
      signal,
    );
  }
  async preview(
    revision: string,
    frame: string,
    signal?: AbortSignal,
  ): Promise<PreviewPublication> {
    const publication = await this.publication(
      "preview",
      revision,
      previewPublicationSchema,
      signal,
    );
    if (publication.data.frame !== frame)
      throw new Error("Preview description does not match its requested frame");
    return publication;
  }
  async verificationJob(
    revision: string,
    job: string,
    signal?: AbortSignal,
  ): Promise<VerificationJobPublication> {
    const publication = await this.publication(
      "verification-job",
      revision,
      verificationJobPublicationSchema,
      signal,
    );
    if (publication.data.context.job !== job)
      throw new Error(
        "Verification observation does not match its requested job",
      );
    return publication;
  }
  async history(
    revision: string,
    signal?: AbortSignal,
  ): Promise<HistorySnapshot> {
    return this.publication("history", revision, historySnapshotSchema, signal);
  }
  async analysis(
    revision: string,
    signal?: AbortSignal,
  ): Promise<AnalysisCurrentPublication> {
    return this.publication(
      "analysis",
      revision,
      analysisCurrentSchema,
      signal,
    );
  }
  async analysisHistory(
    revision: string,
    signal?: AbortSignal,
  ): Promise<AnalysisHistoryPublication> {
    return this.publication(
      "analysis-history",
      revision,
      analysisHistorySchema,
      signal,
    );
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
      | "preview"
      | "history"
      | "source"
      | "reader"
      | "inventory"
      | "directory"
      | "preparation-log"
      | "analysis"
      | "analysis-history"
      | "evidence"
      | "verification-job",
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
