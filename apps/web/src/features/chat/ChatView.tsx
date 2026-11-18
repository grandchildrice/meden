import { useEffect, useMemo, useRef } from "react";
import {
  AssistantRuntimeProvider,
  ComposerPrimitive,
  MessagePrimitive,
  ThreadPrimitive,
  useExternalStoreRuntime,
  type AppendMessage,
  type ThreadMessageLike,
} from "@assistant-ui/react";
import { MarkdownTextPrimitive } from "@assistant-ui/react-markdown";
import { Button, IconButton, Typography } from "@mui/material";
import ArrowUpwardRounded from "@mui/icons-material/ArrowUpwardRounded";
import InsertDriveFileOutlined from "@mui/icons-material/InsertDriveFileOutlined";
import ArrowForwardRounded from "@mui/icons-material/ArrowForwardRounded";
import { RecordedActivity } from "./RecordedActivity";
import type { Conversation } from "@meden/workspace-schema";

interface Props {
  projectName: string;
  conversation?: Conversation;
  draftKey: string;
  running: boolean;
  onSend: (text: string) => Promise<void>;
  onOpenArtifact: () => void;
}
export function ChatView({
  projectName,
  conversation,
  draftKey,
  running,
  onSend,
  onOpenArtifact,
}: Props) {
  const input = useRef<HTMLTextAreaElement>(null);
  const messages = useMemo<ThreadMessageLike[]>(
    () =>
      (conversation?.messages || []).map((message) => ({
        id: message.id,
        role: message.role,
        metadata: { custom: { activities: message.activities } },
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
    if (!text || running) return;
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
  const empty = !conversation?.messages.length;
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
              <ThreadPrimitive.Messages
                components={{ UserMessage, AssistantMessage }}
              />
              {conversation?.messages.some(
                (message) => message.role === "assistant",
              ) && (
                <button className="artifact-card" onClick={onOpenArtifact}>
                  <span className="artifact-card-icon">
                    <InsertDriveFileOutlined />
                  </span>
                  <span>
                    <strong>{projectName}</strong>
                    <small>コード・プレビュー・証明</small>
                  </span>
                  <ArrowForwardRounded fontSize="small" />
                </button>
              )}
              {running && (
                <div role="status" className="reply-activity">
                  <span className="activity-dot" />
                  応答を準備しています
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
              submitMode="enter"
              onChange={(event) =>
                localStorage.setItem(draftKey, event.target.value)
              }
            />
            <div className="composer-actions">
              <span className="composer-project">{projectName}</span>
              <ComposerPrimitive.Send asChild>
                <IconButton className="send-message" aria-label="送信">
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
          <div className="composer-caption">
            {empty
              ? "メッセージから会話を始めます"
              : "Enterで送信・Shift + Enterで改行"}
          </div>
        </ThreadPrimitive.Root>
      </section>
    </AssistantRuntimeProvider>
  );
}
function UserMessage() {
  return (
    <MessagePrimitive.Root className="message user-turn">
      <div className="user-message">
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
        <MessagePrimitive.Parts
          components={{ Text: () => <MarkdownTextPrimitive /> }}
        />
        <RecordedActivity />
      </div>
    </MessagePrimitive.Root>
  );
}
