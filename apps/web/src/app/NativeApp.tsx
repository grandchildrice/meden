import { NativeClient, NativeHttpError } from "../adapters/native-client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Autocomplete,
  Button,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Tooltip,
  TextField,
  Typography,
} from "@mui/material";
import {
  type NativeCommand,
  type NativeReference,
  type NativeState,
  type HistorySnapshot,
  type WorkspaceOverview,
} from "@meden/agent-protocol/native";
import { ChatView } from "../features/chat/ChatView";
import type { Conversation } from "@meden/workspace-schema";
import { NativeDependencyGraph } from "../features/native/NativeDependencyGraph";
import { NativePreview } from "../features/native/NativePreview";
import { NativeRepair } from "../features/native/NativeRepair";
import { NativeDelivery } from "../features/native/NativeDelivery";
import { NativeWork } from "../features/native/NativeWork";
import {
  VerificationProgress,
  verificationActivity,
} from "../features/native/VerificationProgress";
import { visibleHistory } from "../features/native/history";
import HistoryOutlined from "@mui/icons-material/HistoryOutlined";
import {
  SidebarResize,
  useSidebarResize,
} from "../features/native/SidebarResize";
import type { Symbol } from "../features/native/project-map-model";
import { NativeFileWorkspace } from "../features/native/NativeFileWorkspace";
import Tune from "@mui/icons-material/Tune";
import PlayArrow from "@mui/icons-material/PlayArrow";
import Close from "@mui/icons-material/Close";
import { projectMap } from "../features/native/project-map-model";
import { NativeExplorer } from "../features/native/NativeExplorer";
import { NativeWorkspaceGraph } from "../features/native/NativeWorkspaceGraph";

/** A single host workspace is attached; choosing another requires a new host launch. */
import {
  usePublications,
  useRetainedPublication,
  type PublicationRead,
} from "../features/native/usePublications";

async function observePublication<T>(
  request: Promise<T>,
): Promise<PublicationRead<T>> {
  try {
    return { state: "ready", publication: await request };
  } catch (error) {
    if (error instanceof NativeHttpError && error.status === 409)
      return { state: "obsolete" };
    throw error;
  }
}

export function NativeApp({ token }: { token: string }) {
  const client = useMemo(() => new NativeClient(token), [token]);
  const [state, setState] = useState<NativeState>();
  const [history, setHistory] = useState<HistorySnapshot>();
  const historyData = history?.data;
  const [error, setError] = useState("");
  const [connectionError, setConnectionError] = useState("");
  const [previewOpen, setPreviewOpen] = useState(false);
  const [analysisFile, setAnalysisFile] = useState<string>();
  const openRequestedSnapshot = useRef(false);
  const [workspaceError, setWorkspaceError] = useState("");
  const [startupError, setStartupError] = useState("");
  const [files, setFiles] = useState<string[]>([]);
  const [activeFile, setActiveFile] = useState<string>();
  const [selectedSymbol, setSelectedSymbol] = useState<Symbol>();
  const [focusedSymbol, setFocusedSymbol] = useState<Symbol>();
  const [filesOpen, setFilesOpen] = useState(false);
  const [chatOpen, setChatOpen] = useState(false);
  const [readTarget, setReadTarget] = useState<{
    path: string;
    sequence: number;
  }>();
  const dispatchedRead = useRef<number | undefined>(undefined);
  const readSequence = useRef(0);
  const refreshEpoch = useRef(0);
  const fileEpochs = useRef(new Map<string, number>());
  const [utility, setUtility] = useState<
    "history" | "delivery" | "analysis" | null
  >(null);
  const [overview, setOverview] = useState<WorkspaceOverview>();
  const [overviewError, setOverviewError] = useState("");
  const [overviewCycle, setOverviewCycle] = useState(0);
  const mapEpoch = useRef(0);
  const trackedTargets = useMemo(
    () =>
      JSON.stringify(
        [
          ...new Set(
            historyData?.portable.document?.tasks.flatMap(
              (task) => task.theorems,
            ) || [],
          ),
        ].sort(),
      ),
    [historyData],
  );
  const scene = useMemo(
    () =>
      overview
        ? projectMap(overview, new Set<string>(JSON.parse(trackedTargets)))
        : undefined,
    [overview, trackedTargets],
  );
  const [provider, setProvider] = useState("");
  const [model, setModel] = useState("");
  const listedProvider = useRef("");
  const [graphPaths, setGraphPaths] = useState<string>();
  const [review, setReview] = useState<{
    reference: NativeReference;
    text: string;
  }>();
  const [reviewing, setReviewing] = useState<NativeReference>();
  const [approval, setApproval] = useState(false);
  const approving = useRef(false);
  const [approvalPending, setApprovalPending] = useState(false);
  const [processorHeading, setProcessorHeading] =
    useState<HTMLDivElement | null>(null);
  const [approvalVisible, setApprovalVisible] = useState(false);
  const processorIdentity = JSON.stringify([
    state?.endpoint,
    state?.model,
    state?.judge,
    state?.connection_revision,
  ]);
  useEffect(() => {
    setApprovalVisible(false);
    if (!approval || !processorHeading) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        setApprovalVisible(
          Boolean(entry?.isIntersecting && entry.intersectionRatio === 1),
        );
      },
      { threshold: [0, 1] },
    );
    observer.observe(processorHeading);
    return () => observer.disconnect();
  }, [approval, processorHeading, processorIdentity]);

  const initialMessageCount = useRef<number | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const started = useRef(0);
  const inFlight = useRef(0);
  const stateGeneration = useRef(0);
  const [commandActivity, setCommandActivity] = useState({
    generation: 0,
    pending: 0,
  });
  const busy = commandActivity.pending > 0;
  const [chatPending, setChatPending] = useState(false);
  const pendingCancellations = useRef(0);
  const pendingInspections = useRef(0);
  const [queue, setQueue] = useState<
    Array<{
      input: NativeCommand;
      done: (ok: boolean) => void;
    }>
  >([]);
  const [reconnect, setReconnect] = useState(0);
  useEffect(() => {
    const abort = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const generation = stateGeneration.current;
        const readable =
          inFlight.current ===
          pendingCancellations.current + pendingInspections.current;
        const next = await client.state(abort.signal);
        if (
          !abort.signal.aborted &&
          readable &&
          inFlight.current ===
            pendingCancellations.current + pendingInspections.current &&
          generation === stateGeneration.current
        ) {
          if (initialMessageCount.current === null) {
            initialMessageCount.current =
              next.history.state === "ready" ? next.history.messages : 0;
            if (next.stopped?.includes("checkpoint unavailable"))
              setStartupError(next.stopped);
          }
          setState(next);
          setConnectionError("");
          if (next.running) {
            if (!started.current) started.current = Date.now();
            setElapsed(Math.floor((Date.now() - started.current) / 1000));
          } else started.current = 0;
        }
      } catch (error) {
        if (!abort.signal.aborted)
          setConnectionError(
            `Connection unavailable. Check that meden gui is running. ${String(error)}`,
          );
      } finally {
        if (!abort.signal.aborted) timer = setTimeout(poll, 350);
      }
    };
    void poll();
    return () => {
      abort.abort();
      clearTimeout(timer);
    };
  }, [client, reconnect]);
  const historyRevision =
    state?.history.state === "ready" ? state.history.revision : undefined;
  const readHistory = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.history(revision, signal)),
    [client],
  );
  const historyReads = usePublications(
    historyRevision ? [historyRevision] : [],
    readHistory,
    state?.history,
    reconnect,
  );
  const historyPublication = historyReads.get(historyRevision)?.publication;
  const historyError = historyReads.get(historyRevision)?.error || "";
  useEffect(() => {
    if (historyPublication) setHistory(historyPublication);
  }, [historyPublication]);
  const analysisRevision =
    state?.analyses.current.state === "ready"
      ? state.analyses.current.revision
      : undefined;
  const analysisHistoryRevision =
    state?.analyses.history.state === "ready"
      ? state.analyses.history.revision
      : undefined;
  const readAnalysis = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.analysis(revision, signal)),
    [client],
  );
  const readAnalysisHistory = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.analysisHistory(revision, signal)),
    [client],
  );
  const analysisReads = usePublications(
    analysisRevision ? [analysisRevision] : [],
    readAnalysis,
    state?.analyses.current,
    reconnect,
  );
  const analysisHistoryReads = usePublications(
    analysisHistoryRevision ? [analysisHistoryRevision] : [],
    readAnalysisHistory,
    state?.analyses.history,
    reconnect,
  );
  const analysis = analysisReads.get(analysisRevision)?.publication?.data;
  const analysisHistory = analysisHistoryReads.get(analysisHistoryRevision)
    ?.publication?.data.history;
  const analysisReadError = analysisReads.get(analysisRevision)?.error;
  const analysisHistoryReadError = analysisHistoryReads.get(
    analysisHistoryRevision,
  )?.error;
  const analysesError = [
    state?.analyses.current.state === "unavailable"
      ? state.analyses.current.message
      : analysisReadError,
    state?.analyses.history.state === "unavailable"
      ? state.analyses.history.message
      : analysisHistoryReadError,
  ]
    .filter(Boolean)
    .join("\n");
  const evidenceRevision =
    state?.project.state === "ready" ? state.project.revision : undefined;
  const readEvidence = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.evidence(revision, signal)),
    [client],
  );
  const evidenceReads = usePublications(
    evidenceRevision ? [evidenceRevision] : [],
    readEvidence,
    state?.project,
    reconnect,
  );
  const project = evidenceReads.get(evidenceRevision)?.publication?.data;
  const evidenceError =
    state?.project.state === "unavailable"
      ? state.project.message
      : evidenceReads.get(evidenceRevision)?.error;
  const agreement =
    project && project.state !== "empty" ? project.contract : undefined;
  const previewRevision =
    state?.preview.state === "ready" ? state.preview.revision : undefined;
  const previewFrame =
    state?.preview.state === "ready" ? state.preview.frame : undefined;
  const readPreview = useCallback(
    (revision: string, signal: AbortSignal) => {
      if (!previewFrame)
        return Promise.reject(new Error("Preview context is unavailable"));
      return observePublication(client.preview(revision, previewFrame, signal));
    },
    [client, previewFrame],
  );
  const previewReads = usePublications(
    previewRevision ? [previewRevision] : [],
    readPreview,
    state?.preview,
    reconnect,
  );
  const previewData = useRetainedPublication(
    previewReads.get(previewRevision)?.publication?.data,
    previewFrame,
  );
  const previewError =
    state?.preview.state === "unavailable"
      ? state.preview.message
      : previewReads.get(previewRevision)?.error;
  const verificationRevision =
    state?.verification_job.state === "ready"
      ? state.verification_job.revision
      : undefined;
  const verificationJobId =
    state?.verification_job.state === "ready"
      ? state.verification_job.job
      : undefined;
  const readVerification = useCallback(
    (revision: string, signal: AbortSignal) => {
      if (!verificationJobId)
        return Promise.reject(new Error("Verification context is unavailable"));
      return observePublication(
        client.verificationJob(revision, verificationJobId, signal),
      );
    },
    [client, verificationJobId],
  );
  const verificationReads = usePublications(
    verificationRevision ? [verificationRevision] : [],
    readVerification,
    state?.verification_job,
    reconnect,
  );
  const observedVerification =
    verificationReads.get(verificationRevision)?.publication?.data;
  const verification = useRetainedPublication(
    observedVerification,
    verificationJobId,
  );
  const verificationError =
    state?.verification_job.state === "unavailable"
      ? state.verification_job.message
      : verificationReads.get(verificationRevision)?.error;
  const readDirectory = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.directory(revision, signal)),
    [client],
  );
  const readSource = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.source(revision, signal)),
    [client],
  );
  const preparedSources = usePublications(
    [
      ...(state?.sources.current ? [state.sources.current.revision] : []),
      ...files.flatMap((path) =>
        state?.sources.files[path] ? [state.sources.files[path]!.revision] : [],
      ),
    ],
    readSource,
    state?.sources,
    reconnect,
  );
  const readReader = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.reader(revision, signal)),
    [client],
  );
  const preparedReaders = usePublications(
    files.flatMap((path) =>
      state?.readers.files[path] ? [state.readers.files[path]!.revision] : [],
    ),
    readReader,
    state?.readers,
    reconnect,
  );
  const readPreparationLog = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.preparationLog(revision, signal)),
    [client],
  );
  const preparationLogs = usePublications(
    files.flatMap((path) =>
      state?.source_preparation[path]?.log
        ? [state.source_preparation[path]!.log!.revision]
        : [],
    ),
    readPreparationLog,
    state?.source_preparation,
    reconnect,
  );
  const source = preparedSources.get(state?.sources.current?.revision)
    ?.publication?.data;
  const inventoryRevision = state?.overview.revision || undefined;
  const inventoryEpoch = reconnect + overviewCycle;
  const readInventory = useCallback(
    (revision: string, signal: AbortSignal) =>
      observePublication(client.inventory(revision, signal)),
    [client],
  );
  const inventories = usePublications(
    inventoryRevision ? [inventoryRevision] : [],
    readInventory,
    state?.overview,
    inventoryEpoch,
  );
  const inventoryPublication = inventories.get(inventoryRevision)?.publication;
  const inventoryError = inventories.get(inventoryRevision)?.error;
  const observedInventory = useRef<string | undefined>(undefined);
  const currentInventory = useRef(inventoryRevision);
  currentInventory.current = inventoryRevision;
  useEffect(() => {
    if (
      !inventoryPublication ||
      inventoryPublication.revision === observedInventory.current
    )
      return;
    observedInventory.current = inventoryPublication.revision;
    setOverview(inventoryPublication.data);
    setOverviewError("");
  }, [inventoryPublication]);
  useEffect(() => {
    if (approval) {
      const choice =
        state?.connections.find((choice) => choice.id === state.provider) ||
        state?.connections[0];
      setProvider(choice?.id || "");
      setModel(state?.model || choice?.model || "");
    }
  }, [approval, state?.connection_revision]);
  useEffect(() => {
    if (
      !approval ||
      !provider ||
      !model.trim() ||
      running ||
      (provider === state?.provider && model === state?.model)
    )
      return;
    const timer = setTimeout(
      () => void command({ type: "select_connection", provider, model }),
      300,
    );
    return () => clearTimeout(timer);
  }, [
    approval,
    provider,
    model,
    state?.provider,
    state?.model,
    state?.running,
    busy,
  ]);
  // Freeze the reviewed body and exact identity only after the native review completed.
  useEffect(() => {
    if (reviewing && state && !state.running && project) {
      setReviewing(undefined);
      if (
        !state.stopped &&
        project.state !== "empty" &&
        JSON.stringify(project.contract.reference) === JSON.stringify(reviewing)
      )
        setReview({
          reference: reviewing,
          text: project.review,
        });
    }
  }, [reviewing, state, project]);
  useEffect(() => {
    if (graphPaths === undefined && source) {
      setGraphPaths((source.roots ?? Object.keys(source.files)).join("\n"));
    }
  }, [source?.revision, graphPaths]);
  const command = useCallback(
    async (command: NativeCommand) => {
      // These host operations are admitted independently of a running worker.
      // Never lose approval, revocation or cancellation to a background read.
      const immediate = [
        "approve_processor",
        "revoke_processor",
        "cancel",
      ].includes(command.type);
      if (inFlight.current && !immediate) return false;
      if (command.type === "refresh_workspace") refreshEpoch.current++;
      let generation = ++stateGeneration.current;
      inFlight.current++;
      let inspecting = command.type === "inspect";
      if (inspecting) pendingInspections.current++;
      if (command.type === "cancel") pendingCancellations.current++;
      setCommandActivity({ generation, pending: inFlight.current });
      try {
        await client.command(command);
        if (inspecting) {
          pendingInspections.current--;
          inspecting = false;
          // Publish one final read after acknowledgement. Earlier streaming
          // reads cannot arrive later and replace this command's snapshot.
          if (generation === stateGeneration.current)
            generation = ++stateGeneration.current;
        }
        const next = await client.state();
        if (command.type === "refresh_workspace") {
          next.sources = { current: null, files: {} };
          mapEpoch.current++;
          setOverview(undefined);
          observedInventory.current = undefined;
          setOverviewError("");
          setOverviewCycle((value) => value + 1);
        }
        if (generation === stateGeneration.current) setState(next);
        if (
          [
            "instruction",
            "resume_work",
            "approve_processor",
            "select_connection",
          ].includes(command.type)
        )
          setError("");
        else setWorkspaceError("");
        return true;
      } catch (error) {
        if (
          [
            "instruction",
            "resume_work",
            "approve_processor",
            "select_connection",
          ].includes(command.type)
        )
          setError(String(error));
        else setWorkspaceError(String(error));
        return false;
      } finally {
        if (inspecting) pendingInspections.current--;
        if (command.type === "cancel") {
          pendingCancellations.current--;
          // Polls may observe a completed stop before its command response.
          // Fence those reads once the final command snapshot is published.
          if (generation === stateGeneration.current) stateGeneration.current++;
        }
        inFlight.current--;
        setCommandActivity({
          generation: stateGeneration.current,
          pending: inFlight.current,
        });
      }
    },
    [client],
  );
  const openFile = useCallback(
    (path: string, mode: "read" | "retained" | "prepared" = "read") => {
      // A refresh invalidates retained navigation even when the last rendered
      // status still contains the previous host view. Ordinary tab returns reuse it.
      const needsRead =
        mode === "read" ||
        (mode === "retained" &&
          fileEpochs.current.has(path) &&
          fileEpochs.current.get(path) !== refreshEpoch.current);
      fileEpochs.current.set(path, refreshEpoch.current);
      setFiles((previous) =>
        previous.includes(path) ? previous : [...previous, path],
      );
      setActiveFile(path);
      setReadTarget(
        needsRead
          ? {
              path,
              sequence: ++readSequence.current,
            }
          : undefined,
      );
      setFilesOpen(false);
    },
    [],
  );
  const indexMapFile = useCallback(
    async (path: string) => {
      const epoch = mapEpoch.current;
      const parent = inventoryPublication?.revision;
      if (!parent || currentInventory.current !== parent) return false;
      try {
        const chunk = await client.map(parent, path);
        if (mapEpoch.current !== epoch || currentInventory.current !== parent)
          return false;
        setOverview((previous) => {
          if (!previous || observedInventory.current !== parent)
            return previous;
          const files = new Map(chunk.files.map((file) => [file.path, file]));
          return {
            ...previous,
            files: previous.files.map((file) => files.get(file.path) || file),
          };
        });
        return true;
      } catch (error) {
        if (mapEpoch.current !== epoch) return false;
        setOverviewError(`Could not index ${path}: ${String(error)}`);
        return true;
      }
    },
    [client, inventoryPublication],
  );
  const selectSymbol = useCallback(
    (symbol: Symbol) => {
      setSelectedSymbol(symbol);
      if (activeFile !== symbol.file) setFocusedSymbol(symbol);
      const prepared = Boolean(
        state?.sources.current &&
        state.sources.files[symbol.file]?.revision ===
          state.sources.current.revision,
      );
      if (activeFile !== symbol.file || !prepared)
        openFile(
          symbol.file,
          prepared
            ? "prepared"
            : !files.includes(symbol.file) ||
                (state?.sources.files[symbol.file] === undefined &&
                  state?.readers.files[symbol.file] === undefined)
              ? "read"
              : "retained",
        );
    },
    [openFile, activeFile, files, state?.sources],
  );
  const selectMapSymbol = useCallback(
    (symbol: Symbol) => {
      setFocusedSymbol(symbol);
      selectSymbol(symbol);
    },
    [selectSymbol],
  );
  const conversation = useMemo<Conversation | undefined>(
    () =>
      historyData
        ? {
            id: "native",
            title: state?.workspace.split("/").at(-1) || "Workspace",
            messages: visibleHistory(historyData),
          }
        : undefined,
    [historyData, state?.workspace],
  );
  const running = Boolean(state?.running || busy);
  const chatRunning = Boolean(
    state?.running && state.operation === "conversation",
  );
  const enqueue = useCallback(
    (input: NativeCommand) =>
      new Promise<boolean>((done) => {
        setChatPending(true);
        setQueue((previous) => [...previous, { input, done }]);
      }),
    [],
  );
  useEffect(() => {
    const next = queue[0];
    if (!next || running || inFlight.current) return;
    setQueue((previous) => previous.slice(1));
    void command(next.input).then((ok) => {
      next.done(ok);
      setChatPending(false);
    });
  }, [queue, running, command]);
  const sendMessage = useCallback(
    async (text: string) => {
      if (!(await enqueue({ type: "instruction", text })))
        throw new Error("Could not send. Your draft has been retained.");
    },
    [enqueue],
  );
  const cancelChat = useCallback(async () => {
    setChatPending(true);
    try {
      await command({ type: "cancel" });
    } finally {
      setChatPending(false);
    }
  }, [command]);
  const layout = useSidebarResize(state?.workspace);
  const stopNotice = useMemo(
    () =>
      state?.chat_stopped ? (
        <details
          className="chat-stop-notice"
          role={
            state.chat_stopped.startsWith("Stopped: Cancelled")
              ? undefined
              : "alert"
          }
        >
          <summary>
            {state.chat_stopped.startsWith("Stopped: Cancelled")
              ? "Response stopped"
              : "Response could not be completed"}
          </summary>
          <p>
            Your completed edits are retained. Send a new message to continue.
          </p>
          <pre>{state.chat_stopped}</pre>
        </details>
      ) : undefined,
    [state?.chat_stopped],
  );
  const composerActions = useMemo(
    () => (
      <div className="composer-model-tools">
        <Button
          className="composer-model"
          aria-label="Choose model"
          onClick={() => setApproval(true)}
        >
          {state?.model || "Choose model"}
          {state?.model && !state.authorized ? " · Not connected" : ""} ▾
        </Button>
        <Tooltip title="Work history">
          <IconButton
            aria-label="Work history"
            onClick={() => setUtility("history")}
          >
            <HistoryOutlined fontSize="small" />
          </IconButton>
        </Tooltip>
      </div>
    ),
    [state?.model, state?.authorized],
  );
  const toolsCommand = async (input: NativeCommand) => {
    if (input.type === "atlas_snapshot" || input.type === "source_snapshot") {
      const path = input.paths[0];
      openRequestedSnapshot.current = !path;
      setAnalysisFile(path);
      if (path) openFile(path, "prepared");
      setUtility(null);
    }
    return command(input);
  };
  useEffect(() => {
    if (source) setStartupError("");
    if (openRequestedSnapshot.current && source && !state?.running) {
      openRequestedSnapshot.current = false;
      const path = source.roots[0] || Object.keys(source.files)[0];
      if (path) {
        setAnalysisFile(path);
        openFile(path, "prepared");
      }
    }
  }, [source?.revision, state?.running]);
  useEffect(() => {
    if (
      readTarget &&
      !running &&
      !inFlight.current &&
      dispatchedRead.current !== readTarget.sequence
    ) {
      dispatchedRead.current = readTarget.sequence;
      void command({
        type: "inspect",
        path: readTarget.path,
        directory: false,
      });
    }
    // A fast background read can start and finish inside one React batch.
    // Its generation still changes even when the visible busy boolean does not.
  }, [readTarget, running, command, commandActivity]);
  useEffect(() => {
    if (!approval) {
      listedProvider.current = "";
      return;
    }
    if (
      provider &&
      !running &&
      !inFlight.current &&
      provider === state?.provider &&
      listedProvider.current !== provider + ":" + state?.connection_revision
    ) {
      listedProvider.current = provider + ":" + state?.connection_revision;
      void command({ type: "list_models", provider });
    }
  }, [
    approval,
    provider,
    state?.provider,
    state?.connection_revision,
    running,
  ]);
  const sessionLabels = useMemo(() => {
    const sessionLabels: Record<string, string> = {};
    let previousSession = "",
      sessionNumber = 0;
    for (const message of conversation?.messages || []) {
      const session = message.id.slice(0, message.id.lastIndexOf(":"));
      if (session !== previousSession) {
        sessionLabels[message.id] =
          session === "native"
            ? initialMessageCount.current
              ? "Previous conversation"
              : "Current session"
            : "Saved session " + ++sessionNumber;
        previousSession = session;
      }
    }
    if (initialMessageCount.current !== null)
      sessionLabels["native:" + initialMessageCount.current] =
        "Current session";
    return sessionLabels;
  }, [conversation?.messages, initialMessageCount.current]);

  return (
    <div className="native-shell native-ide">
      <header className="ide-header">
        <Typography component="h1">Meden</Typography>

        <Button
          className="ide-files-toggle"
          aria-expanded={filesOpen}
          onClick={() => setFilesOpen((value) => !value)}
        >
          Files
        </Button>
        <Button
          className="ide-chat-toggle"
          aria-expanded={chatOpen}
          onClick={() => setChatOpen((value) => !value)}
        >
          Chat
        </Button>
      </header>
      {!state ? (
        <div className="loading">
          <CircularProgress aria-label="Opening project" />
        </div>
      ) : (
        <main className="ide-main" ref={layout.ref} style={layout.style}>
          <aside
            className={"ide-files " + (filesOpen ? "is-open" : "")}
            aria-label="Files"
          >
            <NativeExplorer
              state={state}
              readDirectory={readDirectory}
              epoch={inventoryEpoch}
              active={activeFile}
              running={running}
              onCommand={command}
              onOpen={(path) =>
                openFile(
                  path,
                  !files.includes(path) ||
                    (state.sources.files[path] === undefined &&
                      state.readers.files[path] === undefined)
                    ? "read"
                    : "retained",
                )
              }
            />
          </aside>
          <SidebarResize side="files" layout={layout} />
          <section
            className="ide-workspace"
            aria-label="Implementation and results"
          >
            <div className="ide-editor-tabs">
              <div
                className="ide-file-tabs"
                role="tablist"
                aria-label="Open files"
              >
                <button
                  role="tab"
                  aria-selected={!activeFile}
                  onClick={() => setActiveFile(undefined)}
                >
                  Project map
                </button>
                {files.map((file) => {
                  const close = () => {
                    setFiles((prev) => prev.filter((p) => p !== file));
                    if (activeFile === file) setActiveFile(undefined);
                  };
                  return (
                    <button
                      key={file}
                      className={
                        "ide-file-tab " + (activeFile === file ? "active" : "")
                      }
                      role="tab"
                      aria-selected={activeFile === file}
                      aria-keyshortcuts="Delete"
                      title={file}
                      onClick={() =>
                        openFile(
                          file,
                          state.sources.files[file] === undefined &&
                            state.readers.files[file] === undefined
                            ? "read"
                            : "retained",
                        )
                      }
                      onKeyDown={(e) => {
                        if (e.key === "Delete") {
                          e.preventDefault();
                          close();
                        }
                      }}
                    >
                      <span>{file.split("/").at(-1)}</span>
                      <span
                        className="file-tab-close"
                        aria-hidden="true"
                        title={"Close: " + file}
                        onClick={(e) => {
                          e.stopPropagation();
                          close();
                        }}
                      >
                        <Close fontSize="inherit" />
                      </span>
                    </button>
                  );
                })}
                {previewOpen && (
                  <button
                    className={
                      "ide-file-tab " +
                      (activeFile === "__preview__" ? "active" : "")
                    }
                    role="tab"
                    aria-selected={activeFile === "__preview__"}
                    onClick={() => setActiveFile("__preview__")}
                  >
                    <span>App</span>
                    <span
                      className="file-tab-close"
                      aria-hidden="true"
                      title="Close preview"
                      onClick={(e) => {
                        e.stopPropagation();
                        setPreviewOpen(false);
                        setActiveFile(undefined);
                      }}
                    >
                      <Close fontSize="inherit" />
                    </span>
                  </button>
                )}
              </div>
              <div className="ide-tab-actions">
                <Tooltip title="Preview app">
                  <IconButton
                    aria-label="Preview app"
                    size="small"
                    onClick={() => {
                      setPreviewOpen(true);
                      setActiveFile("__preview__");
                    }}
                  >
                    <PlayArrow fontSize="small" />
                  </IconButton>
                </Tooltip>
                <Tooltip title="Lean tools">
                  <IconButton
                    aria-label="Lean tools"
                    size="small"
                    onClick={() => {
                      const path =
                        activeFile && activeFile !== "__preview__"
                          ? activeFile
                          : graphPaths ||
                            analysis?.input.target.file ||
                            agreement?.source_path;
                      setGraphPaths(path);
                      if (path && overview?.files.some((f) => f.path === path))
                        openFile(
                          path,
                          state.sources.files[path] === undefined &&
                            state.readers.files[path] === undefined
                            ? "read"
                            : "retained",
                        );
                      setUtility("analysis");
                    }}
                  >
                    <Tune fontSize="small" />
                  </IconButton>
                </Tooltip>
              </div>
            </div>
            {activeFile && activeFile !== "__preview__" && (
              <div className="ide-stage-toolbar">
                <code className="ide-file-path">{activeFile}</code>
              </div>
            )}
            {(workspaceError || startupError) && (
              <Alert severity="error">{workspaceError || startupError}</Alert>
            )}
            {state.operation === "workspace" &&
              !state.running &&
              state.stopped &&
              state.stopped !== state.chat_stopped &&
              !startupError &&
              !state.overview.error && (
                <Alert severity="error">{state.stopped}</Alert>
              )}
            <div className="ide-stage">
              <div className="project-map-page" hidden={Boolean(activeFile)}>
                <NativeWorkspaceGraph
                  error={inventoryError || overviewError}
                  loading={Boolean(
                    inventoryRevision &&
                    !inventoryPublication &&
                    !inventoryError,
                  )}
                  onRetry={
                    inventoryError && inventoryRevision
                      ? () => inventories.retry(inventoryRevision)
                      : undefined
                  }
                  active={!activeFile}
                  canIndex={
                    !running &&
                    queue.length === 0 &&
                    inventoryPublication?.revision === currentInventory.current
                  }
                  onIndex={indexMapFile}
                  scene={scene}
                  state={state}
                  selectedId={selectedSymbol?.id}
                  onSelect={selectMapSymbol}
                />
              </div>
              {files.map((file) => (
                <div
                  key={file}
                  className="ide-file-page"
                  hidden={activeFile !== file}
                >
                  <NativeFileWorkspace
                    state={state}
                    analysis={analysis}
                    history={analysisHistory}
                    analysesLoading={Boolean(
                      !analysesError &&
                      ((analysisRevision && !analysis) ||
                        (state.analyses.history.state === "ready" &&
                          state.analyses.history.runs > 0 &&
                          !analysisHistory)),
                    )}
                    analysesError={analysesError}
                    onRetryAnalyses={
                      analysisReadError || analysisHistoryReadError
                        ? () => {
                            if (analysisReadError && analysisRevision)
                              analysisReads.retry(analysisRevision);
                            if (
                              analysisHistoryReadError &&
                              analysisHistoryRevision
                            )
                              analysisHistoryReads.retry(
                                analysisHistoryRevision,
                              );
                          }
                        : undefined
                    }
                    file={
                      preparedReaders.get(state.readers.files[file]?.revision)
                        ?.publication?.data
                    }
                    readerError={
                      preparedReaders.get(state.readers.files[file]?.revision)
                        ?.error
                    }
                    onRetryReader={() => {
                      const revision = state.readers.files[file]?.revision;
                      if (revision) preparedReaders.retry(revision);
                    }}
                    snapshot={
                      preparedSources.get(state.sources.files[file]?.revision)
                        ?.publication?.data
                    }
                    sourceError={
                      preparedSources.get(state.sources.files[file]?.revision)
                        ?.error
                    }
                    onRetrySource={() => {
                      const revision = state.sources.files[file]?.revision;
                      if (revision) preparedSources.retry(revision);
                    }}
                    preparationLog={
                      preparationLogs.get(
                        state.source_preparation[file]?.log?.revision,
                      )?.publication?.data
                    }
                    preparationLogError={
                      preparationLogs.get(
                        state.source_preparation[file]?.log?.revision,
                      )?.error
                    }
                    onRetryLog={() => {
                      const revision =
                        state.source_preparation[file]?.log?.revision;
                      if (revision) preparationLogs.retry(revision);
                    }}
                    path={file}
                    scene={scene}
                    revision={inventoryPublication?.revision || ""}
                    selected={selectedSymbol}
                    focused={focusedSymbol}
                    running={running}
                    analysisFile={analysisFile}
                    onCommand={command}
                    onSelect={selectSymbol}
                  />
                </div>
              ))}
              {previewOpen && (
                <div
                  className="ide-preview-page"
                  hidden={activeFile !== "__preview__"}
                >
                  <NativePreview
                    snapshot={previewData ?? null}
                    loading={!!previewRevision && !previewData && !previewError}
                    transferError={previewError}
                    onRetry={
                      previewRevision
                        ? () => previewReads.retry(previewRevision)
                        : undefined
                    }
                    error={
                      state.operation === "preview" && !state.running
                        ? state.stopped
                        : null
                    }
                    running={running}
                    onCapture={(root, entry) =>
                      command({ type: "capture_preview", root, entry })
                    }
                  />
                </div>
              )}
            </div>
          </section>
          <SidebarResize side="chat" layout={layout} />
          <aside
            className={"native-chat ide-chat " + (chatOpen ? "is-open" : "")}
            aria-label="Workspace chat"
          >
            {(error ||
              connectionError ||
              historyData?.portable.error ||
              historyError ||
              state.history.state === "unavailable") && (
              <Alert
                severity="error"
                action={
                  connectionError ? (
                    <Button onClick={() => setReconnect((n) => n + 1)}>
                      Reconnect
                    </Button>
                  ) : historyError ? (
                    <Button
                      onClick={() => {
                        if (historyRevision)
                          historyReads.retry(historyRevision);
                      }}
                    >
                      Retry history
                    </Button>
                  ) : undefined
                }
              >
                {error ||
                  connectionError ||
                  historyError ||
                  (state.history.state === "unavailable" &&
                    state.history.message) ||
                  (historyData?.portable.error &&
                    "Could not read saved conversations and work history. " +
                      historyData.portable.error)}
              </Alert>
            )}
            <ChatView
              compact
              projectName={state.workspace.split("/").at(-1) || "Workspace"}
              conversation={conversation}
              sessionLabels={sessionLabels}
              draftKey={"meden-native-input:" + state.workspace}
              running={chatRunning || queue.length > 0}
              pending={chatPending}
              activityText={
                queue.length > 0
                  ? "Waiting to send…"
                  : chatRunning
                    ? (verificationActivity(
                        verification,
                        Boolean(verificationRevision),
                      ) ?? state.activity + " · " + elapsed + "s")
                    : undefined
              }
              disabled={!state.authorized || Boolean(connectionError)}
              composerActions={composerActions}
              notice={stopNotice}
              artifact={null}
              onSend={sendMessage}
            />
            {utility === "delivery" && (
              <NativeDelivery
                exports={state.exports}
                review={state.export_review}
                outcome={state.export_outcome}
                running={running}
                onCommand={command}
              />
            )}
            <div className="chat-utilities">
              {state.exports.length > 0 && (
                <Button onClick={() => setUtility("delivery")}>
                  Review delivery
                </Button>
              )}
              {chatRunning && (
                <Button onClick={() => void cancelChat()}>Stop</Button>
              )}
            </div>
            {chatRunning && state.partial && (
              <pre className="native-stream" aria-label="Live output">
                {state.partial}
              </pre>
            )}
            {evidenceError && (
              <article className="chat-contract">
                <Alert severity="warning">
                  Could not load verification requirements and evidence.
                  <details>
                    <summary>Details</summary>
                    {evidenceError}
                  </details>
                  {evidenceRevision && (
                    <Button
                      onClick={() => evidenceReads.retry(evidenceRevision)}
                    >
                      Retry evidence
                    </Button>
                  )}
                </Alert>
              </article>
            )}
            {reviewing && !project && !evidenceError && (
              <p role="status">Loading contract review…</p>
            )}
            <VerificationProgress
              data={verification}
              error={verificationError}
              retry={
                verificationRevision
                  ? () => verificationReads.retry(verificationRevision)
                  : undefined
              }
            />
            {project && project.state !== "empty" && (
              <article className="chat-contract">
                <Typography component="h3">
                  Verification requirements
                </Typography>
                <Chip
                  size="small"
                  label={
                    project.state === "accepted" &&
                    project.evidence.state === "current"
                      ? project.evidence.completion_label
                      : project.state === "accepted"
                        ? "Contract accepted"
                        : "Proposal"
                  }
                />
                <pre>{project.review}</pre>
                <Button
                  disabled={running}
                  onClick={() => void command({ type: "refresh_status" })}
                >
                  Check current evidence
                </Button>
                {project.state === "proposed" && (
                  <Button
                    disabled={running}
                    onClick={async () => {
                      if (
                        await command({
                          type: "review_contract",
                          reference: project.contract.reference,
                        })
                      )
                        setReviewing(project.contract.reference);
                    }}
                  >
                    Review proposed contract
                  </Button>
                )}
              </article>
            )}
          </aside>
        </main>
      )}
      {state && (
        <Dialog
          open={utility === "history" || utility === "analysis"}
          onClose={() => setUtility(null)}
          fullWidth
          maxWidth="sm"
        >
          <DialogTitle>
            {utility === "history" ? "Work history" : "Lean analysis"}
            <Button onClick={() => setUtility(null)}>Close</Button>
          </DialogTitle>
          <DialogContent>
            {utility === "history" ? (
              historyData ? (
                <NativeWork
                  workspace={historyData.portable}
                  running={running}
                  authorized={state.authorized}
                  onCommand={command}
                  onInspect={(file) => {
                    setUtility(null);
                    openFile(file);
                  }}
                />
              ) : (
                <CircularProgress aria-label="Loading work history" size={24} />
              )
            ) : (
              <NativeDependencyGraph
                state={state}
                overview={overview}
                paths={graphPaths ?? ""}
                onPaths={setGraphPaths}
                onCommand={toolsCommand}
                running={running}
              />
            )}
            {utility === "history" &&
              historyData &&
              historyData.logs.length > 0 && (
                <section aria-label="Tool records">
                  <h3>Tool records</h3>
                  {historyData.logs.map((entry, i) => (
                    <details key={i}>
                      <summary>
                        {entry.type === "notice" ? "Save notice" : entry.tool}
                      </summary>
                      {entry.type === "notice" ? (
                        <Alert severity="warning">{entry.text}</Alert>
                      ) : (
                        <pre>{entry.text}</pre>
                      )}
                    </details>
                  ))}
                </section>
              )}
          </DialogContent>
        </Dialog>
      )}
      <NativeRepair
        review={state?.repair_review || null}
        running={running}
        onCommand={command}
      />
      <Dialog
        open={approval}
        onClose={() => setApproval(false)}
        fullWidth
        maxWidth="sm"
      >
        <DialogTitle
          component="div"
          ref={setProcessorHeading}
          sx={{ overflowWrap: "anywhere" }}
        >
          <Typography component="h2" variant="h3">
            Model
          </Typography>
          <TextField
            select
            fullWidth
            label="provider"
            value={provider}
            slotProps={{ select: { native: true } }}
            sx={{ mt: 1 }}
            onChange={(event) => {
              const id = event.target.value;
              setProvider(id);
              setModel(
                state?.connections.find((choice) => choice.id === id)?.model ||
                  "",
              );
            }}
          >
            <option value="">Choose connection</option>
            {state?.connections.map((choice) => (
              <option key={choice.id} value={choice.id}>
                {choice.label} · {choice.id}
              </option>
            ))}
          </TextField>
          <Autocomplete
            freeSolo
            options={[
              ...new Set(
                [
                  state?.connections.find((choice) => choice.id === provider)
                    ?.model || "",
                  ...(state?.models_provider === provider ? state.models : []),
                ].filter(Boolean),
              ),
            ]}
            inputValue={model}
            onInputChange={(_, value) => setModel(value)}
            value={model || null}
            onChange={(_, value) => setModel(value || "")}
            renderInput={(params) => (
              <TextField {...params} label="model" sx={{ mt: 2 }} />
            )}
          />
          {state?.models_provider === provider && state.models_error && (
            <Alert severity="info">
              Could not load models. Choose a configured model or enter a model
              ID.
            </Alert>
          )}
          {state?.connections.length === 0 && (
            <Alert severity="info">
              No connection settings found. Sign in with Hermes, Codex or
              Claude, or start an Ollama-compatible server, then reopen the GUI.
            </Alert>
          )}
          <Typography
            className="model-destination"
            variant="caption"
            component="p"
          >
            {state?.endpoint}
          </Typography>
          {state?.judge && (
            <Typography variant="caption">
              {state.judge.model} · {state.judge.endpoint}
            </Typography>
          )}
        </DialogTitle>
        <DialogContent>
          {error && <Alert severity="error">{error}</Alert>}
          {state?.authorized && (
            <Button
              color="warning"
              onClick={() => void command({ type: "revoke_processor" })}
            >
              Revoke access
            </Button>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setApproval(false)}>Close</Button>
          <Button
            disabled={
              !state?.approval_available ||
              state.connection_revision === null ||
              provider !== state.provider ||
              model !== state.model ||
              state.authorized ||
              approvalPending ||
              !approvalVisible
            }
            onClick={async () => {
              if (
                !state ||
                state.connection_revision === null ||
                approving.current
              )
                return;
              approving.current = true;
              setApprovalPending(true);
              try {
                if (
                  await command({
                    type: "approve_processor",
                    connection_revision: state.connection_revision,
                    endpoint: state.endpoint,
                    model: state.model,
                    judge: state.judge,
                  })
                )
                  setApproval(false);
              } finally {
                approving.current = false;
                setApprovalPending(false);
              }
            }}
          >
            Use this model
          </Button>
        </DialogActions>
      </Dialog>
      <Dialog
        open={Boolean(review)}
        onClose={() => setReview(undefined)}
        fullWidth
        maxWidth="md"
      >
        <DialogTitle>Review contract meaning</DialogTitle>
        <DialogContent>
          {error && <Alert severity="error">{error}</Alert>}
          <Typography sx={{ overflowWrap: "anywhere" }}>
            Contract: {review?.reference.contract} · revision{" "}
            {review?.reference.revision}
          </Typography>
          <pre className="native-contract">{review?.text}</pre>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setReview(undefined)}>Close</Button>
          <Button
            disabled={running}
            onClick={async () => {
              if (
                review &&
                (await command({
                  type: "reject_contract",
                  reference: review.reference,
                }))
              )
                setReview(undefined);
            }}
          >
            Reject
          </Button>
          <Button
            disabled={running}
            onClick={async () => {
              if (
                review &&
                (await command({
                  type: "accept_contract",
                  reference: review.reference,
                }))
              )
                setReview(undefined);
            }}
          >
            Accept this contract and continue
          </Button>
        </DialogActions>
      </Dialog>
    </div>
  );
}
