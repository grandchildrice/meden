import { useEffect, useMemo, useState } from "react";
import {
  AssistantRuntimeProvider,
  ComposerPrimitive,
  MessagePrimitive,
  ThreadPrimitive,
  useAuiState,
  useExternalStoreRuntime,
  type AppendMessage,
  type ThreadMessageLike,
} from "@assistant-ui/react";
import { Button, Divider, Typography } from "@mui/material";
import ArrowForwardRounded from "@mui/icons-material/ArrowForwardRounded";
import SendOutlined from "@mui/icons-material/SendOutlined";
import {
  type Workspace,
  analysisIsCurrent,
  analysisFor,
} from "@meden/workspace-schema";
import { ProofStatusChip, AnalysisStatusChip } from "../../components/Status";

interface Props {
  workspace: Workspace;
  conversationId?: string;
  onChange: (workspace: Workspace) => Promise<boolean>;
  onSelectConversation: (id: string) => void;
  onClaim: (id: string) => void;
  onGraph: (id: string) => void;
}
export function ChatView({
  workspace,
  conversationId,
  onChange,
  onSelectConversation,
  onClaim,
  onGraph,
}: Props) {
  const [id] = useState(
    () =>
      conversationId ||
      workspace.conversations[workspace.conversations.length - 1]?.id ||
      workspace.id + ":draft",
  );
  const conversation = workspace.conversations.find((item) => item.id === id);
  const draftKey = "meden-composer:" + workspace.id + ":" + id;
  const messages = useMemo<ThreadMessageLike[]>(
    () =>
      (conversation?.messages || []).map((message) => ({
        id: message.id,
        role: message.role,
        content: [{ type: "text", text: message.text }],
        metadata: { custom: { delivery: message.status } },
      })),
    [conversation],
  );
  const onNew = async (message: AppendMessage) => {
    const text = message.content
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n")
      .trim();
    if (!text) return;
    const nextMessage = {
      id: crypto.randomUUID(),
      role: "user" as const,
      text,
      status: "draft" as const,
      createdAt: new Date().toISOString(),
    };
    const next = {
      id,
      title: conversation?.messages.length
        ? conversation.title
        : text.slice(0, 36),
      messages: [...(conversation?.messages || []), nextMessage],
    };
    const saved = await onChange({
      ...workspace,
      conversations: [
        ...workspace.conversations.filter((item) => item.id !== id),
        next,
      ],
    });
    if (saved) localStorage.removeItem(draftKey);
    onSelectConversation(id);
  };
  const runtime = useExternalStoreRuntime<ThreadMessageLike>({
    messages,
    onNew,
    convertMessage: (message) => message,
    isRunning: false,
  });
  useEffect(() => {
    runtime.thread.composer.setText(localStorage.getItem(draftKey) || "");
  }, [draftKey, runtime]);
  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <div className="chat-layout">
        <section className="chat-panel" aria-label="会話">
          <header className="page-heading">
            <Typography variant="h1">
              {conversation?.title || "新しい会話"}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              エージェント未接続 · メッセージは下書きとして保存
            </Typography>
          </header>
          <ThreadPrimitive.Root className="thread">
            <ThreadPrimitive.Viewport className="transcript">
              <ThreadPrimitive.Empty>
                <div className="chat-empty">
                  <Typography color="text.secondary">
                    メッセージを入力してください。
                  </Typography>
                </div>
              </ThreadPrimitive.Empty>
              <ThreadPrimitive.Messages
                components={{ UserMessage, AssistantMessage }}
              />
            </ThreadPrimitive.Viewport>
            <ComposerPrimitive.Root className="composer">
              <ComposerPrimitive.Input
                className="composer-input"
                placeholder="メッセージを入力"
                aria-label="メッセージ"
                submitMode="ctrlEnter"
                onChange={(event) =>
                  localStorage.setItem(draftKey, event.target.value)
                }
              />
              <div className="composer-actions">
                <Typography variant="caption" color="text.secondary">
                  Ctrl + Enter で下書き保存
                </Typography>
                <ComposerPrimitive.Send asChild>
                  <Button
                    variant="contained"
                    endIcon={<SendOutlined />}
                    aria-label="下書きを保存"
                  >
                    下書きを保存
                  </Button>
                </ComposerPrimitive.Send>
              </div>
            </ComposerPrimitive.Root>
          </ThreadPrimitive.Root>
        </section>
        <aside className="requirements-panel">
          <div className="panel-title">
            <Typography variant="h3">要件と証明</Typography>
            <Typography variant="caption" color="text.secondary">
              {workspace.requirements.length}件
            </Typography>
          </div>
          {workspace.requirements.length === 0 ? (
            <Typography
              color="text.secondary"
              variant="body2"
              className="panel-empty"
            >
              要件はまだありません。解析結果を読み込むと、ここに表示されます。
            </Typography>
          ) : (
            workspace.requirements.map((requirement) => {
              const analysis = analysisFor(workspace, {
                claimId: requirement.id,
              });
              const declaration = workspace.declarations.find(
                (item) => item.id === analysis?.declarationId,
              );
              return (
                <div className="requirement-summary" key={requirement.id}>
                  <button onClick={() => onClaim(requirement.id)}>
                    <Typography variant="caption" color="text.secondary">
                      {requirement.id}
                    </Typography>
                    <Typography>{requirement.title}</Typography>
                    <ArrowForwardRounded fontSize="small" />
                  </button>
                  <div className="status-pair">
                    {declaration && (
                      <ProofStatusChip status={declaration.proofStatus} />
                    )}
                    <AnalysisStatusChip
                      analysis={analysis}
                      current={
                        analysis
                          ? analysisIsCurrent(workspace, analysis)
                          : false
                      }
                    />
                  </div>
                  {declaration && (
                    <Button
                      size="small"
                      onClick={() => onGraph(declaration.id)}
                    >
                      依存関係を見る
                    </Button>
                  )}
                </div>
              );
            })
          )}
          <Divider />
          <Typography
            className="panel-footnote"
            variant="caption"
            color="text.secondary"
          >
            証明の状態と意味の一致は、別の結果として表示します。
          </Typography>
        </aside>
      </div>
    </AssistantRuntimeProvider>
  );
}
function UserMessage() {
  const delivery = useAuiState(
    (state) => state.message.metadata.custom.delivery,
  );
  return (
    <MessagePrimitive.Root className="message user-message">
      <div className="message-label">
        あなた <span>{delivery === "draft" ? "未送信" : "履歴"}</span>
      </div>
      <MessagePrimitive.Parts />
    </MessagePrimitive.Root>
  );
}
function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="message assistant-message">
      <div className="message-label">Meden</div>
      <MessagePrimitive.Parts />
    </MessagePrimitive.Root>
  );
}
