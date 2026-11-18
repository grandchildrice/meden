import { workspaceSchema, type Workspace } from "@meden/workspace-schema";
import snapshot from "./snapshot.json" with { type: "json" };
import { recordedSourceGraph } from "./source-graph";

export const DEMO_WORKSPACE_ID = "meden-demo-vending";
export const DEMO_HISTORY_ID = "meden-demo-history";
export const demoSnapshot = snapshot;
const labels: Record<string, string> = {
  insert_exact: "投入額を残高に加える",
  overflow_preserves: "オーバーフロー時は変更しない",
  purchase_exact: "150円で商品を購入する",
  purchase_rejected: "残高不足・在庫切れを拒否する",
  refund_exact: "残高を全額返金する",
  invalid_preserves: "未定義の操作を拒否する",
  stock_change_only_buy: "購入成功時だけ在庫を減らす",
  refund_preserved: "返金以外で返金額を変えない",
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
        messages: snapshot.transcript.map((message, index) => ({
          id: "recorded-" + index,
          role: message.role,
          text: message.text,
          status: "received",
          createdAt: snapshot.recordedAt,
          origin: "recorded",
          activities: "activities" in message ? message.activities : undefined,
        })),
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

/** Refresh only this bundled recording's graph; retain conversation and edit history. */
export function refreshVendingGraph(workspace: Workspace): Workspace {
  if (
    workspace.id !== DEMO_WORKSPACE_ID ||
    workspace.sourceRevision !== "sha256:" + snapshot.sourceSha256
  )
    return workspace;
  const graph = recordedSourceGraph(labels);
  return workspaceSchema.parse({
    ...workspace,
    ...graph,
    conversations: workspace.conversations.map((conversation) =>
      conversation.id !== DEMO_HISTORY_ID
        ? conversation
        : {
            ...conversation,
            messages: conversation.messages.map((message) => {
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
          },
    ),
    declarations: graph.declarations.map((declaration) => {
      if (declaration.proofStatus === "proved" && workspace.inputRevision !== 1)
        return { ...declaration, proofStatus: "stale" };
      return declaration;
    }),
  });
}
