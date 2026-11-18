import ArrowForwardRounded from "@mui/icons-material/ArrowForwardRounded";
import InsertDriveFileOutlined from "@mui/icons-material/InsertDriveFileOutlined";
import type { Workspace } from "@meden/workspace-schema";
import leanLogo from "../../assets/technology-logos/lean.png";
import rustLogo from "../../assets/technology-logos/rust.png";
import { ProofStatusChip } from "../../components/Status";
import { DEMO_WORKSPACE_ID } from "../../fixtures/meden-demo";
import { proofObligations } from "../proof/proof-obligations";

export function ArtifactCard({
  workspace,
  onOpen,
}: {
  workspace: Workspace;
  onOpen: () => void;
}) {
  const obligations = proofObligations(workspace);
  return (
    <section className="artifact-card" aria-label={workspace.name + "の成果物"}>
      <header className="artifact-card-header">
        <button
          type="button"
          className="artifact-card-open"
          aria-label={workspace.name + "のプレビューを開く"}
          onClick={onOpen}
        >
          <InsertDriveFileOutlined fontSize="small" />
          <strong>{workspace.name}</strong>
          <ArrowForwardRounded fontSize="small" />
        </button>
        {workspace.id === DEMO_WORKSPACE_ID && (
          <div className="artifact-card-logos">
            <img src={rustLogo} alt="Rust" width={22} height={22} />
            <img src={leanLogo} alt="Lean" width={22} height={22} />
          </div>
        )}
      </header>
      {obligations.length > 0 && (
        <ul
          className="artifact-card-proofs"
          role="list"
          aria-label="定理と証明ステータス"
        >
          {obligations.map(({ claim, declaration, status }) => (
            <li key={claim.id}>
              <div>
                <span>{claim.title}</span>
                {declaration && <code>{declaration.name}</code>}
              </div>
              <ProofStatusChip status={status} />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
