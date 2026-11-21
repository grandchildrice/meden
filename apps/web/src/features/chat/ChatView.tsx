import { memo, useEffect, useMemo, useRef, type ReactNode } from "react";
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
import { CollapsibleMessage } from "./CollapsibleMessage";

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
  compact?: boolean;
  conversation?: Conversation;
  draftKey: string;
  running: boolean;
  activityText?: string;
  disabled?: boolean;
  pending?: boolean;
  artifact: ReactNode;
  notice?: ReactNode;
  composerActions?: ReactNode;
  sessionLabels?: Record<string, string>;
  onSend: (text: string) => Promise<void>;
}
const convertMessage = (message: ThreadMessageLike) => message;
export const ChatView = memo(function ChatView({
  projectName,
  compact,
  conversation,
  draftKey,
  running,
  activityText,
  disabled,
  pending,
  artifact,
  notice,
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
    [conversation, sessionLabels],
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
    convertMessage,
    isRunning: Boolean(running || pending),
  });
  useEffect(() => {
    runtime.thread.composer.setText(localStorage.getItem(draftKey) || "");
  }, [draftKey, runtime]);
  const empty = !compact && !conversation?.messages.length && !notice;
  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <section
        className={"chat-panel " + (empty ? "chat-home" : "chat-started")}
        aria-label="Conversation"
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
              <Typography variant="h1">
                What would you like to build safely?
              </Typography>
            </div>
          ) : (
            <ThreadPrimitive.Viewport className="transcript">
              <ThreadPrimitive.Messages
                components={{ UserMessage, AssistantMessage }}
              />
              {conversation?.messages.some(
                (message) => message.role === "assistant",
              ) && artifact}
              {notice}
              {running && (
                <div role="status" className="reply-activity">
                  <span className="activity-dot" />
                  {activityText || "Preparing response"}
                </div>
              )}
            </ThreadPrimitive.Viewport>
          )}
          <ComposerPrimitive.Root className="composer">
            <ComposerPrimitive.Input
              ref={input}
              className="composer-input"
              placeholder="Describe what to build or check"
              aria-label="Message"
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
                  aria-label="Send"
                  disabled={disabled || running || pending}
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
                    "Build a vending machine simulator",
                  );
                  input.current?.focus();
                }}
              >
                Build a vending machine simulator
              </Button>
              <Button
                variant="outlined"
                onClick={() => {
                  runtime.thread.composer.setText(
                    "Check the vending machine proofs and dependencies",
                  );
                  input.current?.focus();
                }}
              >
                Check proofs and dependencies
              </Button>
            </div>
          )}
          {empty ? (
            <ul
              className="technology-logos"
              role="list"
              aria-label="Languages and verification tools"
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
              Enter to send · Shift + Enter for a new line
            </div>
          )}
        </ThreadPrimitive.Root>
      </section>
    </AssistantRuntimeProvider>
  );
});
function UserMessage() {
  return (
    <MessagePrimitive.Root className="message user-turn">
      <div className="user-message">
        <RecordedOrigin />
        <CollapsibleMessage>
          <MessagePrimitive.Parts />
        </CollapsibleMessage>
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
        <CollapsibleMessage>
          <MessagePrimitive.Parts
            components={{ Text: () => <MarkdownTextPrimitive /> }}
          />
        </CollapsibleMessage>
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
      Saved conversation
    </Typography>
  ) : null;
}
