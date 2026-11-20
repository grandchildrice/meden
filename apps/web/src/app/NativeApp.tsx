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
  Tab,
  Tabs,
  TextField,
  Typography,
} from "@mui/material";
import {
  NativeClient,
  type NativeCommand,
  type NativeReference,
  type NativeState,
  type WorkspaceOverview,
} from "@meden/agent-protocol/native";
import { ChatView } from "../features/chat/ChatView";
import type { Conversation } from "@meden/workspace-schema";
import { NativeDependencyGraph } from "../features/native/NativeDependencyGraph";
import { NativePreview } from "../features/native/NativePreview";
import { NativeRepair } from "../features/native/NativeRepair";
import { NativeDelivery } from "../features/native/NativeDelivery";
import { NativeWork } from "../features/native/NativeWork";
import { visibleHistory } from "../features/native/history";
import { NativeHistory } from "../features/native/NativeHistory";
import type { Symbol } from "../features/native/project-map-model";
import { NativeCode, NativeExplorer } from "../features/native/NativeExplorer";
import { NativeWorkspaceGraph } from "../features/native/NativeWorkspaceGraph";

/** A single host workspace is attached; choosing another requires a new host launch. */
export function NativeApp({ token }: { token: string }) {
  const client = useMemo(() => new NativeClient(token), [token]);
  const [state, setState] = useState<NativeState>();
  const [error, setError] = useState("");
  const [connectionError, setConnectionError] = useState("");
  const [mode, setMode] = useState("graph");
  const [files, setFiles] = useState<string[]>([]);
  const [activeFile, setActiveFile] = useState<string>();
  const [selectedSymbol, setSelectedSymbol] = useState<Symbol>();
  const [focusSequence, setFocusSequence] = useState(0);
  const [filesOpen, setFilesOpen] = useState(false);
  const [chatOpen, setChatOpen] = useState(false);
  const [readTarget, setReadTarget] = useState<{
    path: string;
    offset: number;
    sequence: number;
  }>();
  const dispatchedRead = useRef<number | undefined>(undefined);
  const [utility, setUtility] = useState<
    "history" | "logs" | "delivery" | "analysis" | null
  >(null);
  const [overview, setOverview] = useState<WorkspaceOverview>();
  const [overviewError, setOverviewError] = useState("");
  const [provider, setProvider] = useState("");
  const [model, setModel] = useState("");
  const listedProvider = useRef("");
  const [graphPaths, setGraphPaths] = useState<string>();
  const [review, setReview] = useState<{
    reference: NativeReference;
    text: string;
  }>();
  const [reviewing, setReviewing] = useState(false);
  const [approval, setApproval] = useState(false);
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
  const inFlight = useRef(false);
  const [busy, setBusy] = useState(false);
  const [reconnect, setReconnect] = useState(0);
  useEffect(() => {
    const abort = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await client.state(abort.signal);
        if (!abort.signal.aborted) {
          if (initialMessageCount.current === null)
            initialMessageCount.current = next.messages.length;
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
            `接続できません。meden guiが起動していることを確認してください。${String(error)}`,
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
  useEffect(() => {
    const abort = new AbortController();
    if (!state?.overview.revision) {
      return;
    }
    void client
      .overview(abort.signal)
      .then((snapshot) => {
        if (!abort.signal.aborted) {
          setOverview(snapshot);
          setOverviewError("");
        }
      })
      .catch((error) => {
        if (!abort.signal.aborted) setOverviewError(String(error));
      });
    return () => abort.abort();
  }, [client, state?.overview.revision, reconnect]);
  useEffect(() => {
    if (approval) {
      const choice =
        state?.connections.find((choice) => choice.id === state.provider) ||
        state?.connections[0];
      setProvider(choice?.id || "");
      setModel(state?.model || choice?.model || "");
    }
  }, [approval, state?.connection_revision]);
  // Freeze the reviewed body and exact identity only after the native review completed.
  useEffect(() => {
    if (reviewing && state && !state.running) {
      setReviewing(false);
      if (!state.stopped)
        setReview({
          reference: state.project.reference,
          text: state.project.review,
        });
    }
  }, [reviewing, state]);
  useEffect(() => {
    if (graphPaths === undefined && state?.source_snapshot) {
      setGraphPaths(
        (
          state.source_snapshot.collection?.roots ??
          Object.keys(state.source_snapshot.files)
        ).join("\n"),
      );
    }
  }, [state?.source_snapshot?.revision, graphPaths]);
  const command = useCallback(
    async (command: NativeCommand) => {
      if (inFlight.current) return false;
      inFlight.current = true;
      setBusy(true);
      try {
        await client.command(command);
        setState(await client.state());
        setError("");
        return true;
      } catch (error) {
        setError(String(error));
        return false;
      } finally {
        inFlight.current = false;
        setBusy(false);
      }
    },
    [client],
  );
  const openFile = useCallback((path: string, code = true, line?: number) => {
    setFiles((previous) =>
      previous.includes(path) ? previous : [...previous, path],
    );
    setActiveFile(path);
    if (code) setMode("code");
    setReadTarget((previous) => ({
      path,
      offset: line ? Math.max(1, line - 5) : 1,
      sequence: (previous?.sequence || 0) + 1,
    }));
    setFilesOpen(false);
  }, []);
  const selectSymbol = useCallback(
    (symbol: Symbol) => {
      setSelectedSymbol(symbol);
      openFile(symbol.file, false, symbol.line);
    },
    [openFile],
  );
  const conversation: Conversation | undefined = state && {
    id: "native",
    title: state.workspace.split("/").at(-1) || "Workspace",
    messages: visibleHistory(state),
  };
  const running = Boolean(state?.running || busy);
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
        offset: readTarget.offset,
      });
    }
  }, [readTarget, running, command]);
  useEffect(() => {
    if (!approval) {
      listedProvider.current = "";
      return;
    }
    if (
      provider &&
      !running &&
      !inFlight.current &&
      listedProvider.current !== provider
    ) {
      listedProvider.current = provider;
      void command({ type: "list_models", provider });
    }
  }, [approval, provider, running]);
  const sessionLabels: Record<string, string> = {};
  let previousSession = "",
    sessionNumber = 0;
  for (const message of conversation?.messages || []) {
    const session = message.id.slice(0, message.id.lastIndexOf(":"));
    if (session !== previousSession) {
      sessionLabels[message.id] =
        session === "native"
          ? initialMessageCount.current
            ? "前回までの会話"
            : "現在のセッション"
          : "保存セッション " + ++sessionNumber;
      previousSession = session;
    }
  }
  if (initialMessageCount.current !== null)
    sessionLabels["native:" + initialMessageCount.current] = "現在のセッション";
  return (
    <div className="native-shell native-ide">
      <header className="ide-header">
        <Typography component="h1">Meden</Typography>
        <Typography className="ide-project-name" title={state?.workspace}>
          {state?.workspace.split("/").at(-1)}
        </Typography>
        <Button
          className="ide-files-toggle"
          aria-expanded={filesOpen}
          onClick={() => setFilesOpen((value) => !value)}
        >
          ファイル
        </Button>
        <Button
          className="ide-chat-toggle"
          aria-expanded={chatOpen}
          onClick={() => setChatOpen((value) => !value)}
        >
          チャット
        </Button>
      </header>
      {!state ? (
        <div className="loading">
          <CircularProgress aria-label="プロジェクトを開いています" />
        </div>
      ) : (
        <main className="ide-main">
          <aside
            className={"ide-files " + (filesOpen ? "is-open" : "")}
            aria-label="ファイル"
          >
            <NativeExplorer
              state={state}
              active={activeFile}
              running={running}
              onCommand={command}
              onOpen={openFile}
            />
          </aside>
          <section className="ide-workspace" aria-label="実装と実行結果">
            <div
              className="ide-file-tabs"
              role="tablist"
              aria-label="開いているファイル"
            >
              <button
                role="tab"
                aria-selected={!activeFile}
                onClick={() => {
                  setActiveFile(undefined);
                  setMode("graph");
                  setFocusSequence((value) => value + 1);
                }}
              >
                プロジェクトマップ
              </button>
              {files.map((file) => (
                <div key={file} className="ide-file-tab">
                  <button
                    role="tab"
                    aria-selected={activeFile === file}
                    title={file}
                    onClick={() => openFile(file)}
                  >
                    {file.split("/").at(-1)}
                  </button>
                  <button
                    aria-label={"閉じる: " + file}
                    onClick={() => {
                      setFiles((previous) =>
                        previous.filter((path) => path !== file),
                      );
                      if (activeFile === file) {
                        setActiveFile(undefined);
                        setMode("graph");
                      }
                    }}
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>
            <div className="ide-stage-toolbar">
              <Typography
                variant="caption"
                className="ide-file-path"
                title={activeFile}
              >
                {activeFile || "プロジェクト"}
              </Typography>
              <Tabs
                value={mode}
                onChange={(_, value: string) => {
                  setMode(value);
                  if (
                    value === "graph" &&
                    activeFile &&
                    selectedSymbol?.file !== activeFile
                  )
                    setFocusSequence((n) => n + 1);
                }}
                aria-label="表示モード"
              >
                <Tab value="graph" label="グラフ" />
                <Tab value="code" label="コード" disabled={!activeFile} />
                <Tab value="preview" label="プレビュー" />
              </Tabs>
              <Button onClick={() => setUtility("analysis")}>解析の詳細</Button>
            </div>
            <div className="ide-stage">
              {utility === "analysis" && (
                <div className="ide-analysis-page">
                  <Button onClick={() => setUtility(null)}>閉じる</Button>
                  <NativeDependencyGraph
                    state={state}
                    overview={overview}
                    paths={
                      graphPaths ?? activeFile ?? state.project.source_path
                    }
                    onPaths={setGraphPaths}
                    onCommand={command}
                    running={running}
                    elapsed={elapsed}
                    connectionError={connectionError}
                  />
                </div>
              )}

              <div
                className="project-map-page"
                hidden={mode !== "graph" || utility === "analysis"}
              >
                {overviewError && (
                  <Alert severity="error">{overviewError}</Alert>
                )}
                <NativeWorkspaceGraph
                  snapshot={overview}
                  state={state}
                  focusPath={activeFile}
                  focusSequence={focusSequence}
                  selectedId={selectedSymbol?.id}
                  onSelect={selectSymbol}
                  onRefresh={() => void command({ type: "refresh_workspace" })}
                />
              </div>
              <div
                className="ide-code-page"
                hidden={mode !== "code" || utility === "analysis"}
              >
                <NativeCode
                  state={state}
                  path={activeFile}
                  line={
                    selectedSymbol?.file === activeFile
                      ? selectedSymbol?.line
                      : undefined
                  }
                  running={running}
                  onCommand={command}
                />
              </div>
              <div
                className="ide-preview-page"
                hidden={mode !== "preview" || utility === "analysis"}
              >
                <NativePreview
                  snapshot={state.preview}
                  running={running}
                  onCapture={(root, entry) =>
                    command({ type: "capture_preview", root, entry })
                  }
                />
              </div>
            </div>
          </section>
          <aside
            className={"native-chat ide-chat " + (chatOpen ? "is-open" : "")}
            aria-label="作業チャット"
          >
            {(error || connectionError || state.portable.error) && (
              <Alert
                severity="error"
                action={
                  connectionError ? (
                    <Button onClick={() => setReconnect((n) => n + 1)}>
                      再接続
                    </Button>
                  ) : undefined
                }
              >
                {error ||
                  connectionError ||
                  (state.portable.error &&
                    "保存済みの会話・作業履歴を読み込めませんでした。" +
                      state.portable.error)}
              </Alert>
            )}
            {state.stopped && utility !== "analysis" && (
              <Alert
                severity={state.activity === "Complete" ? "info" : "warning"}
                sx={{ whiteSpace: "pre-wrap" }}
              >
                {state.stopped}
              </Alert>
            )}
            <ChatView
              projectName={conversation!.title}
              conversation={conversation}
              sessionLabels={sessionLabels}
              draftKey={"meden-native-input:" + state.workspace}
              running={running}
              activityText={state.activity + " · " + elapsed + "秒"}
              disabled={!state.authorized || Boolean(connectionError)}
              composerActions={
                <Button
                  className="composer-model"
                  aria-label="モデルを選択"
                  onClick={() => setApproval(true)}
                >
                  {state.model || "モデルを選択"}
                  {state.model && !state.authorized ? " · 未接続" : ""} ▾
                </Button>
              }
              history={
                <NativeHistory
                  state={state}
                  running={running}
                  onCommand={command}
                  onOpen={openFile}
                  onManage={() => setUtility("history")}
                />
              }
              artifact={null}
              onSend={async (text) => {
                if (!(await command({ type: "instruction", text })))
                  throw new Error(
                    "送信できませんでした。入力を保持しています。",
                  );
              }}
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
              <Button onClick={() => setUtility("logs")}>実行ログ</Button>
              {state.exports.length > 0 && (
                <Button onClick={() => setUtility("delivery")}>
                  送信提案を確認
                </Button>
              )}
              {state.running && (
                <Button onClick={() => void command({ type: "cancel" })}>
                  実行を中止
                </Button>
              )}
            </div>
            {state.partial && (
              <pre className="native-stream" aria-label="実行中の出力">
                {state.partial}
              </pre>
            )}
            {(state.project.proposed || state.project.accepted) && (
              <article className="chat-contract">
                <Typography component="h3">検証する要件</Typography>
                <Chip
                  size="small"
                  label={
                    state.project.verified
                      ? "検証済み"
                      : state.project.accepted
                        ? "契約受理済み"
                        : "提案"
                  }
                />
                <pre>{state.project.review}</pre>
                <Button
                  disabled={running}
                  onClick={() => void command({ type: "refresh_status" })}
                >
                  現在の証拠を確認
                </Button>
                {state.project.proposed && !state.project.accepted && (
                  <Button
                    disabled={running}
                    onClick={async () => {
                      if (
                        await command({
                          type: "review_contract",
                          reference: state.project.reference,
                        })
                      )
                        setReviewing(true);
                    }}
                  >
                    提案された契約を確認
                  </Button>
                )}
              </article>
            )}
          </aside>
        </main>
      )}
      {state && (
        <Dialog
          open={utility === "history" || utility === "logs"}
          onClose={() => setUtility(null)}
          fullWidth
          maxWidth="lg"
        >
          <DialogTitle>
            {utility === "history"
              ? "作業の管理"
              : utility === "logs"
                ? "実行ログ"
                : utility === "delivery"
                  ? "送信提案"
                  : "解析の詳細"}
            <Button onClick={() => setUtility(null)}>閉じる</Button>
          </DialogTitle>
          <DialogContent>
            {utility === "history" && (
              <NativeWork
                workspace={state.portable}
                running={running}
                authorized={state.authorized}
                onCommand={command}
                onInspect={(file) => {
                  setUtility(null);
                  openFile(file);
                }}
              />
            )}
            {utility === "logs" && (
              <section aria-label="ツール実行ログ">
                {state.logs.length === 0 && (
                  <p>実行結果がここに表示されます。</p>
                )}
                {state.logs.map((entry, index) => (
                  <article key={index}>
                    {entry.type === "notice" ? (
                      <Alert severity="warning">{entry.text}</Alert>
                    ) : (
                      <>
                        <Typography component="h3">
                          {entry.tool} ·{" "}
                          {entry.type === "call"
                            ? "入力"
                            : entry.error
                              ? "失敗"
                              : "結果"}
                        </Typography>
                        <pre className="native-stream">{entry.text}</pre>
                      </>
                    )}
                  </article>
                ))}
                {state.reasoning && <pre>{state.reasoning}</pre>}
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
            会話とツール結果の送信先
          </Typography>
          <Typography>作業ディレクトリ: {state?.workspace}</Typography>
          <Typography sx={{ overflowWrap: "anywhere", mt: 2 }}>
            送信先: {state?.endpoint}
          </Typography>
          <Typography>モデル: {state?.model}</Typography>
          {state?.judge && (
            <Typography sx={{ overflowWrap: "anywhere" }}>
              別モデルjudge: {state.judge.model} · {state.judge.endpoint}
              （元要件と逆翻訳を送信）
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
              送信の許可を取り消す
            </Button>
          )}
          <Button
            disabled={running}
            onClick={() => void command({ type: "refresh_connections" })}
          >
            このPCの接続設定を読み直す
          </Button>
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
            <option value="">接続先を選択</option>
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
          <Button
            disabled={!provider || running}
            onClick={() => void command({ type: "list_models", provider })}
          >
            モデル一覧を更新
          </Button>
          {state?.models_provider === provider && state.models_error && (
            <Alert severity="info">
              モデル一覧を取得できませんでした。設定済みのモデルか、モデルIDを入力して選択できます。
            </Alert>
          )}
          {state?.connections.length === 0 && (
            <Alert severity="info">
              既存の接続設定が見つかりません。Hermes／Codex／Claudeでログインするか、Ollamaなどのサーバーを起動してからGUIを開き直してください。
            </Alert>
          )}
          <Button
            disabled={
              running ||
              !provider ||
              !model.trim() ||
              (provider === state?.provider &&
                model === state?.model &&
                (state.authorized || state.approval_available))
            }
            onClick={() =>
              void command({ type: "select_connection", provider, model })
            }
          >
            この接続を選ぶ
          </Button>
          {!approvalVisible && (
            <Alert severity="info">
              送信先とモデルの見出し全体が見えるように画面を広げてください。
            </Alert>
          )}
          <p>
            承認すると、このセッションの会話・ソース・ツール結果をこの接続先に送信できます。有効期間は最大8時間です。コマンドと検証はローカルの隔離環境で実行します。
          </p>
          <p>
            接続を変更すると以前の送信許可を終了し、新しく選んだ送信先とモデルの承認が必要になります。実行中の接続は変更できません。
          </p>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setApproval(false)}>閉じる</Button>
          <Button
            disabled={
              !state?.approval_available ||
              provider !== state.provider ||
              model !== state.model ||
              state.authorized ||
              busy ||
              !approvalVisible
            }
            onClick={async () => {
              if (
                state &&
                (await command({
                  type: "approve_processor",
                  connection_revision: state.connection_revision,
                  endpoint: state.endpoint,
                  model: state.model,
                  judge: state.judge,
                }))
              )
                setApproval(false);
            }}
          >
            この送信先を承認
          </Button>
        </DialogActions>
      </Dialog>
      <Dialog
        open={Boolean(review)}
        onClose={() => setReview(undefined)}
        fullWidth
        maxWidth="md"
      >
        <DialogTitle>この契約の意味を確認</DialogTitle>
        <DialogContent>
          {error && <Alert severity="error">{error}</Alert>}
          <Typography sx={{ overflowWrap: "anywhere" }}>
            契約: {review?.reference.contract} · revision{" "}
            {review?.reference.revision}
          </Typography>
          <pre className="native-contract">{review?.text}</pre>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setReview(undefined)}>閉じる</Button>
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
            拒否
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
            この契約を受理して続行
          </Button>
        </DialogActions>
      </Dialog>
    </div>
  );
}
