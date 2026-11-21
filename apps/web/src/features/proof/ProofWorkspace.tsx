import { lazy, Suspense, useState } from "react";
import {
  Button,
  Chip,
  CircularProgress,
  Dialog,
  IconButton,
  Tooltip,
  Typography,
} from "@mui/material";
import OpenInFullRounded from "@mui/icons-material/OpenInFullRounded";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import AutoAwesomeOutlined from "@mui/icons-material/AutoAwesomeOutlined";
import type { Declaration, Workspace } from "@meden/workspace-schema";
import { ProofInspector } from "./ProofInspector";
import { proofObligations } from "./proof-obligations";
import { ProofStatusChip } from "../../components/Status";
import { SemanticMethodInfo } from "./SemanticMethodInfo";
import { SemanticRun } from "./SemanticRun";
const GraphView = lazy(() =>
  import("../graph/GraphView").then((module) => ({
    default: module.GraphView,
  })),
);
const AnalysisView = lazy(() =>
  import("../analysis/AnalysisView").then((module) => ({
    default: module.AnalysisView,
  })),
);
const ReviewView = lazy(() =>
  import("../review/ReviewView").then((module) => ({
    default: module.ReviewView,
  })),
);
export function ProofWorkspace({
  workspace,
  selectedId,
  onSelect,
  onSource,
  onChange,
  onNotify,
}: {
  workspace: Workspace;
  selectedId?: string;
  onSelect: (id: string | undefined) => void;
  onSource: (declaration: Declaration) => void;
  onChange: (workspace: Workspace) => Promise<boolean>;
  onNotify: (text: string) => void;
}) {
  const [mode, setMode] = useState<"graph" | "run" | "compare" | "review">(
    "graph",
  );
  const [expanded, setExpanded] = useState(false);
  const [reader, setReader] = useState<"statement" | "source">("statement");
  const select = (id: string | undefined) => {
    onSelect(id);
    setReader("statement");
  };
  const [claimId, setClaimId] = useState<string>();
  const [focusId, setFocusId] = useState<string>();
  const selected = workspace.declarations.find(
    (item) => item.id === selectedId,
  );
  const obligations = proofObligations(workspace);
  const proved = obligations.filter(({ status }) => status === "proved").length;
  const inspect = (id: string) => {
    setClaimId(id);
    setMode("compare");
  };
  const board = (
    <div hidden={mode !== "graph"} className="proof-graph-view">
      <header className="proof-workspace-heading">
        <details className="proof-obligations">
          <summary>
            <span>Proof status</span>
            <Chip
              size="small"
              className="status-chip positive"
              label={proved + " / " + obligations.length}
            />
          </summary>
          <div className="proof-list">
            {obligations.map(({ claim, declaration, status }) => (
              <button
                key={claim.id}
                onClick={(event) => {
                  if (declaration) onSelect(declaration.id);
                  event.currentTarget
                    .closest("details")
                    ?.removeAttribute("open");
                }}
              >
                <span>{claim.title}</span>
                <ProofStatusChip status={status} />
              </button>
            ))}
          </div>
        </details>
        <div className="meaning-actions">
          <Tooltip title="Expand graph">
            <IconButton
              aria-label="Expand graph"
              onClick={() => setExpanded(true)}
            >
              <OpenInFullRounded fontSize="small" />
            </IconButton>
          </Tooltip>
          <Button
            variant="outlined"
            startIcon={<AutoAwesomeOutlined />}
            onClick={() => {
              setExpanded(false);
              setMode("run");
            }}
          >
            Meaning analysis
          </Button>
          <SemanticMethodInfo />
        </div>
      </header>
      <div
        className={"proof-explorer-body " + (selected ? "has-selection" : "")}
      >
        <GraphView
          workspace={workspace}
          focusId={focusId}
          selectedId={selectedId}
          onSelect={(declaration) => select(declaration.id)}
        />
        {selected && (
          <ProofInspector
            workspace={workspace}
            selected={selected}
            wide={expanded}
            reader={reader}
            onReader={setReader}
            onClose={() => select(undefined)}
            onSource={() =>
              expanded ? setReader("source") : onSource(selected)
            }
            onExpand={() => setExpanded(true)}
            onCenter={() => setFocusId(selected.id)}
            onClaim={(id) => {
              setExpanded(false);
              inspect(id);
            }}
            onSelect={select}
            onNotify={onNotify}
          />
        )}
      </div>
      <Tooltip title="References in saved Lean sources. This is not complete compiler dependency extraction; standard libraries are omitted.">
        <div className="proof-scope-note">
          {workspace.id === "meden-demo-vending"
            ? "Saved source references · Pure step verified · CLI and I/O excluded"
            : "Imported declaration references · See original records for verification scope"}
        </div>
      </Tooltip>
    </div>
  );
  return (
    <section className="proof-workspace" aria-label="Proof workspace">
      <Suspense
        fallback={
          <div className="loading">
            <CircularProgress size={24} />
          </div>
        }
      >
        {!expanded && board}
        <Dialog
          fullScreen
          open={expanded}
          onClose={() => setExpanded(false)}
          slotProps={{ paper: { className: "expanded-proof-dialog" } }}
          aria-labelledby="expanded-proof-title"
        >
          <header className="expanded-proof-header">
            <div>
              <Typography variant="h2" id="expanded-proof-title">
                Proof dependencies
              </Typography>
              <span>{workspace.name}</span>
            </div>
            <Button onClick={() => setExpanded(false)}>
              Back to conversation
            </Button>
          </header>
          {board}
        </Dialog>
        {mode === "run" && (
          <SemanticRun
            workspace={workspace}
            onBack={() => setMode("graph")}
            onInspect={inspect}
          />
        )}
        {(mode === "compare" || mode === "review") && (
          <div className="proof-nested-view">
            <Button
              size="small"
              startIcon={<ArrowBackRounded />}
              onClick={() => setMode("graph")}
            >
              Back to graph
            </Button>
            {mode === "compare" ? (
              <AnalysisView
                embedded
                workspace={workspace}
                claimId={claimId}
                onSelectClaim={setClaimId}
                onChange={onChange}
                onGraph={(id) => {
                  onSelect(id);
                  setFocusId(id);
                  setMode("graph");
                }}
                onReview={(id) => {
                  setClaimId(id);
                  setMode("review");
                }}
              />
            ) : (
              <ReviewView
                embedded
                workspace={workspace}
                claimId={claimId}
                onChange={onChange}
                onNotify={onNotify}
              />
            )}
          </div>
        )}
      </Suspense>
    </section>
  );
}
