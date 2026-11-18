import { useEffect, useState } from "react";
import { Alert, Button, Chip, TextField, Typography } from "@mui/material";
import type { MeaningInput, MeaningRecord } from "@meden/agent-protocol/native";

type Target = MeaningInput["target"];
export function NativeMeaning({
  record,
  target,
  sourceRevision,
  graphRevision,
  authorized,
  running,
  onRun,
}: {
  record: MeaningRecord | null;
  target?: Target;
  sourceRevision?: string;
  graphRevision?: string;
  authorized: boolean;
  running: boolean;
  onRun: (input: MeaningInput) => Promise<boolean>;
}) {
  const [requirement, setRequirement] = useState("");
  useEffect(() => {
    setRequirement(
      record && sameTarget(record.input.target, target)
        ? record.input.requirement
        : "",
    );
  }, [target?.file, target?.line, target?.name]);
  const sameInput = Boolean(
    record &&
    target &&
    sourceRevision === record?.input.source_revision &&
    graphRevision === record?.input.graph_revision &&
    sameTarget(record.input.target, target) &&
    requirement === record.input.requirement,
  );
  const current = !running && record?.state === "complete" && sameInput;
  const result = record?.assessment;
  const active =
    record && ["observing", "translating", "comparing"].includes(record.state);
  const verdict =
    result?.verdict === "match"
      ? "一致の候補"
      : result?.verdict === "mismatch"
        ? "意味の差異候補"
        : "判定保留";
  return (
    <section className="native-meaning" aria-label="意味解析">
      <Typography component="h3">要件と宣言の意味を比較</Typography>
      {target && sourceRevision && (
        <>
          <TextField
            label="比較する要件"
            value={requirement}
            onChange={(event) => setRequirement(event.target.value)}
            multiline
            minRows={3}
            fullWidth
            size="small"
          />
          <Button
            disabled={running || !authorized || !requirement.trim()}
            onClick={() =>
              void onRun({
                source_revision: sourceRevision,
                ...(graphRevision ? { graph_revision: graphRevision } : {}),
                target,
                requirement,
              })
            }
          >
            この宣言の意味を解析
          </Button>
          {!authorized && <p>実行するには接続先を確認・承認してください。</p>}
          <Typography variant="caption" component="p">
            {graphRevision
              ? "コンパイル後の宣言と意味に関係する依存を自然言語化します。元ソース・コメント・証明専用の依存は最初のリクエストに含めません。"
              : "選択したファイルを自然言語化します。"}
            別のリクエストで要件と比較します。同じ設定モデルを使い、証明・契約の受理状態は変更しません。
          </Typography>
        </>
      )}
      {record && (
        <div aria-label="意味解析の結果">
          {active && (
            <p role="status">
              {
                (
                  {
                    observing: "対象のソースを再確認しています",
                    translating: "宣言を自然言語化しています（要件は未送信）",
                    comparing: "元の要件と比較しています",
                  } as Record<string, string>
                )[record.state]
              }
            </p>
          )}
          {record.state === "historical" && (
            <Alert severity="info">
              保存された解析です。現在のソースに対する結果として使うには、グラフを読み直して明示的に再実行してください。
            </Alert>
          )}
          {record.state === "stale" && (
            <Alert severity="warning">
              対象のソースが変更されました。グラフを読み直して再実行してください。
            </Alert>
          )}
          {record.state === "complete" && !current && (
            <Alert severity="warning">
              {running
                ? "処理を実行中です。表示中の解析は前回の結果です。"
                : "表示中の要件・宣言・ソース版と異なる解析です。再実行が必要です。"}
            </Alert>
          )}
          {record.state === "cancelled" && (
            <Alert severity="warning">
              意味解析を中止しました。再実行は明示的に指示してください。
            </Alert>
          )}
          {record.state === "failed" && (
            <Alert severity="error">
              意味解析を完了できませんでした。内容と接続状態を確認して再実行してください。
            </Alert>
          )}
          {record.error && <pre>{record.error}</pre>}
          {result && (
            <>
              <Chip
                label={`${verdict}（モデル評価${current ? "" : "・過去の対象"}）`}
                color={
                  current && result.verdict === "mismatch"
                    ? "warning"
                    : "default"
                }
              />
              <Typography variant="caption" component="p">
                証明済みを意味しません。自然言語の意味や解析精度は形式証明されていません。
              </Typography>
              <div className="native-meaning-comparison">
                <div>
                  <Typography component="h4">解析時の要件</Typography>
                  <pre>{record.input.requirement}</pre>
                </div>
                <div>
                  <Typography component="h4">宣言からの自然言語化</Typography>
                  <pre>{result.translation.naturalLanguage}</pre>
                </div>
              </div>
              <Typography component="h4">前提条件</Typography>
              <pre>{result.translation.preconditions}</pre>
              <Typography component="h4">結論・動作</Typography>
              <pre>{result.translation.postcondition}</pre>
              <Typography component="h4">適用範囲</Typography>
              <pre>{result.translation.scope}</pre>
              <Typography component="h4">差異と比較理由</Typography>
              <pre>
                {[result.comparison.discrepancy, result.comparison.explanation]
                  .filter(Boolean)
                  .join("\n\n")}
              </pre>
            </>
          )}
          <details>
            <summary>解析対象と処理の詳細</summary>
            <p>
              {record.input.target.name} · {record.input.target.file}:
              {record.input.target.line}
            </p>
            <p>
              モデル: {record.model} · {record.engine}
            </p>
            <p className="native-revision">{record.input.source_revision}</p>
            <p>
              claimcheckの二段階方式をRust／Leanへ適用するMedenアダプタです。別リクエストで処理し、元の会話は渡しません。完全な依存抽出や解析精度の保証は含みません。
            </p>
          </details>
        </div>
      )}
    </section>
  );
}
function sameTarget(left: Target, right?: Target) {
  return Boolean(
    right &&
    left.file === right.file &&
    left.line === right.line &&
    left.name === right.name,
  );
}
