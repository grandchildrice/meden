import { z } from "zod";

const id = z.string().min(1).max(256);
const text = z.string().max(100_000);
export const proofStatuses = [
  "proved",
  "bounded",
  "empirical",
  "refuted",
  "unproved",
  "unknown",
  "unsupported",
  "stale",
] as const;
export const verdicts = [
  "full",
  "partial",
  "mismatch",
  "vacuous",
  "missing",
  "unformalizable",
  "unclear",
] as const;
export const sourceSchema = z
  .object({
    file: z.string().max(4096),
    line: z.number().int().positive().optional(),
  })
  .strict();
export const requirementSchema = z
  .object({
    id,
    title: z.string().min(1).max(300),
    text,
    source: sourceSchema.optional(),
  })
  .strict();
export const declarationSchema = z
  .object({
    id,
    name: z.string().min(1).max(1000),
    label: z.string().max(300),
    kind: z.enum(["theorem", "definition", "axiom"]),
    statement: text,
    summary: z.string().max(10_000).optional(),
    source: sourceSchema.optional(),
    proofStatus: z.enum(proofStatuses),
    evidenceRefs: z.array(id).max(1000),
    proofSourceRevision: id.optional(),
    proofInputRevision: z.number().int().nonnegative().optional(),
    axioms: z.array(z.string().max(1000)).max(1000),
  })
  .strict();
export const edgeSchema = z
  .object({
    source: id,
    target: id,
    site: z.enum(["type", "value", "unknown"]),
  })
  .strict();
export const evidenceSchema = z
  .object({
    id,
    kind: z.enum(["declaration", "requirement", "counterexample", "report"]),
    label: z.string().max(300),
    content: text,
  })
  .strict();
export const analysisSchema = z
  .object({
    id,
    claimId: id,
    declarationId: id,
    inputRevision: z.number().int().nonnegative(),
    sourceRevision: id,
    status: z.enum(["complete", "pending", "failed", "stale"]),
    verdict: z.enum(verdicts),
    backTranslation: text,
    reasoning: text,
    evidenceRefs: z.array(id).max(1000),
    modelVersions: z.array(z.string().max(300)).max(32),
    assumptions: z.array(z.string().max(3000)).max(1000),
  })
  .strict();
export const messageSchema = z
  .object({
    id,
    role: z.enum(["user", "assistant"]),
    text,
    status: z.enum(["draft", "received"]),
    createdAt: z.string().datetime(),
  })
  .strict();
export const conversationSchema = z
  .object({
    id,
    title: z.string().max(300),
    messages: z.array(messageSchema).max(10_000),
  })
  .strict();
export const reviewDraftSchema = z
  .object({
    id,
    title: z.string().max(300),
    notes: text,
    claimIds: z.array(id).max(1000),
    sourceRevision: id,
    inputRevision: z.number().int().nonnegative(),
    updatedAt: z.string().datetime(),
  })
  .strict();

export const workspaceSchema = z
  .object({
    schemaVersion: z.literal(1),
    id,
    name: z.string().min(1).max(300),
    sourceRevision: id,
    inputRevision: z.number().int().nonnegative(),
    origin: z.enum(["local", "imported", "sample"]),
    updatedAt: z.string().datetime(),
    requirements: z.array(requirementSchema).max(10_000),
    declarations: z.array(declarationSchema).max(20_000),
    edges: z.array(edgeSchema).max(100_000),
    analyses: z.array(analysisSchema).max(10_000),
    evidence: z.array(evidenceSchema).max(20_000),
    conversations: z.array(conversationSchema).max(1000),
    reviewDrafts: z.array(reviewDraftSchema).max(1000),
  })
  .strict()
  .superRefine((workspace, ctx) => {
    for (const key of [
      "requirements",
      "declarations",
      "analyses",
      "evidence",
      "conversations",
      "reviewDrafts",
    ] as const) {
      const ids = workspace[key].map((item) => item.id);
      if (new Set(ids).size !== ids.length)
        ctx.addIssue({
          code: "custom",
          path: [key],
          message: "IDが重複しています",
        });
    }
    const declarations = new Set(workspace.declarations.map((item) => item.id));
    const claims = new Set(workspace.requirements.map((item) => item.id));
    const evidence = new Set(workspace.evidence.map((item) => item.id));
    workspace.edges.forEach((edge, index) => {
      if (!declarations.has(edge.source) || !declarations.has(edge.target))
        ctx.addIssue({
          code: "custom",
          path: ["edges", index],
          message: "依存先の宣言がありません",
        });
    });
    workspace.analyses.forEach((analysis, index) => {
      if (
        !claims.has(analysis.claimId) ||
        !declarations.has(analysis.declarationId)
      )
        ctx.addIssue({
          code: "custom",
          path: ["analyses", index],
          message: "要件または定理がありません",
        });
      if (analysis.evidenceRefs.some((ref) => !evidence.has(ref)))
        ctx.addIssue({
          code: "custom",
          path: ["analyses", index, "evidenceRefs"],
          message: "参照する根拠がありません",
        });
    });
    workspace.declarations.forEach((declaration, index) => {
      if (
        declaration.evidenceRefs.some((ref) => !evidence.has(ref)) ||
        (["proved", "bounded", "empirical", "refuted"].includes(
          declaration.proofStatus,
        ) &&
          (!declaration.evidenceRefs.length ||
            declaration.proofSourceRevision === undefined ||
            declaration.proofInputRevision === undefined))
      )
        ctx.addIssue({
          code: "custom",
          path: ["declarations", index, "evidenceRefs"],
          message: "検証結果の表示には根拠と対象版が必要です",
        });
    });
    workspace.reviewDrafts.forEach((draft, index) => {
      if (draft.claimIds.some((claim) => !claims.has(claim)))
        ctx.addIssue({
          code: "custom",
          path: ["reviewDrafts", index],
          message: "レビュー対象の要件がありません",
        });
    });
  });

export type Workspace = z.infer<typeof workspaceSchema>;
export type Declaration = z.infer<typeof declarationSchema>;
export type Analysis = z.infer<typeof analysisSchema>;
export type Conversation = z.infer<typeof conversationSchema>;
export type ProofStatus = (typeof proofStatuses)[number];
export type Verdict = (typeof verdicts)[number];

export function analysisIsCurrent(
  workspace: Workspace,
  analysis: Analysis,
): boolean {
  return (
    analysis.status === "complete" &&
    analysis.inputRevision === workspace.inputRevision &&
    analysis.sourceRevision === workspace.sourceRevision
  );
}

/** Prefer an analysis for the current binding; otherwise return the latest retained result. */
export function analysisFor(
  workspace: Workspace,
  target: { claimId?: string; declarationId?: string },
): Analysis | undefined {
  if (target.claimId === undefined && target.declarationId === undefined)
    return undefined;
  const candidates = workspace.analyses.filter(
    (analysis) =>
      (target.claimId === undefined || analysis.claimId === target.claimId) &&
      (target.declarationId === undefined ||
        analysis.declarationId === target.declarationId),
  );
  for (let index = candidates.length - 1; index >= 0; index--)
    if (analysisIsCurrent(workspace, candidates[index]))
      return candidates[index];
  return candidates[candidates.length - 1];
}

/** Imports are displayed reports, never host authority or newly executed proof checks. */
export function importWorkspace(input: unknown): Workspace {
  const workspace = workspaceSchema.parse(input);
  return {
    ...workspace,
    origin: workspace.origin === "sample" ? "sample" : "imported",
    declarations: workspace.declarations.map((declaration) =>
      ["proved", "bounded", "empirical", "refuted"].includes(
        declaration.proofStatus,
      ) &&
      (declaration.proofSourceRevision !== workspace.sourceRevision ||
        declaration.proofInputRevision !== workspace.inputRevision)
        ? { ...declaration, proofStatus: "stale" }
        : declaration,
    ),
    analyses: workspace.analyses.map((analysis) =>
      analysisIsCurrent(workspace, analysis)
        ? analysis
        : { ...analysis, status: "stale" },
    ),
  };
}

/** Editing a draft cannot preserve evidence for its previous meaning. */
export function updateRequirement(
  workspace: Workspace,
  claimId: string,
  body: string,
): Workspace {
  if (!workspace.requirements.some((claim) => claim.id === claimId))
    throw new Error("要件がありません");
  if (
    workspace.requirements.find((claim) => claim.id === claimId)?.text === body
  )
    return workspace;
  return {
    ...workspace,
    inputRevision: workspace.inputRevision + 1,
    updatedAt: new Date().toISOString(),
    requirements: workspace.requirements.map((claim) =>
      claim.id === claimId ? { ...claim, text: body } : claim,
    ),
    declarations: workspace.declarations.map((declaration) => ({
      ...declaration,
      proofStatus:
        declaration.proofStatus === "unproved" ||
        declaration.proofStatus === "unknown" ||
        declaration.proofStatus === "unsupported"
          ? declaration.proofStatus
          : "stale",
    })),
    analyses: workspace.analyses.map((analysis) => ({
      ...analysis,
      status: "stale",
    })),
  };
}

export function createWorkspace(name: string): Workspace {
  return workspaceSchema.parse({
    schemaVersion: 1,
    id: crypto.randomUUID(),
    name,
    sourceRevision: "unversioned",
    inputRevision: 0,
    origin: "local",
    updatedAt: new Date().toISOString(),
    requirements: [],
    declarations: [],
    edges: [],
    analyses: [],
    evidence: [],
    conversations: [],
    reviewDrafts: [],
  });
}

export const proofLabels: Record<ProofStatus, string> = {
  proved: "証明済み",
  bounded: "範囲内で確認",
  empirical: "テスト済み",
  refuted: "反例あり",
  unproved: "未証明",
  unknown: "未確認",
  unsupported: "対象外",
  stale: "再検証が必要",
};
export const verdictLabels: Record<Verdict, string> = {
  full: "一致",
  partial: "一部一致",
  mismatch: "不一致",
  vacuous: "空虚な命題",
  missing: "定理なし",
  unformalizable: "形式化の対象外",
  unclear: "判定保留",
};
