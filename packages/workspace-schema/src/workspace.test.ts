import { describe, expect, it } from "vitest";
import {
  analysisIsCurrent,
  analysisFor,
  importWorkspace,
  updateRequirement,
  createWorkspace,
  workspaceSchema,
} from "./index";
import { sampleWorkspace } from "../../../apps/web/src/fixtures/ledger";

describe("workspace data and freshness", () => {
  it("keeps incomplete proof and a matching semantic verdict independent", () => {
    const workspace = sampleWorkspace();
    expect(
      workspace.declarations.find(
        (node) => node.id === "Ledger.retry_unchanged",
      )?.proofStatus,
    ).toBe("unproved");
    expect(
      workspace.analyses.find((item) => item.claimId === "REQ-003")?.verdict,
    ).toBe("full");
  });
  it("invalidates retained proof and analysis when a requirement changes, including after JSON roundtrip", () => {
    const original = sampleWorkspace();
    const edited = updateRequirement(
      original,
      "REQ-001",
      "Changed requirement",
      "2026-11-23T00:00:00.000Z",
    );
    expect(edited).toEqual(
      updateRequirement(
        original,
        "REQ-001",
        "Changed requirement",
        "2026-11-23T00:00:00.000Z",
      ),
    );
    expect(edited.inputRevision).toBe(2);
    expect(
      edited.declarations.find((node) => node.id === "Ledger.accepted_transfer")
        ?.proofStatus,
    ).toBe("stale");
    expect(
      edited.analyses.every((analysis) => !analysisIsCurrent(edited, analysis)),
    ).toBe(true);
    const restored = importWorkspace(JSON.parse(JSON.stringify(edited)));
    expect(
      restored.analyses.every((analysis) => analysis.status === "stale"),
    ).toBe(true);
    expect(original.requirements[0].text).not.toBe("Changed requirement");
  });
  it("creates a reproducible draft from explicit identity and time", () => {
    const metadata = {
      id: "same-input",
      updatedAt: "2026-11-23T00:00:00.000Z",
    };
    const created = createWorkspace("Review", metadata);
    expect(createWorkspace("Review", metadata)).toEqual(created);
    expect(created.id).toBe(metadata.id);
    expect(created.updatedAt).toBe(metadata.updatedAt);
  });
  it("does not invalidate unchanged text or accept an unknown claim edit", () => {
    const workspace = sampleWorkspace();
    expect(
      updateRequirement(
        workspace,
        "REQ-001",
        workspace.requirements[0].text,
        workspace.updatedAt,
      ),
    ).toBe(workspace);
    expect(() =>
      updateRequirement(workspace, "missing", "text", workspace.updatedAt),
    ).toThrow();
  });
  it("refuses dangling graph, analysis and evidence references and duplicate IDs", () => {
    const workspace = sampleWorkspace();
    expect(
      workspaceSchema.safeParse({
        ...workspace,
        declarations: [...workspace.declarations, workspace.declarations[0]],
      }).success,
    ).toBe(false);
    expect(
      workspaceSchema.safeParse({
        ...workspace,
        edges: [
          {
            source: "missing",
            target: workspace.declarations[0].id,
            site: "type",
          },
        ],
      }).success,
    ).toBe(false);
    expect(
      workspaceSchema.safeParse({ ...workspace, evidence: [] }).success,
    ).toBe(false);
    expect(
      workspaceSchema.safeParse({ ...workspace, requirements: [] }).success,
    ).toBe(false);
  });
  it("marks mismatched source revisions stale on import and never treats them as current", () => {
    const workspace = sampleWorkspace();
    workspace.sourceRevision = "new-source";
    const imported = importWorkspace(workspace);
    expect(imported.origin).toBe("sample");
    expect(importWorkspace({ ...workspace, origin: "local" }).origin).toBe(
      "imported",
    );
    expect(
      imported.analyses.every((analysis) => analysis.status === "stale"),
    ).toBe(true);
    expect(
      imported.declarations.find(
        (declaration) => declaration.id === "Ledger.accepted_transfer",
      )?.proofStatus,
    ).toBe("stale");
  });
  it("prefers a current result over a later imported historical result", () => {
    const workspace = sampleWorkspace();
    const current = workspace.analyses[0];
    workspace.analyses.push({
      ...current,
      id: "historical",
      sourceRevision: "older",
    });
    expect(analysisFor(workspace, { claimId: "REQ-001" })?.id).toBe(current.id);
    expect(
      analysisFor(workspace, { declarationId: current.declarationId })?.id,
    ).toBe(current.id);
    expect(analysisFor(workspace, {})).toBeUndefined();
  });
  it("rejects unknown schema versions and unsupported authority-like fields", () => {
    const workspace = sampleWorkspace();
    expect(
      workspaceSchema.safeParse({ ...workspace, schemaVersion: 2 }).success,
    ).toBe(false);
    expect(
      workspaceSchema.safeParse({ ...workspace, acceptedContract: true })
        .success,
    ).toBe(false);
  });
});
