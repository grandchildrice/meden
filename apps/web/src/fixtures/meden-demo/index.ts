import {
  workspaceSchema,
  type Workspace,
  type Declaration,
} from "@meden/workspace-schema";
import snapshot from "./snapshot.json";

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
function leanDeclaration(
  file: string,
  name: string,
): { statement: string; line: number } {
  const lines = file.split("\n");
  const start = lines.findIndex((line) =>
    new RegExp(
      "^(theorem|def|structure) " + name.replaceAll(".", "\\.") + "(?: |\\s|$)",
    ).test(line),
  );
  if (start < 0) return { statement: name, line: 1 };
  let end = start + 1;
  while (
    end < lines.length &&
    !/^(theorem|def|structure|end )/.test(lines[end])
  )
    end++;
  return {
    statement: lines.slice(start, end).join("\n").split(":= by")[0].trim(),
    line: start + 1,
  };
}
export function createVendingWorkspace(): Workspace {
  const sourceRevision = "sha256:" + snapshot.sourceSha256;
  const declarations: Declaration[] = [];
  const edges: Workspace["edges"] = [];
  for (const property of snapshot.contract.properties) {
    const id = "KernelProof.law_" + property.name;
    const predicate = "KernelContract.property_" + property.name;
    const spec = "KernelContract.law_" + property.name;
    const actual = leanDeclaration(
      snapshot.files["Check.lean"],
      "law_" + property.name,
    );
    const axiomsLine = snapshot.report.audit.find((line) =>
      line.startsWith("'" + id + "'"),
    );
    const axioms =
      axiomsLine
        ?.match(/\[([^\]]*)\]/)?.[1]
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean) || [];
    declarations.push({
      id,
      name: id,
      label: labels[property.name],
      kind: "theorem",
      statement: actual.statement,
      source: { file: "Check.lean", line: actual.line },
      proofStatus: "proved",
      proofSourceRevision: sourceRevision,
      proofInputRevision: 1,
      evidenceRefs: ["recorded-verification"],
      axioms,
    });
    const formal = leanDeclaration(
      snapshot.files["Spec.lean"],
      "property_" + property.name,
    );
    declarations.push({
      id: predicate,
      name: predicate,
      label: property.name,
      kind: "definition",
      statement: formal.statement,
      source: { file: "Spec.lean", line: formal.line },
      proofStatus: "unknown",
      evidenceRefs: [],
      axioms: [],
    });
    const law = leanDeclaration(
      snapshot.files["Spec.lean"],
      "law_" + property.name,
    );
    declarations.push({
      id: spec,
      name: spec,
      label: "仕様の性質",
      kind: "theorem",
      statement: law.statement,
      source: { file: "Spec.lean", line: law.line },
      proofStatus: "proved",
      proofSourceRevision: sourceRevision,
      proofInputRevision: 1,
      evidenceRefs: ["recorded-verification"],
      axioms: [],
    });
    edges.push(
      { source: id, target: predicate, site: "type" },
      { source: id, target: "KernelImpl.kernel.step", site: "type" },
      { source: id, target: "KernelProof.abstractState", site: "type" },
      { source: id, target: spec, site: "value" },
      { source: predicate, target: "KernelContract.State", site: "type" },
      { source: spec, target: "KernelContract.transition", site: "type" },
    );
  }
  for (const [id, label, file, declaration] of [
    [
      "KernelImpl.kernel.step",
      "Rustの状態遷移",
      "Candidate.lean",
      "kernel.step",
    ],
    [
      "KernelImpl.kernel.Machine",
      "Rustの状態",
      "Candidate.lean",
      "kernel.Machine",
    ],
    ["KernelContract.transition", "仕様の状態遷移", "Spec.lean", "transition"],
    ["KernelContract.State", "仕様の状態", "Spec.lean", "State"],
    [
      "KernelProof.abstractState",
      "Rustと仕様の状態対応",
      "Check.lean",
      "abstractState",
    ],
  ] as const) {
    const source = leanDeclaration(snapshot.files[file], declaration);
    declarations.push({
      id,
      name: id,
      label,
      kind: "definition",
      statement: source.statement,
      source: { file, line: source.line },
      proofStatus: "unknown",
      evidenceRefs: [],
      axioms: [],
    });
  }
  edges.push(
    {
      source: "KernelImpl.kernel.step",
      target: "KernelImpl.kernel.Machine",
      site: "type",
    },
    {
      source: "KernelProof.abstractState",
      target: "KernelImpl.kernel.Machine",
      site: "type",
    },
    {
      source: "KernelProof.abstractState",
      target: "KernelContract.State",
      site: "type",
    },
    {
      source: "KernelContract.transition",
      target: "KernelContract.State",
      site: "type",
    },
  );
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
    ? "meden-demoの自販機シミュレーターを開きました。商品は150円で、投入・購入・返金に対応しています。\n\n右側のプレビューを操作できます。証明、依存関係、コードも同じパネルで確認できます。"
    : "読み込んだワークスペースを右側で確認できます。";
}
