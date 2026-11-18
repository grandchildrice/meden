import { workspaceSchema, type Workspace } from "@meden/workspace-schema";
import snapshot from "./snapshot.json" with { type: "json" };
import opening from "./opening-request.json" with { type: "json" };
import { mergeVendingHistory } from "../../adapters/merge-vending-history";
import { recordedSourceGraph } from "./source-graph";

export const DEMO_WORKSPACE_ID = "meden-demo-vending";
export const DEMO_HISTORY_ID = "meden-demo-history";
export const demoSnapshot = snapshot;
const labels: Record<string, string> = {
  insert_exact: "残高の上限内なら、投入額だけ残高が増える",
  overflow_preserves: "残高の上限を超える投入では、何も変わらない",
  purchase_exact: "購入が成立すると、残高が150円、在庫が1個減る",
  purchase_rejected: "残高が150円未満か在庫が0個なら、購入は成立しない",
  refund_exact: "残高の全額が返金され、残高は0円になる",
  invalid_preserves: "対応していない操作では、何も変わらない",
  stock_change_only_buy: "在庫が変わるのは、購入が成立したときだけ",
  refund_preserved: "返金操作以外では、直近の返金額は変わらない",
};
export function createVendingWorkspace(): Workspace {
  const sourceRevision = "sha256:" + snapshot.sourceSha256;
  const { declarations, edges } = recordedSourceGraph(labels);
  return workspaceSchema.parse({
    schemaVersion: 1,
    id: DEMO_WORKSPACE_ID,
    name: snapshot.name,
    sourceRevision,
    inputRevision: 1,
    origin: "replay",
    updatedAt: snapshot.recordedAt,
    requirements: snapshot.contract.properties.map((property) => ({
      id: property.name,
      title: labels[property.name],
      text: property.description,
      source: { file: "contract.json" },
    })),
    declarations,
    edges,
    analyses: snapshot.contract.properties.map((property) => ({
      id: "review-" + property.name,
      claimId: property.name,
      declarationId: "KernelProof.law_" + property.name,
      inputRevision: 1,
      sourceRevision,
      status: "pending",
      verdict: "unclear",
      backTranslation: property.description,
      reasoning:
        "形式証明は保存された検証記録を表示しています。意味の比較はGUIのデモ表示で、独立した解析は実行していません。",
      evidenceRefs: ["recorded-verification"],
      modelVersions: [],
      assumptions: snapshot.contract.assumptions,
    })),
    evidence: [
      {
        id: "recorded-verification",
        kind: "report",
        label: "meden-demoの検証記録",
        content:
          snapshot.report.scope +
          "\n\nsource_sha256: " +
          snapshot.sourceSha256 +
          "\n\n" +
          snapshot.report.audit.join("\n"),
      },
    ],
    conversations: [
      {
        id: DEMO_HISTORY_ID,
        title: snapshot.name,
        messages: [
          {
            id: opening.id,
            role: "user",
            text: opening.text,
            status: "sent",
            origin: "provided",
            createdAt: snapshot.recordedAt,
          },
          ...snapshot.transcript.map((message, index) => ({
            id: "recorded-" + index,
            role: message.role,
            text: message.text,
            status: "received",
            createdAt: snapshot.recordedAt,
            origin: "recorded",
            activities:
              "activities" in message ? message.activities : undefined,
          })),
        ],
      },
    ],
    reviewDrafts: [],
  });
}
export function titleFromInstruction(text: string): string {
  if (/自販機|自動販売機|vending\s*machine/i.test(text))
    return "自販機シミュレーター";
  return (
    Array.from(text.trim().split("\n")[0]).slice(0, 32).join("") || "新しい会話"
  );
}
export function demoResponse(text: string, workspace: Workspace): string {
  if (/証明|検証|verify/i.test(text) && workspace.id === DEMO_WORKSPACE_ID) {
    return "保存されている検証記録では、kernel.rsの純粋なstepについて8件の性質が検証されています。\n\n右側で証明の状態と依存関係を確認できます。CLIの入力解析・表示・I/Oは形式検証の対象外です。";
  }
  return workspace.id === DEMO_WORKSPACE_ID
    ? "meden-demoの自販機シミュレーターを開きました。商品は150円で、投入・購入・返金に対応しています。\n\n右側のプレビューを操作できます。コードと証明も同じパネルで確認できます。"
    : "読み込んだワークスペースを右側で確認できます。";
}

/** Refresh this bundled recording's graph and display titles; retain conversation and edit history. */
export function refreshVendingGraph(workspace: Workspace): Workspace {
  if (
    workspace.id !== DEMO_WORKSPACE_ID ||
    workspace.sourceRevision !== "sha256:" + snapshot.sourceSha256
  )
    return workspace;
  workspace = mergeVendingHistory(workspace);
  const existingOpening = workspace.conversations
    .flatMap((conversation) => conversation.messages)
    .find((message) => message.id === opening.id);
  const openingMessage = {
    ...createOpeningMessage(),
    ...(existingOpening?.activities
      ? { activities: existingOpening.activities }
      : {}),
  };
  const graph = recordedSourceGraph(labels);
  return workspaceSchema.parse({
    ...workspace,
    ...graph,
    requirements: workspace.requirements.map((requirement) => ({
      ...requirement,
      title: labels[requirement.id] ?? requirement.title,
    })),
    conversations: workspace.conversations.map((conversation) =>
      conversation.id !== DEMO_HISTORY_ID
        ? conversation
        : {
            ...conversation,
            messages: [
              openingMessage,
              ...conversation.messages
                .filter((message) => message.id !== opening.id)
                .map((message) => {
                  const index =
                    message.origin === "recorded"
                      ? Number(message.id.replace(/^recorded-/, ""))
                      : NaN;
                  const original = snapshot.transcript[index];
                  return original
                    ? {
                        ...message,
                        activities:
                          "activities" in original
                            ? original.activities
                            : undefined,
                      }
                    : message;
                }),
            ],
          },
    ),
    declarations: graph.declarations.map((declaration) => {
      if (declaration.proofStatus === "proved" && workspace.inputRevision !== 1)
        return { ...declaration, proofStatus: "stale" };
      return declaration;
    }),
  });
}

function createOpeningMessage() {
  return {
    id: opening.id,
    role: "user" as const,
    text: opening.text,
    status: "sent" as const,
    origin: "provided" as const,
    createdAt: snapshot.recordedAt,
  };
}
export const vendingOpeningRequest = opening.text;
