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
  embedded = false,
}: {
  embedded?: boolean;
  workspace: Workspace;
  claimId?: string;
  onChange: (workspace: Workspace) => Promise<boolean>;
  onNotify: (message: string) => void;
}) {
  const initial = claimId
    ? workspace.reviewDrafts.find((draft) => draft.claimIds.includes(claimId))
    : workspace.reviewDrafts[workspace.reviewDrafts.length - 1];
  const [draftId, setDraftId] = useState(initial?.id || crypto.randomUUID());
  const [title, setTitle] = useState(initial?.title || "Specification review");
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
      onNotify("Draft saved");
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
          <Typography variant={embedded ? "h2" : "h1"}>
            Review request
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Prepare a review request with requirements, theorems and evidence.
          </Typography>
        </div>
        <Button
          startIcon={<AddRounded />}
          onClick={() => {
            setDraftId(crypto.randomUUID());
            setTitle("Specification review");
            setNotes("");
            setSelected([]);
            setTarget({
              sourceRevision: workspace.sourceRevision,
              inputRevision: workspace.inputRevision,
            });
            setDirty(true);
          }}
        >
          New request
        </Button>
      </header>
      <div className="review-workspace">
        <aside className="review-list" aria-label="Saved review requests">
          <Typography variant="caption" color="text.secondary">
            Draft · {workspace.reviewDrafts.length}
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
                {item.claimIds.length} requirements · {item.sourceRevision}
              </span>
            </button>
          ))}
          {!workspace.reviewDrafts.length && (
            <Typography variant="body2" color="text.secondary">
              No saved review requests.
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
            <Typography variant="h2">Request details</Typography>
            <Chip
              size="small"
              label={dirty ? "Unsaved" : "Draft"}
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
                  Update target
                </Button>
              }
            >
              Requirements or source revision changed.
            </Alert>
          )}
          <TextField
            fullWidth
            required
            label="Subject"
            value={title}
            onChange={(event) => {
              setTitle(event.target.value);
              setDirty(true);
            }}
            slotProps={{ htmlInput: { maxLength: 300 } }}
          />
          <div className="review-target">
            <Typography variant="h3">Target requirements</Typography>
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
                No requirements available.
              </Typography>
            )}
          </div>
          <TextField
            fullWidth
            multiline
            minRows={5}
            label="Review questions"
            placeholder="Assumptions, boundary conditions or open questions"
            value={notes}
            onChange={(event) => {
              setNotes(event.target.value);
              setDirty(true);
            }}
            slotProps={{ htmlInput: { maxLength: 100_000 } }}
          />
          <dl className="metadata review-binding">
            <dt>Source revision</dt>
            <dd>{target.sourceRevision}</dd>
            <dt>Requirement revision</dt>
            <dd>{target.inputRevision}</dd>
          </dl>
          <Typography variant="body2" color="text.secondary">
            Export selected requirements, related theorems and evidence as
            unencrypted JSON. Conversations are excluded. Nothing is sent
            externally.
          </Typography>
          <div className="review-actions">
            <Button
              type="submit"
              variant="contained"
              disabled={!title.trim() || !selected.length}
            >
              Save draft
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
                  onNotify("Review request exported");
                } catch (error) {
                  onNotify(
                    error instanceof Error ? error.message : "Export failed",
                  );
                }
              }}
            >
              Export JSON
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}
