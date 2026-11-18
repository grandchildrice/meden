import { analysisFor, type Workspace } from "@meden/workspace-schema";

export function proofObligations(workspace: Workspace) {
  return workspace.requirements.map((claim) => {
    const analysis = analysisFor(workspace, { claimId: claim.id });
    const declaration = workspace.declarations.find(
      (item) => item.id === analysis?.declarationId,
    );
    return {
      claim,
      declaration,
      status: declaration?.proofStatus ?? ("unproved" as const),
    };
  });
}
