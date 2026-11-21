import { workspaceSchema, type Workspace } from "@meden/workspace-schema";
import recordedSnapshot from "./snapshot.json" with { type: "json" };
import recordedOpening from "./opening-request.json" with { type: "json" };
import { mergeVendingHistory } from "../../adapters/merge-vending-history";
import { recordedSourceGraph } from "./source-graph";

import {
  messages,
  openingText,
  propertyDescriptions,
  assumptions,
} from "./english";
const opening = { ...recordedOpening, text: openingText };
const snapshot = {
  ...recordedSnapshot,
  name: "Vending machine simulator",
  contract: {
    ...recordedSnapshot.contract,
    description:
      "Pure vending machine step for all u64 inputs. Fixed price 150. Insertion refuses overflow; purchase requires balance at least 150 and positive stock; refund returns the full balance; unsupported actions preserve state. Only refunds update the last refund.",
    properties: recordedSnapshot.contract.properties.map((p, i) => ({
      ...p,
      description: propertyDescriptions[i] || p.description,
    })),
    assumptions,
  },
  transcript: recordedSnapshot.transcript.map((m, i) => ({
    ...m,
    text: messages[i] || m.text,
  })),
};

export const DEMO_WORKSPACE_ID = "meden-demo-vending";
export const DEMO_HISTORY_ID = "meden-demo-history";
export const demoSnapshot = snapshot;
const labels: Record<string, string> = {
  insert_exact:
    "Inserting within the balance limit adds exactly the inserted amount",
  overflow_preserves:
    "An insertion exceeding the balance limit changes nothing",
  purchase_exact: "A successful purchase subtracts 150 yen and one item",
  purchase_rejected: "Purchase is rejected below 150 yen or when out of stock",
  refund_exact: "Refund returns the full balance and resets it to zero",
  invalid_preserves: "Unsupported operations change nothing",
  stock_change_only_buy: "Only successful purchases change stock",
  refund_preserved: "Only refunds change the last refunded amount",
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
        "Proof status comes from saved verification records. The meaning comparison is a GUI demo; no independent analysis was run.",
      evidenceRefs: ["recorded-verification"],
      modelVersions: [],
      assumptions: snapshot.contract.assumptions,
    })),
    evidence: [
      {
        id: "recorded-verification",
        kind: "report",
        label: "meden-demo verification record",
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
    return "Vending machine simulator";
  return (
    Array.from(text.trim().split("\n")[0]).slice(0, 32).join("") ||
    "New conversation"
  );
}
export function demoResponse(text: string, workspace: Workspace): string {
  if (/Proof|検証|verify/i.test(text) && workspace.id === DEMO_WORKSPACE_ID) {
    return "The saved record verifies eight properties of the pure step in kernel.rs.\n\nInspect proof status and dependencies on the right. CLI input parsing, rendering and I/O are outside the formal verification scope.";
  }
  return workspace.id === DEMO_WORKSPACE_ID
    ? "Opened the meden-demo vending machine simulator. Items cost 150 yen; insert, purchase and refund are supported.\n\nUse the preview on the right. Code and proofs are available in the same panel."
    : "The imported workspace is available on the right.";
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
