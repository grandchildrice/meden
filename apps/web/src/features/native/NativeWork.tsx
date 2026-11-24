import { useState } from "react";
import { Alert, Button, Chip, TextField, Typography } from "@mui/material";
import type {
  NativeCommand,
  PortableWorkspace,
  WorkItem,
  VerificationJobPublication,
} from "@meden/agent-protocol/native";
import {
  VerificationProgress,
  verificationActivity,
} from "./VerificationProgress";

export function NativeWork({
  workspace,
  check,
  checkError,
  retryCheck,
  running,
  authorized,
  onCommand,
  onInspect,
}: {
  workspace: PortableWorkspace;
  check?: VerificationJobPublication["data"];
  checkError?: string;
  retryCheck?: () => void;
  running: boolean;
  authorized: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  onInspect: (path: string) => void;
}) {
  const [name, setName] = useState("");
  const [selected, setSelected] = useState("");
  const [note, setNote] = useState("");
  const [next, setNext] = useState("");
  const [draft, setDraft] = useState<WorkItem>();
  const document = workspace.document;
  const task =
    document?.tasks.find((item) => item.id === selected) || document?.tasks[0];
  const events =
    document?.events.filter((event) => event.task === task?.id) || [];
  const latest = events.filter((event) => event.check).at(-1);
  return (
    <section aria-label="Work and shared history" className="native-work">
      <Typography variant="h3">Work and shared history</Typography>
      <p>
        Requirements, proof checks, notes and conversations are saved in .meden
        . Include it in Git to read it after cloning.
      </p>
      {workspace.error && (
        <Alert severity="error">
          {workspace.error} The original record is retained.
        </Alert>
      )}
      {!document ? (
        <>
          <TextField
            label="Project name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            fullWidth
          />
          <Button
            disabled={running || !name.trim() || Boolean(workspace.error)}
            onClick={() => void onCommand({ type: "initialize_work", name })}
          >
            Create shared history
          </Button>
        </>
      ) : (
        <>
          <Typography component="h3">{document.name}</Typography>
          <Button
            disabled={running}
            onClick={() => void onCommand({ type: "save_work" })}
          >
            Save conversation and work
          </Button>
          <Button
            disabled={running}
            onClick={() =>
              setDraft({
                id: "",
                title: "",
                requirement: "",
                module: "",
                theorems: [],
                next: "",
              })
            }
          >
            Add work item
          </Button>
          {document.tasks.length > 0 && (
            <TextField
              select
              fullWidth
              label="Saved work item"
              value={task?.id || ""}
              slotProps={{ select: { native: true } }}
              onChange={(e) => {
                setSelected(e.target.value);
                setNote("");
                setNext("");
              }}
            >
              {document.tasks.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.title}
                </option>
              ))}
            </TextField>
          )}
          {task && (
            <article aria-label="Work item details">
              <Typography component="h3">{task.title}</Typography>
              <Typography component="h4">Requirement</Typography>
              <pre>{task.requirement}</pre>
              <Typography component="h4">Proof targets</Typography>
              <pre>
                {task.module}
                {"\n"}
                {task.theorems.join("\n")}
              </pre>
              <Button
                disabled={running}
                onClick={() =>
                  onInspect(`${task.module.replaceAll(".", "/")}.lean`)
                }
              >
                Read proof source
              </Button>
              <Button disabled={running} onClick={() => setDraft({ ...task })}>
                Edit work item
              </Button>
              <Typography component="h4">Next steps</Typography>
              <pre>{task.next || "No continuation note yet."}</pre>
              {latest && (
                <Chip
                  label={`Saved check: ${latest.check?.success ? "Passed" : "Failed or interrupted"}`}
                />
              )}
              <p>
                Saved results are historical. Recheck on this machine to
                validate them. Accepted contracts and authorization are not
                restored.
              </p>
              <Button
                disabled={
                  running ||
                  !workspace.lean_available ||
                  task.theorems.length === 0
                }
                onClick={() =>
                  void onCommand({ type: "check_work", id: task.id })
                }
              >
                Recheck with Lean
              </Button>
              {check?.kind === "recorded" &&
                check.context.id === task.id &&
                check.phase.state === "running" && (
                  <>
                    <Typography component="p" role="status">
                      {verificationActivity(check, false)}
                    </Typography>
                    <Button onClick={() => void onCommand({ type: "cancel" })}>
                      Stop check
                    </Button>
                  </>
                )}
              <VerificationProgress
                data={
                  check?.kind === "recorded" && check.context.id === task.id
                    ? check
                    : undefined
                }
                error={checkError}
                retry={retryCheck}
              />
              {task.theorems.length === 0 && (
                <p>Add theorem names to this work item before checking.</p>
              )}
              <Button
                disabled={running || !authorized}
                onClick={() =>
                  void onCommand({ type: "resume_work", id: task.id })
                }
              >
                Resume this work item
              </Button>
              {!authorized && (
                <p>
                  Review and approve a connection to continue with a model.
                  Reading history and checking with Lean do not use inference.
                </p>
              )}
              {!workspace.lean_available && (
                <Alert severity="info">
                  Lean setup required: {workspace.lean}
                </Alert>
              )}
              <TextField
                label="Progress note"
                multiline
                minRows={2}
                fullWidth
                value={note}
                onChange={(e) => setNote(e.target.value)}
              />
              <TextField
                label="Next steps note"
                multiline
                fullWidth
                value={next}
                onChange={(e) => setNext(e.target.value)}
              />
              <Button
                disabled={running || !note.trim()}
                onClick={async () => {
                  if (
                    await onCommand({
                      type: "note_work",
                      id: task.id,
                      text: note,
                      next: next || null,
                    })
                  ) {
                    setNote("");
                    setNext("");
                  }
                }}
              >
                Save progress
              </Button>
              <Typography component="h4">Work history</Typography>
              {[...events].reverse().map((event) => (
                <details key={event.id}>
                  <summary>
                    {new Date(event.time * 1000).toLocaleString("en-US")} ·{" "}
                    {event.check
                      ? event.check.success
                        ? "Successful Lean check"
                        : "Failed or interrupted Lean check"
                      : "Work note"}
                  </summary>
                  <pre>{event.note}</pre>
                  {event.check && (
                    <>
                      <p>
                        Requirement at check time: {event.check.requirement}
                      </p>
                      <p>Target module: {event.check.module}</p>
                      <p>Lean executable: {event.check.engine_sha256}</p>
                      <p>
                        Input source revision: {event.check.source_revision}
                      </p>
                      <p>
                        Source revision after check:
                        {event.check.after_revision || "Unavailable"}
                      </p>
                      <pre aria-label="Proof check output">
                        {event.check.stdout}
                        {"\n"}
                        {event.check.stderr}
                        {"\n"}
                        {event.check.termination}
                      </pre>
                      <pre aria-label="Theorem types and axioms">
                        {event.check.inspection}
                      </pre>
                    </>
                  )}
                </details>
              ))}
            </article>
          )}
          {draft && (
            <section aria-label="Edit work item">
              <TextField
                label="Work item ID"
                fullWidth
                value={draft.id}
                onChange={(e) => setDraft({ ...draft, id: e.target.value })}
              />
              <TextField
                label="Work item title"
                fullWidth
                value={draft.title}
                onChange={(e) => setDraft({ ...draft, title: e.target.value })}
              />
              <TextField
                label="Work item requirement"
                multiline
                minRows={3}
                fullWidth
                value={draft.requirement}
                onChange={(e) =>
                  setDraft({ ...draft, requirement: e.target.value })
                }
              />
              <TextField
                label="Lean module"
                fullWidth
                value={draft.module}
                onChange={(e) => setDraft({ ...draft, module: e.target.value })}
              />
              <TextField
                label="Theorem names (one per line)"
                multiline
                fullWidth
                value={draft.theorems.join("\n")}
                onChange={(e) =>
                  setDraft({
                    ...draft,
                    theorems: e.target.value.split("\n").filter(Boolean),
                  })
                }
              />
              <TextField
                label="Continuation note"
                multiline
                fullWidth
                value={draft.next}
                onChange={(e) => setDraft({ ...draft, next: e.target.value })}
              />
              <Button
                disabled={
                  running ||
                  !draft.id ||
                  !draft.title ||
                  !draft.module ||
                  !draft.requirement
                }
                onClick={async () => {
                  if (await onCommand({ type: "put_work", work: draft })) {
                    setSelected(draft.id);
                    setDraft(undefined);
                  }
                }}
              >
                Save work item
              </Button>
              <Button onClick={() => setDraft(undefined)}>Close editor</Button>
            </section>
          )}
          <Typography component="h4">Saved conversation</Typography>
          {document.conversations.length === 0 && (
            <p>No conversations saved in this workspace yet.</p>
          )}
          {document.conversations.map((conversation) => (
            <details key={conversation.id}>
              <summary>
                Saved conversation · {conversation.messages.length} messages
              </summary>
              {conversation.messages.map((message, index) => (
                <div key={index}>
                  <strong>
                    {message.speaker === "user" ? "User" : "Assistant"}
                  </strong>
                  <pre>{message.text}</pre>
                </div>
              ))}
            </details>
          ))}
        </>
      )}
    </section>
  );
}
