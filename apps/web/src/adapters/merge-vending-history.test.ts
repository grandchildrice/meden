import { describe, it, expect } from "vitest";
import {
  createVendingWorkspace,
  DEMO_HISTORY_ID,
} from "../fixtures/meden-demo";
import { mergeVendingHistory } from "./merge-vending-history";
describe("continuous vending-machine history", () => {
  it("retains recorded activity once, repeated requests and all continuations in chronological order", () => {
    const workspace = createVendingWorkspace();
    const request = {
      id: "request",
      role: "user" as const,
      text: "お願いします",
      status: "sent" as const,
      createdAt: "2026-11-18T02:00:00Z",
    };
    workspace.conversations.push(
      { id: "later", title: "later", messages: [request] },
      {
        id: "earlier",
        title: "earlier",
        messages: [
          ...workspace.conversations[0].messages,
          {
            ...request,
            id: "earlier-request",
            createdAt: "2026-11-18T01:00:00Z",
          },
        ],
      },
    );
    const merged = mergeVendingHistory(workspace);
    expect(merged.conversations).toHaveLength(1);
    expect(merged.conversations[0].id).toBe(DEMO_HISTORY_ID);
    expect(merged.conversations[0].messages).toHaveLength(15);
    expect(
      merged.conversations[0].messages.slice(-2).map((message) => message.id),
    ).toEqual(["earlier-request", "request"]);
    expect(
      merged.conversations[0].messages.flatMap(
        (message) => message.activities || [],
      ),
    ).toHaveLength(25);
    expect(workspace.conversations).toHaveLength(3);
  });
  it("keeps conflicting message identities without losing content and leaves other projects alone", () => {
    const workspace = createVendingWorkspace();
    const original = workspace.conversations[0].messages[0];
    workspace.conversations.push({
      id: "more",
      title: "more",
      messages: [{ ...original, text: "別の発言" }],
    });
    const merged = mergeVendingHistory(workspace);
    expect(merged.conversations[0].messages).toHaveLength(14);
    expect(
      new Set(merged.conversations[0].messages.map((message) => message.id))
        .size,
    ).toBe(14);
    expect(
      merged.conversations[0].messages.some(
        (message) => message.text === "別の発言",
      ),
    ).toBe(true);
    const other = { ...workspace, id: "another-project" };
    expect(mergeVendingHistory(other)).toBe(other);
  });
});
