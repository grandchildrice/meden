import type { Conversation, Workspace } from "@meden/workspace-schema";

export const VENDING_WORKSPACE_ID = "meden-demo-vending";
const HISTORY_ID = "meden-demo-history";
/** The bundled vending example is one continuous project conversation. */
export function mergeVendingHistory(workspace: Workspace): Workspace {
  if (
    workspace.id !== VENDING_WORKSPACE_ID ||
    workspace.conversations.length === 0
  )
    return workspace;
  if (
    workspace.conversations.length === 1 &&
    workspace.conversations[0].id === HISTORY_ID &&
    workspace.conversations[0].title === workspace.name
  )
    return workspace;
  const canonical =
    workspace.conversations.find((item) => item.id === HISTORY_ID) ||
    workspace.conversations[0];
  const ordered = [
    canonical,
    ...workspace.conversations.filter((item) => item !== canonical),
  ];
  const entries = ordered.flatMap((conversation, thread) =>
    conversation.messages.map((message, index) => ({ message, thread, index })),
  );
  entries.sort(
    (a, b) =>
      Date.parse(a.message.createdAt) - Date.parse(b.message.createdAt) ||
      a.thread - b.thread ||
      a.index - b.index,
  );
  const seen = new Map<string, string>();
  const messages: Conversation["messages"] = [];
  for (const { message, thread, index } of entries) {
    const content = JSON.stringify(message);
    if (seen.get(message.id) === content) continue;
    let id = message.id;
    if (seen.has(id)) {
      id = "merged-" + thread + "-" + index;
      while (seen.has(id)) id += "-copy";
    }
    seen.set(id, content);
    messages.push(id === message.id ? message : { ...message, id });
  }
  return {
    ...workspace,
    conversations: [
      { ...canonical, id: HISTORY_ID, title: workspace.name, messages },
    ],
  };
}
