import { lazy, Suspense, useState, useEffect } from "react";
import {
  CircularProgress,
  IconButton,
  Tab,
  Tabs,
  Typography,
} from "@mui/material";
import CloseRounded from "@mui/icons-material/CloseRounded";
import type { Declaration, Workspace } from "@meden/workspace-schema";
import { DEMO_WORKSPACE_ID, demoSnapshot } from "../../fixtures/meden-demo";
import { VendingPreview } from "../demo/VendingPreview";
import { SourceView } from "../source/SourceView";
const ProofWorkspace = lazy(() =>
  import("../proof/ProofWorkspace").then((module) => ({
    default: module.ProofWorkspace,
  })),
);
export type ArtifactTab = "code" | "preview" | "proof";
export function ArtifactPanel({
  workspace,
  tab,
  onTab,
  onClose,
  onChange,
  onNotify,
}: {
  workspace: Workspace;
  tab: ArtifactTab;
  onTab: (tab: ArtifactTab) => void;
  onClose: () => void;
  onChange: (workspace: Workspace) => Promise<boolean>;
  onNotify: (text: string) => void;
  running: boolean;
}) {
  const [selectedId, setSelectedId] = useState<string>();
  const [source, setSource] = useState<Declaration>();
  const [visitedProof, setVisitedProof] = useState(false);
  const sourceFor = (declaration: Declaration) => {
    setSource(declaration);
    onTab("code");
  };
  // Keep graph selection and running demo state while reading the source tab.
  const showProof = visitedProof || tab === "proof";
  useEffect(() => {
    if (tab === "proof") setVisitedProof(true);
  }, [tab]);
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
        aria-label="実装と検証のタブ"
      >
        {[
          ["code", "コード"],
          ["preview", "プレビュー"],
          ["proof", "証明"],
        ].map(([value, label]) => (
          <Tab
            key={value}
            value={value}
            label={label}
            id={"artifact-tab-" + value}
            aria-controls={"artifact-content-" + value}
          />
        ))}
      </Tabs>
      <div className="artifact-content">
        <div
          hidden={tab !== "code"}
          role="tabpanel"
          id="artifact-content-code"
          aria-labelledby="artifact-tab-code"
          className="artifact-tab-content"
        >
          {tab === "code" && (
            <SourceView
              files={
                workspace.id === DEMO_WORKSPACE_ID ? demoSnapshot.files : {}
              }
              target={source}
              onBack={() => onTab("proof")}
              onNotify={onNotify}
            />
          )}
        </div>
        <div
          hidden={tab !== "preview"}
          role="tabpanel"
          id="artifact-content-preview"
          aria-labelledby="artifact-tab-preview"
          className="artifact-tab-content"
        >
          {tab === "preview" &&
            (workspace.id === DEMO_WORKSPACE_ID ? (
              <VendingPreview />
            ) : (
              <div className="panel-empty">
                このワークスペースにはプレビューがありません。
              </div>
            ))}
        </div>
        <div
          hidden={tab !== "proof"}
          role="tabpanel"
          id="artifact-content-proof"
          aria-labelledby="artifact-tab-proof"
          className="artifact-tab-content"
        >
          <Suspense
            fallback={
              <div className="loading">
                <CircularProgress size={24} />
              </div>
            }
          >
            {showProof && (
              <ProofWorkspace
                workspace={workspace}
                selectedId={selectedId}
                onSelect={setSelectedId}
                onSource={sourceFor}
                onChange={onChange}
                onNotify={onNotify}
              />
            )}
          </Suspense>
        </div>
      </div>
    </section>
  );
}
