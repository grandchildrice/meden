import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
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
import { NativeMeaning } from "../features/native/NativeMeaning";
import { NativePreview } from "../features/native/NativePreview";
import { NativeRepair } from "../features/native/NativeRepair";
import { NativeDelivery } from "../features/native/NativeDelivery";
import { NativeWork } from "../features/native/NativeWork";
import { visibleHistory } from "../features/native/history";
import { NativeWorkspaceGraph } from "../features/native/NativeWorkspaceGraph";
const NativeGraph = lazy(() =>
  import("../features/native/NativeGraph").then((module) => ({
    default: module.NativeGraph,
  })),
);

/** A single host workspace is attached; choosing another requires a new host launch. */
export function NativeApp({ token }: { token: string }) {
  const client = useMemo(() => new NativeClient(token), [token]);
  const [state, setState] = useState<NativeState>();
  const [error, setError] = useState("");
  const [connectionError, setConnectionError] = useState("");
  const [tab, setTab] = useState(3);
  const [overview, setOverview] = useState<WorkspaceOverview>();
  const [overviewError, setOverviewError] = useState("");
  const [provider, setProvider] = useState("");
  const [model, setModel] = useState("");
  const listedProvider = useRef("");
  const [path, setPath] = useState(".");
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
      setGraphPaths(Object.keys(state.source_snapshot.files).join("\n"));
    }
  }, [state?.source_snapshot?.revision, graphPaths]);
  const command = async (command: NativeCommand) => {
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
  };
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
          {state.stopped && (
            <Alert
              severity={state.activity === "Complete" ? "info" : "warning"}
            >
              {state.stopped}
              <br />
              ファイルと会話は保持されています。対象の操作をやり直すか、新しい指示を送信してください。
            </Alert>
          )}
          <main className="native-main">
            <div className="native-chat">
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
                  <Button onClick={() => setTab(0)}>
                    ローカルファイル・実行ログ・契約を開く
                  </Button>
                }
              />
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
                onChange={(_, value: number) => setTab(value)}
                aria-label="作業パネル"
                variant="scrollable"
                scrollButtons="auto"
                allowScrollButtonsMobile
              >
                <Tab label="コード" />
                <Tab label="実行ログ" />
                <Tab label="契約・証明" />
                <Tab label="グラフ" />
                <Tab label="プレビュー" />
                <Tab
                  label={`配送${state.exports.some((item) => item.state === "pending" || item.state === "reviewed") ? " •" : ""}`}
                />
                <Tab label="作業・履歴" />
              </Tabs>
              <div className="native-panel">
                <div hidden={tab !== 6}>
                  <NativeWork
                    workspace={state.portable}
                    running={running}
                    authorized={state.authorized}
                    onCommand={command}
                    onInspect={(file) => {
                      setPath(file);
                      setTab(0);
                      void command({
                        type: "inspect",
                        path: file,
                        directory: false,
                      });
                    }}
                  />
                </div>
                <div hidden={tab !== 5}>
                  <NativeDelivery
                    exports={state.exports}
                    review={state.export_review}
                    outcome={state.export_outcome}
                    running={running}
                    onCommand={command}
                  />
                </div>
                <div hidden={tab !== 4}>
                  <NativePreview
                    snapshot={state.preview}
                    running={running}
                    onCapture={(root, entry) =>
                      command({ type: "capture_preview", root, entry })
                    }
                  />
                </div>
                {tab === 0 && (
                  <>
                    <Typography variant="h3">ローカルファイル</Typography>
                    <TextField
                      fullWidth
                      size="small"
                      label="ワークスペース内のパス"
                      value={path}
                      onChange={(event) => setPath(event.target.value)}
                      sx={{ my: 2 }}
                    />
                    <Button
                      disabled={running}
                      onClick={() =>
                        void command({ type: "inspect", path, directory: true })
                      }
                    >
                      フォルダを開く
                    </Button>
                    <Button
                      disabled={running}
                      onClick={() =>
                        void command({
                          type: "inspect",
                          path,
                          directory: false,
                        })
                      }
                    >
                      ファイルを読む
                    </Button>
                    <Button
                      disabled={running || !/\.(rs|lean)$/.test(path)}
                      onClick={() => {
                        setGraphPaths(path);
                        setTab(3);
                        void command({
                          type: "source_snapshot",
                          paths: [path],
                        });
                      }}
                    >
                      このファイルのグラフ
                    </Button>
                    {state.inspection && (
                      <>
                        <Typography component="h3">
                          {state.inspection.path}
                        </Typography>
                        {!state.inspection.directory &&
                          !state.inspection.error && (
                            <>
                              <p>{state.inspection.offset || 1}行目から表示</p>
                              <Button
                                disabled={
                                  running || (state.inspection.offset || 1) <= 1
                                }
                                onClick={() =>
                                  void command({
                                    type: "inspect",
                                    path: state.inspection!.path,
                                    directory: false,
                                    offset: Math.max(
                                      1,
                                      (state.inspection!.offset || 1) - 200,
                                    ),
                                  })
                                }
                              >
                                前のページ
                              </Button>
                              <Button
                                disabled={
                                  running || !state.inspection.next_offset
                                }
                                onClick={() =>
                                  void command({
                                    type: "inspect",
                                    path: state.inspection!.path,
                                    directory: false,
                                    offset: state.inspection!.next_offset!,
                                  })
                                }
                              >
                                次のページ
                              </Button>
                            </>
                          )}
                        <pre aria-label="ファイルの内容">
                          {state.inspection.error ||
                            formatInspection(state.inspection.text || "")}
                        </pre>
                      </>
                    )}
                  </>
                )}
                {tab === 1 && (
                  <>
                    <Typography variant="h3">
                      今回の起動中のツール実行
                    </Typography>
                    {state.logs.length === 0 && (
                      <p>実行結果がここに表示されます。</p>
                    )}
                    {state.logs.map((entry, index) =>
                      entry.type === "notice" ? (
                        <Alert key={index} severity="warning">
                          {entry.text}
                        </Alert>
                      ) : (
                        <details key={index} open={Boolean(entry.error)}>
                          <summary>
                            {entry.tool} ·{" "}
                            {entry.type === "call"
                              ? "入力"
                              : entry.error
                                ? "失敗"
                                : "結果"}
                          </summary>
                          <pre>{entry.text}</pre>
                        </details>
                      ),
                    )}
                  </>
                )}
                {tab === 2 && (
                  <>
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
                  </>
                )}
                <div hidden={tab !== 3}>
                  {overviewError && (
                    <Alert severity="error">{overviewError}</Alert>
                  )}
                  <NativeWorkspaceGraph
                    snapshot={overview}
                    state={state}
                    onRead={(file) => {
                      setPath(file);
                      setTab(0);
                      void command({
                        type: "inspect",
                        path: file,
                        directory: false,
                      });
                    }}
                    onRefresh={() => {
                      setOverviewError("");
                      void command({ type: "refresh_workspace" });
                    }}
                  />
                  <Typography variant="h3" sx={{ mt: 3 }}>
                    宣言の依存を詳しく見る
                  </Typography>
                  {state.running && (
                    <Typography>
                      {state.activity} · {elapsed}秒
                    </Typography>
                  )}
                  <Typography variant="body2">
                    Lean Atlas:{" "}
                    {state.atlas.prepared
                      ? "オフライン抽出の準備済み"
                      : state.atlas.available
                        ? "Leanを自動検出しました。初回の準備は依存抽出時に自動で行います"
                        : "Lean実行環境が見つかりません。下の案内を確認してください。"}
                  </Typography>
                  {!state.atlas.available && (
                    <>
                      <Alert
                        severity="info"
                        sx={{ whiteSpace: "pre-wrap", mt: 1 }}
                      >
                        {state.atlas.detail}
                      </Alert>
                      <Button
                        disabled={running}
                        onClick={() => void command({ type: "refresh_status" })}
                      >
                        再検出
                      </Button>
                    </>
                  )}
                  {!state.atlas.prepared && (
                    <Button
                      disabled={running || !state.atlas.available}
                      onClick={() => void command({ type: "prepare_atlas" })}
                    >
                      依存抽出の準備
                    </Button>
                  )}
                  <TextField
                    label="グラフの対象ファイル（1行に1つ）"
                    sx={{ mt: 2 }}
                    multiline
                    minRows={2}
                    fullWidth
                    size="small"
                    value={graphPaths ?? state.project.source_path}
                    onChange={(event) => setGraphPaths(event.target.value)}
                    placeholder={
                      "空欄ならLakeのターゲットを自動選択\nSpec.lean"
                    }
                  />
                  <Button
                    disabled={
                      running ||
                      !(graphPaths ?? state.project.source_path).trim()
                    }
                    onClick={() =>
                      void command({
                        type: "source_snapshot",
                        paths: (graphPaths ?? state.project.source_path)
                          .split("\n")
                          .map((path) => path.trim())
                          .filter(Boolean),
                      })
                    }
                  >
                    ソースを読み直してグラフを表示
                  </Button>
                  <Button
                    disabled={
                      running ||
                      !state.atlas.available ||
                      (graphPaths ?? state.project.source_path)
                        .split("\n")
                        .filter((path) => path.trim())
                        .some((path) => !path.trim().endsWith(".lean"))
                    }
                    onClick={() =>
                      void command({
                        type: "atlas_snapshot",
                        paths: (graphPaths ?? state.project.source_path)
                          .split("\n")
                          .map((path) => path.trim())
                          .filter(Boolean),
                      })
                    }
                  >
                    Leanでコンパイルして依存を抽出
                  </Button>
                  {state.source_snapshot ? (
                    <Suspense
                      fallback={
                        <CircularProgress
                          aria-label="ソースグラフを読み込み中"
                          size={24}
                        />
                      }
                    >
                      <NativeGraph
                        key={
                          state.source_snapshot.atlas?.revision ||
                          state.source_snapshot.revision
                        }
                        snapshot={state.source_snapshot}
                        analysis={state.analysis}
                        history={state.analysis_history}
                        onRepair={(finding_id) =>
                          command({ type: "prepare_repair", finding_id })
                        }
                        onSelectAnalysis={(id) =>
                          command({ type: "select_analysis", id })
                        }
                        authorized={state.authorized && !connectionError}
                        running={running}
                        onAnalyze={(input) =>
                          command({ type: "analyze_meaning", input })
                        }
                      />
                    </Suspense>
                  ) : (
                    <p>
                      Rust／Leanファイルを選んで読み込んでください。作業の実行後は読み直しが必要です。
                    </p>
                  )}
                  {!state.source_snapshot && state.analysis && (
                    <NativeMeaning
                      record={state.analysis}
                      onRepair={(finding_id) =>
                        command({ type: "prepare_repair", finding_id })
                      }
                      history={state.analysis_history}
                      onSelect={(id) =>
                        command({ type: "select_analysis", id })
                      }
                      authorized={state.authorized}
                      running={running}
                      onRun={(input) =>
                        command({ type: "analyze_meaning", input })
                      }
                    />
                  )}
                </div>
              </div>
            </aside>
          </main>
        </>
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
function formatInspection(text: string): string {
  try {
    const value: unknown = JSON.parse(text);
    return typeof value === "string" ? value : JSON.stringify(value, null, 2);
  } catch {
    return text;
  }
}
