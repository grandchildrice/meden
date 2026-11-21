import { workspaceSchema, type Workspace } from "@meden/workspace-schema";

/** Hand-authored sample. No model, Lean run or real source revision is implied. */
export function sampleWorkspace(): Workspace {
  const now = new Date().toISOString();
  const sourceRevision = "sample-ledger-v1";
  return workspaceSchema.parse({
    schemaVersion: 1,
    id: crypto.randomUUID(),
    name: "Transfers",
    sourceRevision,
    inputRevision: 1,
    origin: "sample",
    updatedAt: now,
    requirements: [
      {
        id: "REQ-001",
        title: "Accept equal-balance transfers",
        text: "Accept transfers whose amount does not exceed the balance.",
        source: { file: "requirements.md", line: 12 },
      },
      {
        id: "REQ-002",
        title: "Preserve total balance",
        text: "The total balance of both accounts is unchanged by a transfer.",
        source: { file: "requirements.md", line: 16 },
      },
      {
        id: "REQ-003",
        title: "No duplicate transfer on retry",
        text: "Retrying the same request ID leaves account balances unchanged.",
        source: { file: "requirements.md", line: 20 },
      },
    ],
    declarations: [
      {
        id: "Ledger.accepted_transfer",
        name: "Ledger.accepted_transfer",
        label: "Transfer admission",
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
        label: "Transfer admission condition",
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
        label: "Ledger state",
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
        label: "Total balance preservation",
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
        label: "Balance after retry",
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
        label: "Total balance",
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
        label: "Transfers",
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
        label: "64-bit integer maximum",
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
        label: "Request ID",
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
        label: "Addition and subtraction lemma",
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
        label: "Proof status sample",
        content: "UI sample report; not a Lean execution result.",
      },
      {
        id: "boundary-001",
        kind: "counterexample",
        label: "Equal-balance boundary case",
        content:
          "Balance: 100\nTransfer amount: 100\nRequirement: Accept\nTheorem assumption: amount < balance  is not satisfied",
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
        backTranslation:
          "Accept transfers whose amount is strictly below the balance.",
        reasoning:
          "The theorem uses amount < balance. Equal-balance transfers do not satisfy this assumption and are outside its scope.",
        evidenceRefs: ["boundary-001", "decl-001"],
        modelVersions: ["sample/back-translation", "sample/judge"],
        assumptions: [
          "Account authorization and request ID are valid.",
          "The receiving balance does not exceed the 64-bit integer maximum.",
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
        backTranslation:
          "The total balance of both accounts is unchanged by a transfer.",
        reasoning:
          "totalBalance is the sum of both balances. The theorem states that this sum is unchanged by transfers.",
        evidenceRefs: ["sample-check"],
        modelVersions: ["sample/back-translation", "sample/judge"],
        assumptions: ["Balances are represented by natural numbers."],
      },
      {
        id: "analysis-003",
        claimId: "REQ-003",
        declarationId: "Ledger.retry_unchanged",
        inputRevision: 1,
        sourceRevision,
        status: "complete",
        verdict: "full",
        backTranslation:
          "Retrying the same request ID leaves account balances unchanged.",
        reasoning:
          "Past or repeated request IDs return the original state. Meaning alignment and proof completion are checked separately.",
        evidenceRefs: [],
        modelVersions: ["sample/back-translation", "sample/judge"],
        assumptions: ["Request IDs are issued in strictly increasing order."],
      },
    ],
    conversations: [
      {
        id: "conversation-sample",
        title: "Transfer specification",
        messages: [
          {
            id: "message-sample",
            role: "user",
            text: "Build a ledger accepting transfers up to the available balance. Preserve total balance and prevent duplicate transfers with the same request ID.",
            status: "draft",
            createdAt: now,
          },
        ],
      },
    ],
    reviewDrafts: [],
  });
}
