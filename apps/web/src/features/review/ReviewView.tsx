import { useState } from "react";
import {
  Alert,
  Button,
  Checkbox,
  Chip,
  FormControlLabel,
  TextField,
  Typography,
} from "@mui/material";
import AddRounded from "@mui/icons-material/AddRounded";
import FileDownloadOutlined from "@mui/icons-material/FileDownloadOutlined";
import { createReviewPackage } from "@meden/expert-review";
import type { Workspace } from "@meden/workspace-schema";
import { downloadJson } from "../../adapters/files";

export function ReviewView({
  workspace,
  claimId,
  onChange,
  onNotify,
}: {
  workspace: Workspace;
  claimId?: string;
  onChange: (workspace: Workspace) => Promise<boolean>;
  onNotify: (message: string) => void;
}) {
  const initial = claimId
    ? workspace.reviewDrafts.find((draft) => draft.claimIds.includes(claimId))
    : workspace.reviewDrafts[workspace.reviewDrafts.length - 1];
  const [draftId, setDraftId] = useState(initial?.id || crypto.randomUUID());
  const [title, setTitle] = useState(initial?.title || "仕様のレビュー");
  const [notes, setNotes] = useState(initial?.notes || "");
  const [selected, setSelected] = useState<string[]>(
    initial?.claimIds || (claimId ? [claimId] : []),
  );
  const [target, setTarget] = useState({
    sourceRevision: initial?.sourceRevision || workspace.sourceRevision,
    inputRevision: initial?.inputRevision ?? workspace.inputRevision,
  });
  const [dirty, setDirty] = useState(!initial);
  const stale =
    target.sourceRevision !== workspace.sourceRevision ||
    target.inputRevision !== workspace.inputRevision;
  const draft = {
    id: draftId,
    title: title.trim(),
    notes,
    claimIds: selected,
    ...target,
    updatedAt: new Date().toISOString(),
  };
  const save = async () => {
    const saved = await onChange({
      ...workspace,
      reviewDrafts: [
        ...workspace.reviewDrafts.filter((item) => item.id !== draftId),
        draft,
      ],
    });
    if (saved) {
      setDirty(false);
      onNotify("下書きを保存しました");
    }
    return saved;
  };
  const load = (id: string) => {
    const existing = workspace.reviewDrafts.find((item) => item.id === id)!;
    setDraftId(id);
    setTitle(existing.title);
    setNotes(existing.notes);
    setSelected(existing.claimIds);
    setTarget({
      sourceRevision: existing.sourceRevision,
      inputRevision: existing.inputRevision,
    });
    setDirty(false);
  };
  return (
    <section className="review-page">
      <header className="page-heading">
        <div>
          <Typography variant="h1">レビュー依頼</Typography>
          <Typography variant="body2" color="text.secondary">
            要件、定理、根拠をまとめた依頼書を作成します。
          </Typography>
        </div>
        <Button
          startIcon={<AddRounded />}
          onClick={() => {
            setDraftId(crypto.randomUUID());
            setTitle("仕様のレビュー");
            setNotes("");
            setSelected([]);
            setTarget({
              sourceRevision: workspace.sourceRevision,
              inputRevision: workspace.inputRevision,
            });
            setDirty(true);
          }}
        >
          新しい依頼
        </Button>
      </header>
      <div className="review-workspace">
        <aside className="review-list" aria-label="保存したレビュー依頼">
          <Typography variant="caption" color="text.secondary">
            下書き · {workspace.reviewDrafts.length}
          </Typography>
          {workspace.reviewDrafts.map((item) => (
            <button
              className={
                "review-list-item " + (draftId === item.id ? "selected" : "")
              }
              key={item.id}
              onClick={() => load(item.id)}
            >
              <strong>{item.title}</strong>
              <span>
                {item.claimIds.length}件の要件 · {item.sourceRevision}
              </span>
            </button>
          ))}
          {!workspace.reviewDrafts.length && (
            <Typography variant="body2" color="text.secondary">
              保存した依頼はありません。
            </Typography>
          )}
        </aside>
        <form
          className="review-form"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <div className="review-form-header">
            <Typography variant="h2">依頼内容</Typography>
            <Chip
              size="small"
              label={dirty ? "未保存" : "下書き"}
              variant="outlined"
            />
          </div>
          {stale && (
            <Alert
              severity="warning"
              action={
                <Button
                  color="inherit"
                  onClick={() => {
                    setTarget({
                      sourceRevision: workspace.sourceRevision,
                      inputRevision: workspace.inputRevision,
                    });
                    setDirty(true);
                  }}
                >
                  対象を更新
                </Button>
              }
            >
              要件または対象版が変更されています。
            </Alert>
          )}
          <TextField
            fullWidth
            required
            label="件名"
            value={title}
            onChange={(event) => {
              setTitle(event.target.value);
              setDirty(true);
            }}
            slotProps={{ htmlInput: { maxLength: 300 } }}
          />
          <div className="review-target">
            <Typography variant="h3">対象の要件</Typography>
            {workspace.requirements.map((claim) => (
              <FormControlLabel
                key={claim.id}
                control={
                  <Checkbox
                    checked={selected.includes(claim.id)}
                    onChange={(_, checked) => {
                      setSelected((previous) =>
                        checked
                          ? [...previous, claim.id]
                          : previous.filter((id) => id !== claim.id),
                      );
                      setDirty(true);
                    }}
                  />
                }
                label={
                  <span>
                    <span className="muted">{claim.id}</span> {claim.title}
                  </span>
                }
              />
            ))}
            {!workspace.requirements.length && (
              <Typography color="text.secondary">
                選択できる要件はありません。
              </Typography>
            )}
          </div>
          <TextField
            fullWidth
            multiline
            minRows={5}
            label="確認してほしい点"
            placeholder="前提、境界条件、判断に迷っている点など"
            value={notes}
            onChange={(event) => {
              setNotes(event.target.value);
              setDirty(true);
            }}
            slotProps={{ htmlInput: { maxLength: 100_000 } }}
          />
          <dl className="metadata review-binding">
            <dt>対象版</dt>
            <dd>{target.sourceRevision}</dd>
            <dt>要件の版</dt>
            <dd>{target.inputRevision}</dd>
          </dl>
          <Typography variant="body2" color="text.secondary">
            選択した要件と関連する定理・根拠を、暗号化していないJSONに書き出します。会話は含めません。外部への送信は行いません。
          </Typography>
          <div className="review-actions">
            <Button
              type="submit"
              variant="contained"
              disabled={!title.trim() || !selected.length}
            >
              下書きを保存
            </Button>
            <Button
              variant="outlined"
              startIcon={<FileDownloadOutlined />}
              disabled={!title.trim() || !selected.length || stale}
              onClick={async () => {
                try {
                  const value = createReviewPackage(workspace, draft);
                  await save();
                  downloadJson("meden-review-request.json", value);
                  onNotify("依頼書を書き出しました");
                } catch (error) {
                  onNotify(
                    error instanceof Error
                      ? error.message
                      : "書き出せませんでした",
                  );
                }
              }}
            >
              JSONを書き出す
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}
