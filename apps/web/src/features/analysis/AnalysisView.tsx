import { useState } from "react";
import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Alert,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  MenuItem,
  Select,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import EditOutlined from "@mui/icons-material/EditOutlined";
import ExpandMoreRounded from "@mui/icons-material/ExpandMoreRounded";
import CompareArrowsRounded from "@mui/icons-material/CompareArrowsRounded";
import { diffChars } from "diff";
import {
  analysisMatchesRevision,
  analysisFor,
  updateRequirement,
  type Workspace,
} from "@meden/workspace-schema";
import { AnalysisStatusChip, ProofStatusChip } from "../../components/Status";
import { EmptyState } from "../../components/EmptyState";
import { SemanticMethodInfo } from "../proof/SemanticMethodInfo";

export function AnalysisView({
  workspace,
  claimId,
  onSelectClaim,
  onChange,
  onGraph,
  onReview,
  embedded = false,
}: {
  embedded?: boolean;
  workspace: Workspace;
  claimId?: string;
  onSelectClaim: (id: string) => void;
  onChange: (workspace: Workspace) => Promise<boolean>;
  onGraph: (id: string) => void;
  onReview: (id: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [body, setBody] = useState("");
  const claim =
    workspace.requirements.find((item) => item.id === claimId) ||
    workspace.requirements[0];
  const analysis = analysisFor(workspace, { claimId: claim?.id });
  const declaration = workspace.declarations.find(
    (item) => item.id === analysis?.declarationId,
  );
  const current = analysis
    ? analysisMatchesRevision(workspace, analysis)
    : false;
  const evidence = workspace.evidence.filter((item) =>
    analysis?.evidenceRefs.includes(item.id),
  );
  if (!claim)
    return (
      <section className="analysis-page">
        <header className="page-heading">
          <Typography variant={embedded ? "h2" : "h1"}>
            Meaning comparison
          </Typography>
        </header>
        <EmptyState
          icon={<CompareArrowsRounded />}
          title="No requirements"
          detail="Load a workspace containing requirements and analyses."
        />
      </section>
    );
  const parts = analysis ? diffChars(claim.text, analysis.backTranslation) : [];
  return (
    <section className="analysis-page">
      <div className="comparison-panel">
        <header className="semantic-title">
          <Typography variant={embedded ? "h2" : "h1"}>
            Meaning comparison
          </Typography>
          <SemanticMethodInfo />
        </header>
        <header className="comparison-heading">
          <Select
            size="small"
            fullWidth
            value={claim.id}
            onChange={(event) => onSelectClaim(event.target.value)}
            inputProps={{ "aria-label": "Requirement to compare" }}
          >
            {workspace.requirements.map((requirement) => (
              <MenuItem key={requirement.id} value={requirement.id}>
                {requirement.title}
              </MenuItem>
            ))}
          </Select>
          <AnalysisStatusChip analysis={analysis} current={current} />
        </header>
        {analysis && !current && (
          <Alert severity="info">
            Requirements or source revisions changed. Showing a previous
            analysis.
          </Alert>
        )}
        <div className="comparison-grid">
          <section className="comparison-card">
            <div className="comparison-card-title">
              <Typography variant="h3">Requirement</Typography>
              <Tooltip title="Edit requirement">
                <IconButton
                  aria-label="Edit requirement"
                  onClick={() => {
                    setBody(claim.text);
                    setEditing(true);
                  }}
                >
                  <EditOutlined fontSize="small" />
                </IconButton>
              </Tooltip>
            </div>
            <div className="compared-text">
              {analysis
                ? parts
                    .filter((part) => !part.added)
                    .map((part, index) =>
                      part.removed ? (
                        <mark className="expected-diff" key={index}>
                          {part.value}
                        </mark>
                      ) : (
                        <span key={index}>{part.value}</span>
                      ),
                    )
                : claim.text}
            </div>
            <Typography variant="caption" color="text.secondary">
              {claim.source?.file}
              {claim.source?.line ? ":" + claim.source.line : ""}
            </Typography>
          </section>
          <section className="comparison-card">
            <div className="comparison-card-title">
              <Typography variant="h3">
                {workspace.origin === "replay" && analysis?.status === "pending"
                  ? "Theorem comparison (example)"
                  : "Theorem back-translation"}
              </Typography>
              {declaration && (
                <ProofStatusChip status={declaration.proofStatus} />
              )}
            </div>
            <div className="compared-text">
              {analysis ? (
                parts
                  .filter((part) => !part.removed)
                  .map((part, index) =>
                    part.added ? (
                      <mark className="actual-diff" key={index}>
                        {part.value}
                      </mark>
                    ) : (
                      <span key={index}>{part.value}</span>
                    ),
                  )
              ) : (
                <Typography color="text.secondary">
                  No back-translation yet.
                </Typography>
              )}
            </div>
            {declaration && (
              <Button
                className="source-button"
                size="small"
                onClick={() => onGraph(declaration.id)}
              >
                {declaration.name}
              </Button>
            )}
          </section>
        </div>
        {analysis ? (
          <>
            <div className="reasoning">
              <Typography variant="h3">Assessment reasoning</Typography>
              <Typography>{analysis.reasoning}</Typography>
            </div>
            {evidence.length > 0 && (
              <div className="analysis-evidence">
                <Typography variant="h3">Evidence</Typography>
                {evidence.map((item) => (
                  <div className="evidence-item" key={item.id}>
                    <div>
                      <Chip
                        size="small"
                        variant="outlined"
                        label={
                          item.kind === "counterexample"
                            ? "Boundary case"
                            : item.kind === "declaration"
                              ? "Declaration"
                              : item.kind === "report"
                                ? "Report"
                                : "Requirement"
                        }
                      />
                      <Typography component="span" variant="body2">
                        {item.label}
                      </Typography>
                    </div>
                    <pre>{item.content}</pre>
                  </div>
                ))}
              </div>
            )}
            <Accordion className="analysis-metadata" disableGutters>
              <AccordionSummary expandIcon={<ExpandMoreRounded />}>
                <Typography>Analysis information</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <dl className="metadata">
                  <dt>Source revision</dt>
                  <dd>{analysis.sourceRevision}</dd>
                  <dt>Requirement revision</dt>
                  <dd>{analysis.inputRevision}</dd>
                  <dt>Model</dt>
                  <dd>{analysis.modelVersions.join(", ") || "Not recorded"}</dd>
                  <dt>Assumptions and unchecked scope</dt>
                  <dd>
                    {analysis.assumptions.length
                      ? analysis.assumptions.map((item) => (
                          <div key={item}>{item}</div>
                        ))
                      : "Not recorded"}
                  </dd>
                </dl>
              </AccordionDetails>
            </Accordion>
            <div className="comparison-footer">
              <Typography variant="caption" color="text.secondary">
                Source revision {workspace.sourceRevision}
              </Typography>
              <Button variant="outlined" onClick={() => onReview(claim.id)}>
                Create review request
              </Button>
            </div>
          </>
        ) : (
          <EmptyState
            title="No analyses"
            detail="Connect an agent to run an analysis, or import existing results."
          />
        )}
      </div>
      <Dialog open={editing} onClose={() => setEditing(false)}>
        <DialogTitle>Edit requirement</DialogTitle>
        <DialogContent>
          <TextField
            fullWidth
            multiline
            minRows={5}
            label="Requirement"
            value={body}
            onChange={(event) => setBody(event.target.value)}
          />
          <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
            Edits require rechecking previous proof and analysis results. They
            do not change the runtime's accepted specification.
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditing(false)}>Cancel</Button>
          <Button
            variant="contained"
            disabled={!body.trim() || body.length > 100_000}
            onClick={async () => {
              if (await onChange(updateRequirement(workspace, claim.id, body)))
                setEditing(false);
            }}
          >
            Save
          </Button>
        </DialogActions>
      </Dialog>
    </section>
  );
}
