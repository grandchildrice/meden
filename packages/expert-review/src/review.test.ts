import { describe, expect, it } from "vitest";
import { createReviewPackage } from "./index";
import { sampleWorkspace } from "../../../apps/web/src/fixtures/ledger";

function setup() {
  const workspace = sampleWorkspace();
  const draft = {
    id: "request-1",
    title: "レビュー",
    notes: "同額のケース",
    claimIds: ["REQ-001"],
    sourceRevision: workspace.sourceRevision,
    inputRevision: workspace.inputRevision,
    updatedAt: new Date().toISOString(),
  };
  return { workspace, draft };
}
describe("local review package", () => {
  it("limits content to the selected claims, semantic dependencies and linked evidence", () => {
    const { workspace, draft } = setup();
    const result = createReviewPackage(workspace, draft);
    expect(result.requirements.map((claim) => claim.id)).toEqual(["REQ-001"]);
    expect(result.analyses.map((analysis) => analysis.claimId)).toEqual([
      "REQ-001",
    ]);
    expect(
      result.declarations.some(
        (declaration) => declaration.id === "Ledger.u64Max",
      ),
    ).toBe(true);
    expect(
      result.declarations.some(
        (declaration) => declaration.id === "Ledger.retry_unchanged",
      ),
    ).toBe(false);
    expect(
      result.declarations.some(
        (declaration) => declaration.id === "Nat.add_sub_cancel",
      ),
    ).toBe(false);
    expect(result).not.toHaveProperty("conversations");
    expect(result.encryption).toBe("none");
  });
  it("refuses stale target versions and absent claims", () => {
    const { workspace, draft } = setup();
    expect(() =>
      createReviewPackage(workspace, { ...draft, inputRevision: 0 }),
    ).toThrow();
    expect(() =>
      createReviewPackage(workspace, { ...draft, sourceRevision: "other" }),
    ).toThrow();
    expect(() =>
      createReviewPackage(workspace, { ...draft, claimIds: ["missing"] }),
    ).toThrow();
    expect(() =>
      createReviewPackage(workspace, { ...draft, claimIds: [] }),
    ).toThrow();
  });
});
