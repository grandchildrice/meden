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
          <Typography variant={embedded ? "h2" : "h1"}>意味の比較</Typography>
        </header>
        <EmptyState
          icon={<CompareArrowsRounded />}
          title="要件がありません"
          detail="要件と解析結果を含むワークスペースを読み込んでください。"
        />
      </section>
    );
  const parts = analysis ? diffChars(claim.text, analysis.backTranslation) : [];
  return (
    <section className="analysis-page">
      <div className="comparison-panel">
        <header className="semantic-title">
          <Typography variant={embedded ? "h2" : "h1"}>意味の比較</Typography>
          <SemanticMethodInfo />
        </header>
        <header className="comparison-heading">
          <Select
            size="small"
            fullWidth
            value={claim.id}
            onChange={(event) => onSelectClaim(event.target.value)}
            inputProps={{ "aria-label": "比較する要件" }}
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
            要件または対象版が変更されています。以前の解析結果を表示しています。
          </Alert>
        )}
        <div className="comparison-grid">
          <section className="comparison-card">
            <div className="comparison-card-title">
              <Typography variant="h3">要件</Typography>
              <Tooltip title="要件を編集">
                <IconButton
                  aria-label="要件を編集"
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
                  ? "定理の比較文（表示例）"
                  : "定理の逆翻訳"}
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
                  逆翻訳はまだありません。
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
              <Typography variant="h3">判定の理由</Typography>
              <Typography>{analysis.reasoning}</Typography>
            </div>
            {evidence.length > 0 && (
              <div className="analysis-evidence">
                <Typography variant="h3">根拠</Typography>
                {evidence.map((item) => (
                  <div className="evidence-item" key={item.id}>
                    <div>
                      <Chip
                        size="small"
                        variant="outlined"
                        label={
                          item.kind === "counterexample"
                            ? "境界ケース"
                            : item.kind === "declaration"
                              ? "宣言"
                              : item.kind === "report"
                                ? "レポート"
                                : "要件"
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
                <Typography>解析の情報</Typography>
              </AccordionSummary>
              <AccordionDetails>
                <dl className="metadata">
                  <dt>対象版</dt>
                  <dd>{analysis.sourceRevision}</dd>
                  <dt>要件の版</dt>
                  <dd>{analysis.inputRevision}</dd>
                  <dt>モデル</dt>
                  <dd>{analysis.modelVersions.join(", ") || "記録なし"}</dd>
                  <dt>前提・未確認範囲</dt>
                  <dd>
                    {analysis.assumptions.length
                      ? analysis.assumptions.map((item) => (
                          <div key={item}>{item}</div>
                        ))
                      : "記録なし"}
                  </dd>
                </dl>
              </AccordionDetails>
            </Accordion>
            <div className="comparison-footer">
              <Typography variant="caption" color="text.secondary">
                対象版 {workspace.sourceRevision}
              </Typography>
              <Button variant="outlined" onClick={() => onReview(claim.id)}>
                レビュー依頼を作成
              </Button>
            </div>
          </>
        ) : (
          <EmptyState
            title="解析結果がありません"
            detail="エージェントへの接続後に解析するか、解析済みの結果を読み込んでください。"
          />
        )}
      </div>
      <Dialog open={editing} onClose={() => setEditing(false)}>
        <DialogTitle>要件を編集</DialogTitle>
        <DialogContent>
          <TextField
            fullWidth
            multiline
            minRows={5}
            label="要件"
            value={body}
            onChange={(event) => setBody(event.target.value)}
          />
          <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
            変更すると、以前の証明と解析は再確認が必要になります。実行系の承認済み仕様は変更しません。
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setEditing(false)}>キャンセル</Button>
          <Button
            variant="contained"
            disabled={!body.trim() || body.length > 100_000}
            onClick={async () => {
              if (await onChange(updateRequirement(workspace, claim.id, body)))
                setEditing(false);
            }}
          >
            保存
          </Button>
        </DialogActions>
      </Dialog>
    </section>
  );
}
