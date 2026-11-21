import { DEMO_WORKSPACE_ID, demoSnapshot } from "../../fixtures/meden-demo";
import { Button, IconButton, Tab, Tabs, Typography } from "@mui/material";
import CloseRounded from "@mui/icons-material/CloseRounded";
import OpenInFullRounded from "@mui/icons-material/OpenInFullRounded";
import ArrowForwardRounded from "@mui/icons-material/ArrowForwardRounded";
import {
  analysisFor,
  type Declaration,
  type Workspace,
} from "@meden/workspace-schema";
import { ProofStatusChip } from "../../components/Status";
import { SourceView } from "../source/SourceView";
export function ProofInspector({
  workspace,
  selected,
  wide,
  reader,
  onReader,
  onClose,
  onSource,
  onExpand,
  onCenter,
  onClaim,
  onSelect,
  onNotify,
}: {
  workspace: Workspace;
  selected: Declaration;
  wide: boolean;
  reader: "statement" | "source";
  onReader: (value: "statement" | "source") => void;
  onClose: () => void;
  onSource: () => void;
  onExpand: () => void;
  onCenter: () => void;
  onClaim: (id: string) => void;
  onSelect: (id: string) => void;
  onNotify: (text: string) => void;
}) {
  const dependencies = workspace.edges.filter(
    (edge) => edge.source === selected.id,
  );
  return (
    <aside
      className={"proof-inspector " + (wide ? "docked" : "peek")}
      aria-label="Selected proof"
    >
      <header className="inspector-heading">
        <div>
          <Typography variant="h3">
            {selected.label || selected.name}
          </Typography>
          <code>{selected.name}</code>
        </div>
        <IconButton aria-label="Close proof details" onClick={onClose}>
          <CloseRounded fontSize="small" />
        </IconButton>
      </header>
      {wide && (
        <Tabs
          value={reader}
          onChange={(_, value: "statement" | "source") => onReader(value)}
          aria-label="Declaration view"
        >
          <Tab value="statement" label="Statement" />
          <Tab value="source" label="Lean source" />
        </Tabs>
      )}
      {wide && reader === "source" ? (
        <SourceView
          files={workspace.id === DEMO_WORKSPACE_ID ? demoSnapshot.files : {}}
          target={selected}
          showBack={false}
          onBack={() => onReader("statement")}
          onNotify={onNotify}
        />
      ) : (
        <div
          className="inspector-reading"
          tabIndex={wide ? 0 : undefined}
          aria-label={wide ? "Proof statement" : undefined}
        >
          <div className="inspector-proof-status">
            {selected.kind === "theorem" && (
              <ProofStatusChip status={selected.proofStatus} />
            )}
            <span>
              {selected.source?.file}:{selected.source?.line}
            </span>
          </div>
          <button className="proof-statement-link" onClick={onSource}>
            {selected.summary && <p>{selected.summary}</p>}
            {!selected.statement && (
              <p>The declaration body is not included in this data.</p>
            )}
            <span>
              Open Lean source <ArrowForwardRounded fontSize="small" />
            </span>
          </button>
          {wide && selected.statement && (
            <pre className="inspector-statement" tabIndex={0}>
              {selected.statement}
            </pre>
          )}
          <div className="proof-detail-actions">
            {!wide && (
              <Button
                size="small"
                startIcon={<OpenInFullRounded />}
                onClick={onExpand}
              >
                Expand reader
              </Button>
            )}
            <Button size="small" onClick={onCenter}>
              Center here
            </Button>
            {analysisFor(workspace, { declarationId: selected.id }) && (
              <Button
                size="small"
                onClick={() =>
                  onClaim(
                    analysisFor(workspace, { declarationId: selected.id })!
                      .claimId,
                  )
                }
              >
                View requirement
              </Button>
            )}
          </div>
          {wide && (
            <>
              <Typography variant="h3" className="inspector-section-heading">
                References <span>{dependencies.length}</span>
              </Typography>
              <div className="inspector-dependencies">
                {dependencies.map((edge) => (
                  <button
                    key={edge.target + edge.site}
                    onClick={() => onSelect(edge.target)}
                  >
                    <span>
                      {workspace.declarations.find(
                        (item) => item.id === edge.target,
                      )?.label || edge.target}
                    </span>
                    <small>
                      {edge.site === "type" ? "Statement" : "Proof"}
                    </small>
                  </button>
                ))}
              </div>
              {selected.axioms.length > 0 && (
                <>
                  <Typography
                    variant="h3"
                    className="inspector-section-heading"
                  >
                    Axioms used
                  </Typography>
                  <pre className="inspector-axioms">
                    {selected.axioms.join("\n")}
                  </pre>
                </>
              )}
            </>
          )}
        </div>
      )}
    </aside>
  );
}
