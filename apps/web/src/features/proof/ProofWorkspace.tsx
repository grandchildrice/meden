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
            <span>証明の状態</span>
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
          <Tooltip title="グラフを広げる">
            <IconButton
              aria-label="グラフを広げる"
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
            意味解析
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
      <Tooltip title="保存されたLeanソース内の参照を表示しています。Leanによる完全な依存抽出ではなく、標準ライブラリは省略しています。">
        <div className="proof-scope-note">
          {workspace.id === "meden-demo-vending"
            ? "保存ソースの参照 · 純粋なstepが検証対象 · CLI・I/Oは対象外"
            : "読み込んだ宣言の参照 · 検証範囲は元の記録を参照"}
        </div>
      </Tooltip>
    </div>
  );
  return (
    <section className="proof-workspace" aria-label="証明ワークスペース">
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
                証明の依存関係
              </Typography>
              <span>{workspace.name}</span>
            </div>
            <Button onClick={() => setExpanded(false)}>会話に戻る</Button>
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
