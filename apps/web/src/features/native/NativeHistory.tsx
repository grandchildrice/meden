import { Alert, Button, Typography } from "@mui/material";
import type { NativeState, NativeCommand } from "@meden/agent-protocol/native";
export function NativeHistory({
  state,
  running,
  onCommand,
  onOpen,
  onManage,
}: {
  state: NativeState;
  running: boolean;
  onCommand: (command: NativeCommand) => Promise<boolean>;
  onOpen: (file: string) => void;
  onManage: () => void;
}) {
  const document = state.portable.document;
  return (
    <section className="chat-work-history" aria-label="作業の記録">
      <div className="chat-work-heading">
        <Typography variant="caption">作業の記録</Typography>
        <Button onClick={onManage}>作業・履歴</Button>
      </div>
      {state.logs.length > 0 && (
        <details className="chat-tool-records">
          <summary>ツールの記録</summary>
          {state.logs.map((entry, i) => (
            <details key={i}>
              <summary>
                {entry.type === "notice" ? "保存の通知" : entry.tool}
              </summary>
              {entry.type === "notice" ? (
                <Alert severity="warning">{entry.text}</Alert>
              ) : (
                <pre>{entry.text}</pre>
              )}
            </details>
          ))}
        </details>
      )}
      {document?.tasks.map((task) => (
        <article key={task.id} className="chat-work-card">
          <Typography component="h3">{task.title}</Typography>
          <p>{task.requirement}</p>
          <div className="chat-work-actions">
            <Button
              disabled={running}
              onClick={() => onOpen(task.module.replaceAll(".", "/") + ".lean")}
            >
              コードを見る
            </Button>
            <Button
              disabled={
                running ||
                !state.portable.lean_available ||
                !task.theorems.length
              }
              onClick={() =>
                void onCommand({ type: "check_work", id: task.id })
              }
            >
              Leanを再チェック
            </Button>
            <Button
              disabled={running || !state.authorized}
              onClick={() =>
                void onCommand({ type: "resume_work", id: task.id })
              }
            >
              続ける
            </Button>
          </div>
          {document.events
            .filter((e) => e.task === task.id)
            .map((event) => (
              <details key={event.id}>
                <summary>
                  {event.check
                    ? event.check.success
                      ? "Leanチェック成功の記録"
                      : "Leanチェック失敗・中断の記録"
                    : "作業メモ"}{" "}
                  · {new Date(event.time * 1000).toLocaleString()}
                </summary>
                <p>{event.note}</p>
                {event.check && (
                  <>
                    <pre aria-label="定理の型と公理">
                      {event.check.inspection}
                    </pre>
                    <pre aria-label="証明チェックの出力">
                      {event.check.stdout}
                      {event.check.stderr}
                      {event.check.termination}
                    </pre>
                  </>
                )}
              </details>
            ))}
        </article>
      ))}
    </section>
  );
}
