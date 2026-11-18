import { useAuiState } from "@assistant-ui/react";
import CheckRounded from "@mui/icons-material/CheckRounded";
import ErrorOutlineRounded from "@mui/icons-material/ErrorOutlineRounded";
import TerminalRounded from "@mui/icons-material/TerminalRounded";
import type { Conversation } from "@meden/workspace-schema";
type Activities = NonNullable<Conversation["messages"][number]["activities"]>;
function input(argumentsText: string) {
  try {
    const value = JSON.parse(argumentsText);
    return value.command || value.path || value.contract?.name || "";
  } catch {
    return "";
  }
}
function output(text: string) {
  try {
    const value = JSON.parse(text);
    if ("stdout" in value || "stderr" in value)
      return [
        value.stdout,
        value.stderr,
        typeof value.exit_code === "number" ? "exit " + value.exit_code : "",
      ]
        .filter(Boolean)
        .join("\n");
  } catch {
    /* Plain text output. */
  }
  return text;
}
export function RecordedActivity() {
  const activities = useAuiState(
    (state) => state.message.metadata.custom.activities,
  ) as Activities | undefined;
  if (!activities?.length) return null;
  return (
    <section className="recorded-activities" aria-label="保存された実行履歴">
      <details open>
        <summary>
          <TerminalRounded fontSize="small" />
          <span>実行履歴 · {activities.length}件</span>
        </summary>
        <div className="activity-entries">
          {activities.map((activity) => (
            <details
              key={activity.id}
              className={"recorded-tool " + activity.status}
            >
              <summary>
                {activity.status === "complete" ? (
                  <CheckRounded />
                ) : (
                  <ErrorOutlineRounded />
                )}
                <code>{activity.name}</code>
                <span className="tool-input-summary">
                  {input(activity.arguments)}
                </span>
                <small>
                  {activity.status === "complete"
                    ? "完了"
                    : activity.status === "failed"
                      ? "失敗"
                      : activity.status === "incomplete"
                        ? "未完了"
                        : "結果なし"}
                </small>
              </summary>
              <div className="recorded-tool-details">
                <div>入力</div>
                <pre tabIndex={0}>{activity.arguments}</pre>
                <div>出力</div>
                <pre tabIndex={0}>
                  {output(activity.output) || "出力は記録されていません。"}
                </pre>
              </div>
            </details>
          ))}
        </div>
      </details>
    </section>
  );
}
