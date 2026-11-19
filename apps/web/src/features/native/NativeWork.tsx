import { useState } from "react";
import { Alert, Button, Chip, TextField, Typography } from "@mui/material";
import type {
  NativeCommand,
  PortableWorkspace,
  WorkItem,
} from "@meden/agent-protocol/native";

export function NativeWork({
  workspace,
  running,
  authorized,
  onCommand,
  onInspect,
}: {
  workspace: PortableWorkspace;
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
    <section aria-label="作業と共有履歴" className="native-work">
      <Typography variant="h3">作業と共有履歴</Typography>
      <p>
        要件・証明チェック・再開メモ・会話を .meden
        に保存します。Gitに含めると、clone先でも確認できます。
      </p>
      {workspace.error && (
        <Alert severity="error">
          {workspace.error} 元の記録は保持されています。
        </Alert>
      )}
      {!document ? (
        <>
          <TextField
            label="プロジェクト名"
            value={name}
            onChange={(e) => setName(e.target.value)}
            fullWidth
          />
          <Button
            disabled={running || !name.trim() || Boolean(workspace.error)}
            onClick={() => void onCommand({ type: "initialize_work", name })}
          >
            共有履歴を作成
          </Button>
        </>
      ) : (
        <>
          <Typography component="h3">{document.name}</Typography>
          <Button
            disabled={running}
            onClick={() => void onCommand({ type: "save_work" })}
          >
            会話と作業を保存
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
            作業を追加
          </Button>
          {document.tasks.length > 0 && (
            <TextField
              select
              fullWidth
              label="保存された作業"
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
            <article aria-label="作業の詳細">
              <Typography component="h3">{task.title}</Typography>
              <Typography component="h4">要件</Typography>
              <pre>{task.requirement}</pre>
              <Typography component="h4">証明対象</Typography>
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
                証明コードを読む
              </Button>
              <Button disabled={running} onClick={() => setDraft({ ...task })}>
                作業を編集
              </Button>
              <Typography component="h4">次にすること</Typography>
              <pre>{task.next || "再開メモはまだありません。"}</pre>
              {latest && (
                <Chip
                  label={`保存されたチェック記録：${latest.check?.success ? "成功" : "失敗・中断"}`}
                />
              )}
              <p>
                保存された結果は履歴です。このPCで確かめるには再チェックしてください。受理した契約や承認権限は復元しません。
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
                Leanを再チェック
              </Button>
              {task.theorems.length === 0 && (
                <p>チェックする定理名を作業に追加してください。</p>
              )}
              <Button
                disabled={running || !authorized}
                onClick={() =>
                  void onCommand({ type: "resume_work", id: task.id })
                }
              >
                この作業を再開
              </Button>
              {!authorized && (
                <p>
                  モデルと作業を再開するには、接続先を確認・承認してください。履歴の閲覧とLeanチェックには推論を使いません。
                </p>
              )}
              {!workspace.lean_available && (
                <Alert severity="info">
                  Leanの準備が必要です：{workspace.lean}
                </Alert>
              )}
              <TextField
                label="進捗メモ"
                multiline
                minRows={2}
                fullWidth
                value={note}
                onChange={(e) => setNote(e.target.value)}
              />
              <TextField
                label="次の作業のメモ"
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
                進捗を保存
              </Button>
              <Typography component="h4">作業履歴</Typography>
              {[...events].reverse().map((event) => (
                <details key={event.id}>
                  <summary>
                    {new Date(event.time * 1000).toLocaleString()} ·{" "}
                    {event.check
                      ? event.check.success
                        ? "Leanチェック成功の記録"
                        : "Leanチェック失敗・中断の記録"
                      : "作業メモ"}
                  </summary>
                  <pre>{event.note}</pre>
                  {event.check && (
                    <>
                      <p>チェック時の要件：{event.check.requirement}</p>
                      <p>対象モジュール：{event.check.module}</p>
                      <p>Lean実行ファイル：{event.check.engine_sha256}</p>
                      <p>入力ソース版：{event.check.source_revision}</p>
                      <p>
                        終了後のソース版：
                        {event.check.after_revision || "取得できませんでした"}
                      </p>
                      <pre aria-label="証明チェックの出力">
                        {event.check.stdout}
                        {"\n"}
                        {event.check.stderr}
                        {"\n"}
                        {event.check.termination}
                      </pre>
                      <pre aria-label="定理の型と公理">
                        {event.check.inspection}
                      </pre>
                    </>
                  )}
                </details>
              ))}
            </article>
          )}
          {draft && (
            <section aria-label="作業の編集">
              <TextField
                label="作業ID"
                fullWidth
                value={draft.id}
                onChange={(e) => setDraft({ ...draft, id: e.target.value })}
              />
              <TextField
                label="作業名"
                fullWidth
                value={draft.title}
                onChange={(e) => setDraft({ ...draft, title: e.target.value })}
              />
              <TextField
                label="作業の要件"
                multiline
                minRows={3}
                fullWidth
                value={draft.requirement}
                onChange={(e) =>
                  setDraft({ ...draft, requirement: e.target.value })
                }
              />
              <TextField
                label="Leanモジュール"
                fullWidth
                value={draft.module}
                onChange={(e) => setDraft({ ...draft, module: e.target.value })}
              />
              <TextField
                label="定理名（1行に1件）"
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
                label="再開時の作業"
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
                作業を保存
              </Button>
              <Button onClick={() => setDraft(undefined)}>編集を閉じる</Button>
            </section>
          )}
          <Typography component="h4">保存された会話</Typography>
          {document.conversations.length === 0 && (
            <p>このworkspaceで保存した会話はまだありません。</p>
          )}
          {document.conversations.map((conversation) => (
            <details key={conversation.id}>
              <summary>
                保存された会話 · {conversation.messages.length}メッセージ
              </summary>
              {conversation.messages.map((message, index) => (
                <div key={index}>
                  <strong>
                    {message.speaker === "user" ? "ユーザー" : "アシスタント"}
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
