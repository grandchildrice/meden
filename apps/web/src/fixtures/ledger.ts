import { workspaceSchema, type Workspace } from "@meden/workspace-schema";

/** Hand-authored sample. No model, Lean run or real source revision is implied. */
export function sampleWorkspace(): Workspace {
  const now = new Date().toISOString();
  const sourceRevision = "sample-ledger-v1";
  return workspaceSchema.parse({
    schemaVersion: 1,
    id: crypto.randomUUID(),
    name: "送金処理",
    sourceRevision,
    inputRevision: 1,
    origin: "sample",
    updatedAt: now,
    requirements: [
      {
        id: "REQ-001",
        title: "同額の送金を受け付ける",
        text: "送金額が残高以下なら、送金を受け付ける。",
        source: { file: "requirements.md", line: 12 },
      },
      {
        id: "REQ-002",
        title: "総残高を保つ",
        text: "送金の前後で、二つの口座の総残高が変わらない。",
        source: { file: "requirements.md", line: 16 },
      },
      {
        id: "REQ-003",
        title: "再試行で二重送金しない",
        text: "同じ要求IDで再試行しても、口座の残高を変更しない。",
        source: { file: "requirements.md", line: 20 },
      },
    ],
    declarations: [
      {
        id: "Ledger.accepted_transfer",
        name: "Ledger.accepted_transfer",
        label: "送金の受理",
        kind: "theorem",
        statement:
          "∀ (s : LedgerState) (amount : Nat),\n  transferAllowed s amount →\n  (transfer s amount).accepted = true",
        source: { file: "Ledger.lean", line: 42 },
        proofStatus: "proved",
        proofSourceRevision: sourceRevision,
        proofInputRevision: 1,
        evidenceRefs: ["sample-check"],
        axioms: [],
      },
      {
        id: "Ledger.transferAllowed",
        name: "Ledger.transferAllowed",
        label: "送金の受理条件",
        kind: "definition",
        statement:
          "def transferAllowed (s : LedgerState) (amount : Nat) : Prop :=\n  0 < amount ∧ amount < s.debit ∧\n  s.credit + amount ≤ u64Max",
        source: { file: "Ledger.lean", line: 18 },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
      {
        id: "Ledger.State",
        name: "Ledger.State",
        label: "台帳の状態",
        kind: "definition",
        statement:
          "structure LedgerState where\n  debit : Nat\n  credit : Nat\n  lastRequest : Nat",
        source: { file: "Ledger.lean", line: 4 },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
      {
        id: "Ledger.total_balance",
        name: "Ledger.total_balance",
        label: "総残高の保存",
        kind: "theorem",
        statement:
          "∀ s amount,\n  totalBalance (transfer s amount) = totalBalance s",
        source: { file: "Ledger.lean", line: 68 },
        proofStatus: "proved",
        proofSourceRevision: sourceRevision,
        proofInputRevision: 1,
        evidenceRefs: ["sample-check"],
        axioms: [],
      },
      {
        id: "Ledger.retry_unchanged",
        name: "Ledger.retry_unchanged",
        label: "再試行時の残高",
        kind: "theorem",
        statement:
          "∀ s request,\n  request.id ≤ s.lastRequest →\n  applyRequest s request = s",
        source: { file: "Ledger.lean", line: 91 },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
      {
        id: "Ledger.totalBalance",
        name: "Ledger.totalBalance",
        label: "総残高",
        kind: "definition",
        statement: "def totalBalance (s : LedgerState) := s.debit + s.credit",
        source: { file: "Ledger.lean", line: 15 },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
      {
        id: "Ledger.transfer",
        name: "Ledger.transfer",
        label: "送金処理",
        kind: "definition",
        statement:
          "def transfer (s : LedgerState) (amount : Nat) :=\n  if transferAllowed s amount then\n    { s with debit := s.debit - amount,\n             credit := s.credit + amount }\n  else s",
        source: { file: "Ledger.lean", line: 25 },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
      {
        id: "Ledger.u64Max",
        name: "Ledger.u64Max",
        label: "64ビット整数の上限",
        kind: "definition",
        statement: "def u64Max : Nat := 18446744073709551615",
        source: { file: "Ledger.lean", line: 12 },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
      {
        id: "Ledger.requestId",
        name: "Ledger.requestId",
        label: "要求ID",
        kind: "definition",
        statement: "def requestId (request : Request) : Nat := request.id",
        source: { file: "Ledger.lean", line: 80 },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
      {
        id: "Nat.add_sub_cancel",
        name: "Nat.add_sub_cancel",
        label: "加減算の補題",
        kind: "theorem",
        statement: "∀ n m : Nat, n + m - m = n",
        source: { file: "Mathlib/Data/Nat/Basic.lean" },
        proofStatus: "unproved",
        evidenceRefs: [],
        axioms: [],
      },
    ],
    edges: [
      {
        source: "Ledger.accepted_transfer",
        target: "Ledger.transferAllowed",
        site: "type",
      },
      {
        source: "Ledger.accepted_transfer",
        target: "Ledger.State",
        site: "type",
      },
      {
        source: "Ledger.accepted_transfer",
        target: "Ledger.transfer",
        site: "type",
      },
      {
        source: "Ledger.accepted_transfer",
        target: "Nat.add_sub_cancel",
        site: "value",
      },
      {
        source: "Ledger.transferAllowed",
        target: "Ledger.State",
        site: "type",
      },
      {
        source: "Ledger.transferAllowed",
        target: "Ledger.u64Max",
        site: "value",
      },
      {
        source: "Ledger.transfer",
        target: "Ledger.transferAllowed",
        site: "value",
      },
      {
        source: "Ledger.total_balance",
        target: "Ledger.totalBalance",
        site: "type",
      },
      {
        source: "Ledger.total_balance",
        target: "Ledger.transfer",
        site: "type",
      },
      { source: "Ledger.totalBalance", target: "Ledger.State", site: "type" },
      {
        source: "Ledger.retry_unchanged",
        target: "Ledger.requestId",
        site: "type",
      },
      {
        source: "Ledger.retry_unchanged",
        target: "Ledger.State",
        site: "type",
      },
    ],
    evidence: [
      {
        id: "sample-check",
        kind: "report",
        label: "証明状態のサンプル",
        content:
          "画面確認用に作成したレポートです。Leanの実行結果ではありません。",
      },
      {
        id: "boundary-001",
        kind: "counterexample",
        label: "同額送金の境界ケース",
        content:
          "残高: 100\n送金額: 100\n要件: 受理する\n定理の前提: amount < balance を満たさない",
      },
      {
        id: "decl-001",
        kind: "declaration",
        label: "Ledger.transferAllowed",
        content: "0 < amount ∧ amount < s.debit ∧ s.credit + amount ≤ u64Max",
      },
    ],
    analyses: [
      {
        id: "analysis-001",
        claimId: "REQ-001",
        declarationId: "Ledger.accepted_transfer",
        inputRevision: 1,
        sourceRevision,
        status: "complete",
        verdict: "partial",
        backTranslation: "送金額が残高未満なら、送金を受け付ける。",
        reasoning:
          "定理が使う受理条件は amount < balance です。同額の送金は前提を満たさず、この定理の対象に含まれません。",
        evidenceRefs: ["boundary-001", "decl-001"],
        modelVersions: ["sample/back-translation", "sample/judge"],
        assumptions: [
          "口座の権限と要求IDが有効であること。",
          "受取口座の残高が64ビット整数の上限を超えないこと。",
        ],
      },
      {
        id: "analysis-002",
        claimId: "REQ-002",
        declarationId: "Ledger.total_balance",
        inputRevision: 1,
        sourceRevision,
        status: "complete",
        verdict: "full",
        backTranslation: "送金の前後で、二つの口座の総残高が変わらない。",
        reasoning:
          "totalBalance は二つの残高の和です。定理は送金前後の和が等しいことを述べています。",
        evidenceRefs: ["sample-check"],
        modelVersions: ["sample/back-translation", "sample/judge"],
        assumptions: ["残高の表現を自然数とする。"],
      },
      {
        id: "analysis-003",
        claimId: "REQ-003",
        declarationId: "Ledger.retry_unchanged",
        inputRevision: 1,
        sourceRevision,
        status: "complete",
        verdict: "full",
        backTranslation: "同じ要求IDで再試行しても、口座の残高を変更しない。",
        reasoning:
          "過去または同一の要求IDに対して、元の状態を返すという命題です。意味の比較と証明の完了は別に確認します。",
        evidenceRefs: [],
        modelVersions: ["sample/back-translation", "sample/judge"],
        assumptions: ["要求IDは単調増加で発行される。"],
      },
    ],
    conversations: [
      {
        id: "conversation-sample",
        title: "送金処理の仕様",
        messages: [
          {
            id: "message-sample",
            role: "user",
            text: "残高以下の送金を受け付ける台帳を作成してください。送金で総残高が変わらず、同じ要求IDでは二重に送金されないようにします。",
            status: "draft",
            createdAt: now,
          },
        ],
      },
    ],
    reviewDrafts: [],
  });
}
