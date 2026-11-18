import { lazy, Suspense, useState } from "react";
import {
  Button,
  Chip,
  CircularProgress,
  IconButton,
  MenuItem,
  Select,
  Tab,
  Tabs,
  Typography,
} from "@mui/material";
import CloseRounded from "@mui/icons-material/CloseRounded";
import AccountTreeOutlined from "@mui/icons-material/AccountTreeOutlined";
import ArrowForwardRounded from "@mui/icons-material/ArrowForwardRounded";
import type { Workspace } from "@meden/workspace-schema";
import { analysisFor } from "@meden/workspace-schema";
import { ProofStatusChip } from "../../components/Status";
import { demoSnapshot, DEMO_WORKSPACE_ID } from "../../fixtures/meden-demo";
import { VendingPreview } from "../demo/VendingPreview";
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
export type ArtifactTab =
  "preview" | "proof" | "graph" | "analysis" | "code" | "review";
interface Props {
  workspace: Workspace;
  tab: ArtifactTab;
  onTab: (tab: ArtifactTab) => void;
  onClose: () => void;
  onChange: (workspace: Workspace) => Promise<boolean>;
  onNotify: (text: string) => void;
  running: boolean;
}
export function ArtifactPanel({
  workspace,
  tab,
  onTab,
  onClose,
  onChange,
  onNotify,
  running,
}: Props) {
  const [claimId, setClaimId] = useState<string>();
  const [nodeId, setNodeId] = useState<string>();
  const [file, setFile] = useState("kernel.rs");
  const graph = (id: string) => {
    setNodeId(id);
    onTab("graph");
  };
  return (
    <section className="artifact-panel" aria-label="実装と検証">
      <header className="artifact-panel-heading">
        <div>
          <Typography variant="h2">{workspace.name}</Typography>
          <Typography variant="caption" color="text.secondary">
            {workspace.id === DEMO_WORKSPACE_ID
              ? "meden-demo"
              : "ワークスペース"}
          </Typography>
        </div>
        <IconButton aria-label="パネルを閉じる" onClick={onClose}>
          <CloseRounded fontSize="small" />
        </IconButton>
      </header>
      <Tabs
        value={tab}
        onChange={(_, value: ArtifactTab) => onTab(value)}
        variant="scrollable"
        scrollButtons="auto"
        aria-label="実装と検証のタブ"
      >
        <Tab
          value="preview"
          label="プレビュー"
          id="artifact-tab-preview"
          aria-controls="artifact-content-preview"
        />
        <Tab
          value="proof"
          label="証明"
          id="artifact-tab-proof"
          aria-controls="artifact-content-proof"
        />
        <Tab
          value="graph"
          label="依存関係"
          id="artifact-tab-graph"
          aria-controls="artifact-content-graph"
        />
        <Tab
          value="analysis"
          label="意味の比較"
          id="artifact-tab-analysis"
          aria-controls="artifact-content-analysis"
        />
        <Tab
          value="code"
          label="コード"
          id="artifact-tab-code"
          aria-controls="artifact-content-code"
        />
        {tab === "review" && (
          <Tab
            value="review"
            label="レビュー依頼"
            id="artifact-tab-review"
            aria-controls="artifact-content-review"
          />
        )}
      </Tabs>
      <div
        role="tabpanel"
        id={"artifact-content-" + tab}
        aria-labelledby={"artifact-tab-" + tab}
        className="artifact-content"
      >
        <Suspense
          fallback={
            <div className="loading">
              <CircularProgress size={24} />
            </div>
          }
        >
          {tab === "preview" &&
            (workspace.id === DEMO_WORKSPACE_ID ? (
              <VendingPreview />
            ) : (
              <div className="panel-empty">
                このワークスペースにはプレビューがありません。
              </div>
            ))}
          {tab === "proof" && (
            <div className="proof-overview">
              <div className="proof-overview-title">
                <div>
                  <Typography variant="h2">証明の状態</Typography>
                  <Typography color="text.secondary" variant="body2">
                    {workspace.id === DEMO_WORKSPACE_ID
                      ? "kernel.rsの純粋なstep"
                      : "読み込んだ検証記録"}
                  </Typography>
                </div>
                <Chip
                  label={
                    workspace.requirements.filter(
                      (claim) =>
                        workspace.declarations.find(
                          (node) =>
                            node.id ===
                            analysisFor(workspace, { claimId: claim.id })
                              ?.declarationId,
                        )?.proofStatus === "proved",
                    ).length +
                    " / " +
                    workspace.requirements.length
                  }
                  className="status-chip positive"
                />
              </div>
              {running && (
                <Typography
                  role="status"
                  variant="caption"
                  color="text.secondary"
                >
                  保存された記録を開いています
                </Typography>
              )}
              <div className="proof-list">
                {workspace.requirements.map((claim) => {
                  const analysis = analysisFor(workspace, {
                    claimId: claim.id,
                  });
                  const declaration = workspace.declarations.find(
                    (node) => node.id === analysis?.declarationId,
                  );
                  return (
                    <div className="proof-item" key={claim.id}>
                      <div>
                        <Typography>{claim.title}</Typography>
                        <code>{claim.id}</code>
                      </div>
                      <ProofStatusChip
                        status={declaration?.proofStatus || "unproved"}
                      />
                      {declaration && (
                        <IconButton
                          aria-label={claim.title + "の依存関係を表示"}
                          onClick={() => graph(declaration.id)}
                        >
                          <AccountTreeOutlined fontSize="small" />
                        </IconButton>
                      )}
                    </div>
                  );
                })}
              </div>
              <div className="proof-scope">
                <Typography variant="h3">検証範囲</Typography>
                <Typography variant="body2" color="text.secondary">
                  {workspace.id === DEMO_WORKSPACE_ID
                    ? "投入・購入・返金の状態遷移。CLIの入力解析、表示、I/Oは形式検証の対象外です。"
                    : "各宣言の記録と対象リビジョンを確認してください。"}
                </Typography>
                <details>
                  <summary>記録の情報</summary>
                  <Typography component="div" variant="caption">
                    保存された検証結果を表示しています。GUIから新しい検証は実行していません。
                  </Typography>
                  <code>{workspace.sourceRevision}</code>
                </details>
              </div>
              <Button
                endIcon={<ArrowForwardRounded />}
                onClick={() => onTab("graph")}
              >
                依存関係を見る
              </Button>
            </div>
          )}
          {tab === "graph" && (
            <GraphView
              embedded
              workspace={workspace}
              initialId={nodeId}
              onClaim={(id) => {
                setClaimId(id);
                onTab("analysis");
              }}
            />
          )}
          {tab === "analysis" && (
            <AnalysisView
              embedded
              workspace={workspace}
              claimId={claimId}
              onSelectClaim={setClaimId}
              onChange={onChange}
              onGraph={graph}
              onReview={(id) => {
                setClaimId(id);
                onTab("review");
              }}
            />
          )}
          {tab === "review" && (
            <ReviewView
              embedded
              workspace={workspace}
              claimId={claimId}
              onChange={onChange}
              onNotify={onNotify}
            />
          )}
          {tab === "code" &&
            (workspace.id !== DEMO_WORKSPACE_ID ? (
              <div className="panel-empty">
                このワークスペースにはソースファイルがありません。
              </div>
            ) : (
              <div className="source-view">
                <div className="source-toolbar">
                  <Select
                    size="small"
                    value={file}
                    onChange={(event) => setFile(event.target.value)}
                    inputProps={{ "aria-label": "表示するファイル" }}
                  >
                    {Object.keys(demoSnapshot.files).map((name) => (
                      <MenuItem key={name} value={name}>
                        {name}
                      </MenuItem>
                    ))}
                  </Select>
                  <Button
                    size="small"
                    onClick={() =>
                      navigator.clipboard
                        .writeText(
                          demoSnapshot.files[
                            file as keyof typeof demoSnapshot.files
                          ],
                        )
                        .then(() => onNotify("コピーしました"))
                        .catch(() => onNotify("コピーできませんでした"))
                    }
                  >
                    コピー
                  </Button>
                </div>
                <pre className="source-code">
                  {demoSnapshot.files[file as keyof typeof demoSnapshot.files]}
                </pre>
              </div>
            ))}
        </Suspense>
      </div>
    </section>
  );
}
