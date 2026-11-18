import { lazy, Suspense, useState } from "react";
import {
  Button,
  Chip,
  CircularProgress,
  IconButton,
  Tooltip,
  Typography,
} from "@mui/material";
import CloseRounded from "@mui/icons-material/CloseRounded";
import ArrowForwardRounded from "@mui/icons-material/ArrowForwardRounded";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import AutoAwesomeOutlined from "@mui/icons-material/AutoAwesomeOutlined";
import {
  analysisFor,
  type Declaration,
  type Workspace,
} from "@meden/workspace-schema";
import { ProofStatusChip } from "../../components/Status";
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
  const [claimId, setClaimId] = useState<string>();
  const [focusId, setFocusId] = useState<string>();
  const selected = workspace.declarations.find(
    (item) => item.id === selectedId,
  );
  const proved = workspace.requirements.filter(
    (claim) =>
      workspace.declarations.find(
        (item) =>
          item.id ===
          analysisFor(workspace, { claimId: claim.id })?.declarationId,
      )?.proofStatus === "proved",
  ).length;
  const inspect = (id: string) => {
    setClaimId(id);
    setMode("compare");
  };
  return (
    <section className="proof-workspace" aria-label="証明ワークスペース">
      <Suspense
        fallback={
          <div className="loading">
            <CircularProgress size={24} />
          </div>
        }
      >
        <div hidden={mode !== "graph"} className="proof-graph-view">
          <header className="proof-workspace-heading">
            <details className="proof-obligations">
              <summary>
                <span>証明の状態</span>
                <Chip
                  size="small"
                  className="status-chip positive"
                  label={proved + " / " + workspace.requirements.length}
                />
              </summary>
              <div className="proof-list">
                {workspace.requirements.map((claim) => {
                  const declaration = workspace.declarations.find(
                    (item) =>
                      item.id ===
                      analysisFor(workspace, { claimId: claim.id })
                        ?.declarationId,
                  );
                  return (
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
                      <ProofStatusChip
                        status={declaration?.proofStatus || "unproved"}
                      />
                    </button>
                  );
                })}
              </div>
            </details>
            <Button
              variant="outlined"
              startIcon={<AutoAwesomeOutlined />}
              onClick={() => setMode("run")}
            >
              意味解析
            </Button>
          </header>
          <GraphView
            workspace={workspace}
            focusId={focusId}
            selectedId={selectedId}
            onSelect={(declaration) => onSelect(declaration.id)}
          />
          {selected && (
            <aside className="proof-detail" aria-label="選択した証明">
              <div className="proof-detail-heading">
                <div>
                  <Typography variant="h3">
                    {selected.label || selected.name}
                  </Typography>
                  <code>{selected.name}</code>
                </div>
                <IconButton
                  aria-label="証明の詳細を閉じる"
                  onClick={() => onSelect(undefined)}
                >
                  <CloseRounded fontSize="small" />
                </IconButton>
              </div>
              {selected.kind === "theorem" && (
                <ProofStatusChip status={selected.proofStatus} />
              )}
              <button
                className="proof-statement-link"
                onClick={() => onSource(selected)}
              >
                {selected.summary && <p>{selected.summary}</p>}
                <pre>
                  {selected.statement ||
                    "宣言の本文はデータに含まれていません。"}
                </pre>
                <span>
                  {selected.source?.file}:{selected.source?.line} ·
                  Leanソースを開く <ArrowForwardRounded fontSize="small" />
                </span>
              </button>
              <div className="proof-detail-actions">
                <Button size="small" onClick={() => setFocusId(selected.id)}>
                  中心にする
                </Button>
                {analysisFor(workspace, { declarationId: selected.id }) && (
                  <Button
                    size="small"
                    onClick={() =>
                      inspect(
                        analysisFor(workspace, { declarationId: selected.id })!
                          .claimId,
                      )
                    }
                  >
                    要件を確認
                  </Button>
                )}
              </div>
              <div className="proof-direct-dependencies">
                {workspace.edges
                  .filter((edge) => edge.source === selected.id)
                  .map((edge) => (
                    <Button
                      size="small"
                      key={edge.target + edge.site}
                      onClick={() => onSelect(edge.target)}
                    >
                      {workspace.declarations.find(
                        (item) => item.id === edge.target,
                      )?.label || edge.target}
                    </Button>
                  ))}
              </div>
            </aside>
          )}
          <Tooltip title="保存されたLeanソース内の参照を表示しています。Leanによる完全な依存抽出ではなく、標準ライブラリは省略しています。">
            <div className="proof-scope-note">
              {workspace.id === "meden-demo-vending"
                ? "保存ソースの参照 · 純粋なstepが検証対象 · CLI・I/Oは対象外"
                : "読み込んだ宣言の参照 · 検証範囲は元の記録を参照"}
            </div>
          </Tooltip>
        </div>
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
              グラフに戻る
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
