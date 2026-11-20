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
import { NativeExplorer } from "../features/native/NativeExplorer";
import { NativeWorkspaceGraph } from "../features/native/NativeWorkspaceGraph";

/** A single host workspace is attached; choosing another requires a new host launch. */
export function NativeApp({ token }: { token: string }) {
  const client = useMemo(() => new NativeClient(token), [token]);
  const [state, setState] = useState<NativeState>();
  const [error, setError] = useState("");
  const [connectionError, setConnectionError] = useState("");
  const [tab, setTab] = useState(1);
  const [codeOpened, setCodeOpened] = useState(false);
  const [utility, setUtility] = useState<
    "history" | "logs" | "delivery" | "analysis" | null
  >(null);
  const [focusedFile, setFocusedFile] = useState<string>();
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
      setOverview(undefined);
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
  const openDependencies = useCallback(
    (file: string) => {
      setUtility(null);
      setGraphPaths(file);
      setFocusedFile(file);
      setTab(1);
      if (file.endsWith(".lean")) {
        if (state?.atlas.available)
          void command({ type: "atlas_snapshot", paths: [file] });
      } else void command({ type: "source_snapshot", paths: [file] });
    },
    [command, state?.atlas.available],
  );
  const conversation: Conversation | undefined = state && {
    id: "native",
    title: state.workspace.split("/").at(-1) || "Workspace",
    messages: visibleHistory(state),
  };
  const running = Boolean(state?.running || busy);
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
  return (
    <div className="native-shell">
      <header className="native-header">
        <Typography component="h1" variant="h3">
          Meden
        </Typography>
        <Chip size="small" label="ローカルワークスペース" />
        <Typography className="native-workspace" title={state?.workspace}>
          {state?.workspace}
        </Typography>
        <Button aria-label="接続先" onClick={() => setApproval(true)}>
          {state?.model ? `接続先 · ${state.model}` : "接続先"}
        </Button>
        {state?.authorized && (
          <Button
            color="warning"
            onClick={() => void command({ type: "revoke_processor" })}
          >
            送信の許可を取り消す
          </Button>
        )}
      </header>
      {error && (
        <Alert
          severity="error"
          action={
            <Button
              color="inherit"
              onClick={() => setReconnect((value) => value + 1)}
            >
              再接続
            </Button>
          }
        >
          {error}
        </Alert>
      )}
      {connectionError && (
        <Alert
          severity="error"
          action={
            <Button
              color="inherit"
              onClick={() => setReconnect((value) => value + 1)}
            >
              再接続
            </Button>
          }
        >
          {connectionError}
        </Alert>
      )}
      {!state ? (
        <div className="loading">
          <CircularProgress aria-label="セッションに接続中" />
        </div>
      ) : (
        <>
          {state.portable.error && (
            <Alert severity="warning">
              保存済みの会話・作業履歴を読み込めませんでした。元の記録は保持しています。
              {state.portable.error}
            </Alert>
          )}
          {!state.authorized && (
            <Alert
              severity="info"
              action={
                <Button onClick={() => setApproval(true)}>接続先を確認</Button>
              }
            >
              {!state.provider
                ? "providerとモデルを選ぶと、このworkspaceの作業を続けられます。履歴とグラフは接続せずに閲覧できます。"
                : state.approval_available
                  ? "実行前に、会話とツール結果の送信先を確認・承認してください。再起動後は新しい承認が必要です。"
                  : "送信の許可は終了しています。続けるにはmeden guiを再起動し、新しいURLで送信先を確認してください。"}
            </Alert>
          )}
          <main className="native-main">
            <div className="native-chat">
              {state.stopped &&
                !(tab === 1 && (focusedFile || utility === "analysis")) && (
                  <Alert
                    severity={
                      state.activity === "Complete" ? "info" : "warning"
                    }
                    sx={{ whiteSpace: "pre-wrap" }}
                  >
                    {state.stopped}
                    <br />
                    ファイルと会話は保持されています。対象の操作をやり直すか、新しい指示を送信してください。
                  </Alert>
                )}

              {conversation!.messages.some(
                (message) => message.origin === "recorded",
              ) && (
                <Typography variant="caption" className="native-history-note">
                  .medenから保存された会話を表示しています。過去のチェックや承認は、この起動の検証状態には引き継ぎません。
                </Typography>
              )}
              <ChatView
                projectName={conversation!.title}
                conversation={conversation}
                draftKey={`meden-native-input:${state.workspace}`}
                running={running}
                activityText={`${state.activity} · ${elapsed}秒`}
                disabled={!state.authorized || Boolean(connectionError)}
                onSend={async (text) => {
                  if (!(await command({ type: "instruction", text })))
                    throw new Error(
                      "送信できませんでした。入力を保持しています。",
                    );
                }}
                artifact={
                  <Button
                    onClick={() => {
                      setCodeOpened(true);
                      setTab(0);
                    }}
                  >
                    コードを開く
                  </Button>
                }
              />
              <div className="chat-utilities">
                <Button onClick={() => setUtility("logs")}>
                  実行ログ{state.logs.length ? ` · ${state.logs.length}` : ""}
                </Button>
                {state.exports.length > 0 && (
                  <Button
                    onClick={() => {
                      setTab(1);
                      setUtility("delivery");
                    }}
                  >
                    送信提案を確認
                  </Button>
                )}
              </div>
              {state.running && (
                <Button onClick={() => void command({ type: "cancel" })}>
                  実行を中止
                </Button>
              )}
              {state.reasoning && (
                <details className="native-stream">
                  <summary>モデルが公開したreasoning（未確定）</summary>
                  <pre>{state.reasoning}</pre>
                </details>
              )}
              {state.partial && (
                <pre className="native-stream" aria-label="実行中の出力">
                  {state.partial}
                </pre>
              )}
            </div>
            <aside className="native-artifacts" aria-label="実装と実行結果">
              <Tabs
                value={tab}
                onChange={(_, value: number) => {
                  setTab(value);
                  setUtility(null);
                  if (value === 0) setCodeOpened(true);
                }}
                aria-label="作業パネル"
                variant="fullWidth"
              >
                <Tab label="コード" />
                <Tab label="証明" />
                <Tab label="プレビュー" />
              </Tabs>
              <div className="native-panel">
                <div hidden={tab !== 0}>
                  {codeOpened && (
                    <NativeExplorer
                      state={state}
                      running={running}
                      onCommand={command}
                      onDependencies={openDependencies}
                    />
                  )}
                </div>
                <div hidden={tab !== 2}>
                  <NativePreview
                    snapshot={state.preview}
                    running={running}
                    onCapture={(root, entry) =>
                      command({ type: "capture_preview", root, entry })
                    }
                  />
                </div>
                <div className="native-proof-panel" hidden={tab !== 1}>
                  <div className="proof-toolbar">
                    <Button onClick={() => setUtility("history")}>
                      作業・履歴
                    </Button>
                    <Button onClick={() => setUtility("analysis")}>
                      解析の詳細
                    </Button>
                  </div>
                  {overviewError && (
                    <Alert severity="error">{overviewError}</Alert>
                  )}
                  <div
                    className="project-map-page"
                    hidden={Boolean(focusedFile) || Boolean(utility)}
                  >
                    <NativeWorkspaceGraph
                      snapshot={overview}
                      state={{ ...state, running }}
                      onDependencies={openDependencies}
                      onRefresh={() => {
                        setOverviewError("");
                        void command({ type: "refresh_workspace" });
                      }}
                    />
                  </div>
                  {focusedFile && !utility && (
                    <>
                      <div className="dependency-breadcrumb">
                        <Button onClick={() => setFocusedFile(undefined)}>
                          ← プロジェクトマップ
                        </Button>
                        <code>{focusedFile}</code>
                      </div>
                      <NativeDependencyGraph
                        compact
                        state={state}
                        overview={overview}
                        paths={focusedFile}
                        onPaths={setGraphPaths}
                        onCommand={command}
                        running={running}
                        elapsed={elapsed}
                        connectionError={connectionError}
                      />
                      {state.portable.document?.tasks
                        .filter((task) =>
                          focusedFile.endsWith(
                            task.module.replaceAll(".", "/") + ".lean",
                          ),
                        )
                        .map((task) => (
                          <article className="file-requirement" key={task.id}>
                            <Typography component="h3">{task.title}</Typography>
                            <p>{task.requirement}</p>
                            <Typography variant="caption">
                              {task.theorems.join(" · ")} · 保存された証明対象
                            </Typography>
                          </article>
                        ))}
                    </>
                  )}
                  {utility === "history" && (
                    <>
                      <Button onClick={() => setUtility(null)}>
                        ← プロジェクトマップ
                      </Button>
                      <NativeWork
                        workspace={state.portable}
                        running={running}
                        authorized={state.authorized}
                        onCommand={command}
                        onInspect={(file) => {
                          setUtility(null);
                          setCodeOpened(true);
                          setTab(0);
                          void command({
                            type: "inspect",
                            path: file,
                            directory: false,
                          });
                        }}
                      />
                    </>
                  )}
                  {utility === "delivery" && (
                    <>
                      <Button onClick={() => setUtility(null)}>
                        ← プロジェクトマップ
                      </Button>
                      <NativeDelivery
                        exports={state.exports}
                        review={state.export_review}
                        outcome={state.export_outcome}
                        running={running}
                        onCommand={command}
                      />
                    </>
                  )}
                  {utility === "analysis" && (
                    <>
                      <Button onClick={() => setUtility(null)}>
                        ← プロジェクトマップ
                      </Button>
                      <NativeDependencyGraph
                        state={state}
                        overview={overview}
                        paths={graphPaths ?? state.project.source_path}
                        onPaths={setGraphPaths}
                        onCommand={command}
                        running={running}
                        elapsed={elapsed}
                        connectionError={connectionError}
                      />
                    </>
                  )}
                  {(state.project.proposed || state.project.accepted) && (
                    <section className="native-contract-summary">
                      <Typography variant="h3">検証する要件</Typography>
                      <Chip
                        label={
                          state.project.verified
                            ? "検証済み（受理した契約の範囲）"
                            : state.project.accepted
                              ? "契約受理済み・現在の証拠なし"
                              : "契約未受理"
                        }
                        color={state.project.verified ? "success" : "default"}
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
                    </section>
                  )}
                </div>
              </div>
            </aside>
          </main>
        </>
      )}
      {state && (
        <Dialog
          open={utility === "logs"}
          onClose={() => setUtility(null)}
          fullWidth
          maxWidth="lg"
        >
          <DialogTitle>
            実行ログ
            <Button onClick={() => setUtility(null)}>閉じる</Button>
          </DialogTitle>
          <DialogContent>
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
