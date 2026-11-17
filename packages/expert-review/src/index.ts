import type { Workspace } from "@meden/workspace-schema";
import { dependencyScope } from "@meden/atlas-adapter";

export function createReviewPackage(
  workspace: Workspace,
  draft: Workspace["reviewDrafts"][number],
) {
  if (
    draft.inputRevision !== workspace.inputRevision ||
    draft.sourceRevision !== workspace.sourceRevision
  )
    throw new Error(
      "要件または対象版が変更されています。レビュー対象を更新してください。",
    );
  const claims = new Set(draft.claimIds);
  if (
    !claims.size ||
    draft.claimIds.some(
      (id) => !workspace.requirements.some((claim) => claim.id === id),
    )
  )
    throw new Error("レビュー対象の要件を選択してください。");
  const analyses = workspace.analyses.filter((analysis) =>
    claims.has(analysis.claimId),
  );
  const nodes = new Set<string>();
  for (const analysis of analyses)
    for (const id of dependencyScope(
      workspace,
      analysis.declarationId,
      "meaning",
    ))
      nodes.add(id);
  const evidence = new Set(
    analyses.flatMap((analysis) => analysis.evidenceRefs),
  );
  for (const declaration of workspace.declarations)
    if (nodes.has(declaration.id))
      for (const ref of declaration.evidenceRefs) evidence.add(ref);
  return {
    schemaVersion: 1 as const,
    kind: "review-request" as const,
    encryption: "none" as const,
    sourceRevision: draft.sourceRevision,
    inputRevision: draft.inputRevision,
    origin: workspace.origin,
    request: draft,
    requirements: workspace.requirements.filter((claim) =>
      claims.has(claim.id),
    ),
    declarations: workspace.declarations.filter((declaration) =>
      nodes.has(declaration.id),
    ),
    edges: workspace.edges.filter(
      (edge) => nodes.has(edge.source) && nodes.has(edge.target),
    ),
    analyses,
    evidence: workspace.evidence.filter((item) => evidence.has(item.id)),
  };
}
