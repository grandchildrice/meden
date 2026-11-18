import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
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
} from "@meden/agent-protocol/native";
import { ChatView } from "../features/chat/ChatView";
import type { Conversation } from "@meden/workspace-schema";
import { NativeMeaning } from "../features/native/NativeMeaning";
import { NativePreview } from "../features/native/NativePreview";
import { NativeDelivery } from "../features/native/NativeDelivery";
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
  const [tab, setTab] = useState(0);
  const [path, setPath] = useState(".");
  const [graphPaths, setGraphPaths] = useState<string>();
  const [review, setReview] = useState<{
    reference: NativeReference;
    text: string;
  }>();
  const [reviewing, setReviewing] = useState(false);
  const [approval, setApproval] = useState(false);
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
    messages: state.messages.map((message, index) => ({
      id: String(index),
      role: message.speaker,
      text: message.text,
      status: message.speaker === "user" ? "sent" : "received",
      createdAt: new Date(0).toISOString(),
    })),
  };
  const running = Boolean(state?.running || busy);
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
        <Button onClick={() => setApproval(true)}>接続先</Button>
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
          {!state.authorized && (
            <Alert
              severity="info"
              action={
                <Button onClick={() => setApproval(true)}>接続先を確認</Button>
              }
            >
              {state.approval_available
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
              </Tabs>
              <div className="native-panel">
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
                    {state.logs.map((entry, index) => (
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
                    ))}
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
                  <TextField
                    label="グラフの対象ファイル（1行に1つ）"
                    multiline
                    minRows={2}
                    fullWidth
                    size="small"
                    value={graphPaths ?? state.project.source_path}
                    onChange={(event) => setGraphPaths(event.target.value)}
                    placeholder={"Spec.lean\nsrc/kernel.rs"}
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
                        key={state.source_snapshot.revision}
                        snapshot={state.source_snapshot}
                        analysis={state.analysis}
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
      <Dialog
        open={approval}
        onClose={() => setApproval(false)}
        fullWidth
        maxWidth="sm"
      >
        <DialogTitle>会話とツール結果の送信先</DialogTitle>
        <DialogContent>
          {error && <Alert severity="error">{error}</Alert>}
          <Typography>作業ディレクトリ: {state?.workspace}</Typography>
          <Typography sx={{ overflowWrap: "anywhere", mt: 2 }}>
            送信先: {state?.endpoint}
          </Typography>
          <Typography>モデル: {state?.model}</Typography>
          <p>
            承認すると、このセッションの会話・ソース・ツール結果をこの接続先に送信できます。有効期間は最大8時間です。コマンドと検証はローカルの隔離環境で実行します。
          </p>
          <p>
            接続先を変更するにはmeden
            guiを終了し、新しい接続設定で起動してください。取り消し・期限切れ後はこの承認を更新できません。
          </p>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setApproval(false)}>閉じる</Button>
          <Button
            disabled={!state?.approval_available || state.authorized || busy}
            onClick={async () => {
              if (
                state &&
                (await command({
                  type: "approve_processor",
                  endpoint: state.endpoint,
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
