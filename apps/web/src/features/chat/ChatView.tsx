import { useEffect, useMemo, useRef, type ReactNode } from "react";
import {
  AssistantRuntimeProvider,
  ComposerPrimitive,
  MessagePrimitive,
  ThreadPrimitive,
  useExternalStoreRuntime,
  useAuiState,
  type AppendMessage,
  type ThreadMessageLike,
} from "@assistant-ui/react";
import { MarkdownTextPrimitive } from "@assistant-ui/react-markdown";
import { Button, IconButton, Typography } from "@mui/material";
import ArrowUpwardRounded from "@mui/icons-material/ArrowUpwardRounded";
import { RecordedActivity } from "./RecordedActivity";
import type { Conversation } from "@meden/workspace-schema";

const technologyLogos = [
  ["Rust", "rust"],
  ["Bend", "bend"],
  ["Lean", "lean"],
  ["Rocq", "rocq"],
  ["Dafny", "dafny"],
  ["Isabelle", "isabelle"],
  ["TLA+", "tla"],
  ["cvc5", "cvc5"],
  ["Z3", "z3"],
  ["Certora", "certora"],
] as const;

interface Props {
  projectName: string;
  conversation?: Conversation;
  draftKey: string;
  running: boolean;
  activityText?: string;
  disabled?: boolean;
  artifact: ReactNode;
  history?: ReactNode;
  composerActions?: ReactNode;
  sessionLabels?: Record<string, string>;
  onSend: (text: string) => Promise<void>;
}
export function ChatView({
  projectName,
  conversation,
  draftKey,
  running,
  activityText,
  disabled,
  artifact,
  history,
  composerActions,
  sessionLabels,
  onSend,
}: Props) {
  const input = useRef<HTMLTextAreaElement>(null);
  const messages = useMemo<ThreadMessageLike[]>(
    () =>
      (conversation?.messages || []).map((message) => ({
        id: message.id,
        role: message.role,
        metadata: {
          custom: {
            activities: message.activities,
            origin: message.origin,
            sessionLabel: sessionLabels?.[message.id],
            groupedSession: Boolean(sessionLabels),
          },
        },
        content: [{ type: "text", text: message.text }],
      })),
    [conversation],
  );
  const onNew = async (message: AppendMessage) => {
    const text = message.content
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n")
      .trim();
    if (!text || running || disabled) return;
    await onSend(text);
    localStorage.removeItem(draftKey);
  };
  const runtime = useExternalStoreRuntime<ThreadMessageLike>({
    messages,
    onNew,
    convertMessage: (message) => message,
    isRunning: running,
  });
  useEffect(() => {
    runtime.thread.composer.setText(localStorage.getItem(draftKey) || "");
  }, [draftKey, runtime]);
  const empty = !conversation?.messages.length && !history;
  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <section
        className={"chat-panel " + (empty ? "chat-home" : "chat-started")}
        aria-label="会話"
      >
        <ThreadPrimitive.Root className="thread">
          {empty ? (
            <div className="chat-welcome">
              <div className="welcome-mark" aria-hidden="true">
                <svg width="32" height="32" viewBox="0 0 24 24">
                  <path
                    d="M3 19 9 5l6 14 6-14"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinejoin="round"
                  />
                </svg>
              </div>
              <Typography variant="h1">何を安全に作りますか？</Typography>
            </div>
          ) : (
            <ThreadPrimitive.Viewport className="transcript">
              {history}
              <ThreadPrimitive.Messages
                components={{ UserMessage, AssistantMessage }}
              />
              {conversation?.messages.some(
                (message) => message.role === "assistant",
              ) && artifact}
              {running && (
                <div role="status" className="reply-activity">
                  <span className="activity-dot" />
                  {activityText || "応答を準備しています"}
                </div>
              )}
            </ThreadPrimitive.Viewport>
          )}
          <ComposerPrimitive.Root className="composer">
            <ComposerPrimitive.Input
              ref={input}
              className="composer-input"
              placeholder="作りたいものや、確認したいことを入力"
              aria-label="メッセージ"
              disabled={disabled}
              submitMode="enter"
              onChange={(event) =>
                localStorage.setItem(draftKey, event.target.value)
              }
            />
            <div className="composer-actions">
              {composerActions || (
                <span className="composer-project">{projectName}</span>
              )}
              <ComposerPrimitive.Send asChild>
                <IconButton
                  className="send-message"
                  aria-label="送信"
                  disabled={disabled || running}
                >
                  <ArrowUpwardRounded fontSize="small" />
                </IconButton>
              </ComposerPrimitive.Send>
            </div>
          </ComposerPrimitive.Root>
          {empty && (
            <div className="starter-prompts">
              <Button
                variant="outlined"
                onClick={() => {
                  runtime.thread.composer.setText(
                    "自販機シミュレーターを作成して",
                  );
                  input.current?.focus();
                }}
              >
                自販機シミュレーターを作る
              </Button>
              <Button
                variant="outlined"
                onClick={() => {
                  runtime.thread.composer.setText(
                    "自販機の証明と依存関係を確認して",
                  );
                  input.current?.focus();
                }}
              >
                証明と依存関係を確認する
              </Button>
            </div>
          )}
          {empty ? (
            <ul
              className="technology-logos"
              role="list"
              aria-label="言語・形式検証ツールの例"
            >
              {technologyLogos.map(([name, file]) => (
                <li key={file}>
                  <img
                    alt={name}
                    src={
                      new URL(
                        `../../assets/technology-logos/${file}.png`,
                        import.meta.url,
                      ).href
                    }
                    width={56}
                    height={34}
                  />
                </li>
              ))}
            </ul>
          ) : (
            <div className="composer-caption">
              Enterで送信・Shift + Enterで改行
            </div>
          )}
        </ThreadPrimitive.Root>
      </section>
    </AssistantRuntimeProvider>
  );
}
function UserMessage() {
  return (
    <MessagePrimitive.Root className="message user-turn">
      <div className="user-message">
        <RecordedOrigin />
        <MessagePrimitive.Parts />
      </div>
      <RecordedActivity />
    </MessagePrimitive.Root>
  );
}
function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="message assistant-message">
      <div className="assistant-avatar" aria-label="Meden">
        M
      </div>
      <div className="assistant-content">
        <RecordedOrigin />
        <MessagePrimitive.Parts
          components={{ Text: () => <MarkdownTextPrimitive /> }}
        />
        <RecordedActivity />
      </div>
    </MessagePrimitive.Root>
  );
}
function RecordedOrigin() {
  const sessionLabel = useAuiState(
    (state) => state.message.metadata.custom.sessionLabel,
  );
  const origin = useAuiState((state) => state.message.metadata.custom.origin);
  const groupedSession = useAuiState(
    (state) => state.message.metadata.custom.groupedSession,
  );
  if (typeof sessionLabel === "string")
    return (
      <Typography
        className="chat-session-divider"
        variant="caption"
        component="p"
      >
        {sessionLabel}
      </Typography>
    );
  return origin === "recorded" && !groupedSession ? (
    <Typography variant="caption" component="p">
      保存された会話
    </Typography>
  ) : null;
}
