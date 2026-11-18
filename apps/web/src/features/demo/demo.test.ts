import { importWorkspace } from "@meden/workspace-schema";
import { createHash } from "node:crypto";
import { describe, it, expect } from "vitest";
import {
  createVendingWorkspace,
  demoSnapshot,
  titleFromInstruction,
} from "../../fixtures/meden-demo";
import { previewStep } from "./VendingPreview";
describe("recorded meden-demo and UI preview", () => {
  it("uses the matching kernel, eight property records and the original visible conversation", () => {
    const workspace = createVendingWorkspace();
    expect(
      createHash("sha256")
        .update(demoSnapshot.files["kernel.rs"])
        .digest("hex"),
    ).toBe(demoSnapshot.sourceSha256);
    expect(workspace.requirements).toHaveLength(8);
    expect(
      importWorkspace(workspace).analyses.every(
        (analysis) => analysis.status === "pending",
      ),
    ).toBe(true);
    expect(workspace.conversations[0].messages).toHaveLength(13);
    expect(workspace.conversations[0].messages[0].text).toBe(
      "開発を続けられますか？",
    );
    expect(
      workspace.conversations[0].messages.every(
        (message) => message.origin === "recorded",
      ),
    ).toBe(true);
    expect(
      workspace.analyses.every(
        (analysis) =>
          analysis.status === "pending" && analysis.modelVersions.length === 0,
      ),
    ).toBe(true);
    expect(JSON.stringify(demoSnapshot)).not.toMatch(
      /api_key|access_token|tool_calls|role.*system/,
    );
  });
  it("derives conversation titles from instructions, never a separate naming form", () => {
    expect(titleFromInstruction("自販機を作ってください")).toBe(
      "自販機シミュレーター",
    );
    expect(titleFromInstruction("  購入条件を確認して\n境界も調べて")).toBe(
      "購入条件を確認して",
    );
    expect(titleFromInstruction("")).toBe("新しい会話");
  });
  it("preserves rejected purchases and exactly changes balance/stock on success", () => {
    const empty = { balance: 100n, stock: 2n, refunded: 7n };
    expect(previewStep(empty, "buy")).toBe(empty);
    expect(previewStep({ ...empty, balance: 200n }, "buy")).toEqual({
      balance: 50n,
      stock: 1n,
      refunded: 7n,
    });
    const out = { balance: 200n, stock: 0n, refunded: 0n };
    expect(previewStep(out, "buy")).toBe(out);
  });
  it("refuses overflow and records the most recent refund without changing stock", () => {
    const max = { balance: (1n << 64n) - 1n, stock: 10n, refunded: 5n };
    expect(previewStep(max, "insert", 1n)).toBe(max);
    expect(previewStep(max, "refund")).toEqual({
      balance: 0n,
      stock: 10n,
      refunded: max.balance,
    });
    expect(
      previewStep({ balance: 0n, stock: 1n, refunded: 3n }, "refund").refunded,
    ).toBe(0n);
  });
});
