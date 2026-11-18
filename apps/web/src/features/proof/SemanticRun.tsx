import { useEffect, useState } from "react";
import { Button, Chip, CircularProgress, Typography } from "@mui/material";
import CheckCircleRounded from "@mui/icons-material/CheckCircleRounded";
import ErrorOutlineRounded from "@mui/icons-material/ErrorOutlineRounded";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import { diffChars } from "diff";
import type { Workspace } from "@meden/workspace-schema";
import { demoSnapshot } from "../../fixtures/meden-demo";
const steps = [
  "対象リビジョンを固定",
  "依存関係を確認",
  "定理の意味を展開",
  "要件と比較",
];
export function SemanticRun({
  workspace,
  onBack,
  onInspect,
}: {
  workspace: Workspace;
  onBack: () => void;
  onInspect: (id: string) => void;
}) {
  const [step, setStep] = useState(0);
  const [state, setState] = useState<"running" | "complete" | "cancelled">(
    "running",
  );
  const [sample, setSample] = useState(false);
  useEffect(() => {
    setState("cancelled");
    setStep(0);
  }, [workspace.sourceRevision, workspace.inputRevision]);
  useEffect(() => {
    setState("running");
  }, []);
  useEffect(() => {
    if (state !== "running") return;
    const timer = setTimeout(() => {
      if (step === 3) {
        setStep(4);
        setState("complete");
      } else setStep((value) => value + 1);
    }, 650);
    return () => clearTimeout(timer);
  }, [state, step]);
  const run = (example: boolean) => {
    setSample(example);
    setStep(0);
    setState("running");
  };
  const claim =
    workspace.requirements.find((item) => item.id === "purchase_exact") ||
    workspace.requirements[0];
  const archived = demoSnapshot.contract.properties.find(
    (item) => item.name === claim?.id,
  )?.description;
  const expected = sample
    ? "残高が150円以上なら商品を購入できる。在庫の有無は購入条件に含めない。"
    : claim?.text || "要件が含まれていません。";
  const actual = sample
    ? "残高が150円以上かつ在庫が1個以上なら商品を購入できる。購入時に残高を150円、在庫を1個減らす。"
    : archived ||
      workspace.analyses.find((item) => item.claimId === claim?.id)
        ?.backTranslation ||
      "定理の意味はまだ取得されていません。";
  const changed = expected !== actual;
  const parts = diffChars(expected, actual);
  return (
    <section className="semantic-run" aria-label="意味解析">
      <div className="semantic-run-heading">
        <Button size="small" startIcon={<ArrowBackRounded />} onClick={onBack}>
          グラフに戻る
        </Button>
        <Chip size="small" label="デモ" variant="outlined" />
      </div>
      <Typography variant="h2">意味解析</Typography>
      <Typography variant="body2" color="text.secondary">
        {sample
          ? "差分サンプル · 保存された要件・証明は変更しません"
          : "保存された要件と比較テキストの表示"}
      </Typography>
      <ol className="check-pipeline" aria-label="解析の進行状況">
        {steps.map((name, index) => (
          <li
            key={name}
            className={
              index < step
                ? "step-complete"
                : index === step && state === "running"
                  ? "step-running"
                  : "step-queued"
            }
          >
            {index < step ? (
              <CheckCircleRounded />
            ) : index === step && state === "running" ? (
              <CircularProgress size={18} />
            ) : (
              <span className="step-pending" />
            )}
            <span>{name}</span>
            <small>
              {index < step
                ? "完了"
                : index === step && state === "running"
                  ? "実行中"
                  : "待機"}
            </small>
          </li>
        ))}
      </ol>
      <div className="semantic-state" role="status" aria-live="polite">
        {state === "running"
          ? "比較画面を準備しています…"
          : state === "cancelled"
            ? "表示を中止しました"
            : "比較表示が完了しました"}
      </div>
      {state === "running" ? (
        <Button size="small" onClick={() => setState("cancelled")}>
          中止
        </Button>
      ) : (
        <Button size="small" onClick={() => run(sample)}>
          もう一度実行
        </Button>
      )}
      {state === "complete" && (
        <div className="semantic-results">
          <div
            className={
              "semantic-result-heading " + (changed ? "has-gap" : "no-gap")
            }
          >
            {changed ? <ErrorOutlineRounded /> : <CheckCircleRounded />}
            <div>
              <strong>
                {changed
                  ? sample
                    ? "在庫条件に差分があります"
                    : "文面に差分があります"
                  : "表示用テキストに差分はありません"}
              </strong>
              <small>
                {sample
                  ? "想定要件を変えたUIサンプルです。実装の不具合を検出した結果ではありません。"
                  : "解析エンジンは未接続です。この表示は意味の一致を保証しません。"}
              </small>
            </div>
          </div>
          <div className="semantic-diff" aria-label="意味の差分">
            <section>
              <header>
                <span className="diff-sign">−</span>
                {sample ? "想定要件（サンプル）" : "求める意味"}
              </header>
              <p>
                {parts
                  .filter((part) => !part.added)
                  .map((part, i) =>
                    part.removed ? (
                      <mark className="expected-diff" key={i}>
                        {part.value}
                      </mark>
                    ) : (
                      <span key={i}>{part.value}</span>
                    ),
                  )}
              </p>
            </section>
            <section>
              <header>
                <span className="diff-sign">＋</span>
                {sample
                  ? "定理が表す意味（サンプル）"
                  : "定理の比較テキスト（デモ）"}
              </header>
              <p>
                {parts
                  .filter((part) => !part.removed)
                  .map((part, i) =>
                    part.added ? (
                      <mark className="actual-diff" key={i}>
                        {part.value}
                      </mark>
                    ) : (
                      <span key={i}>{part.value}</span>
                    ),
                  )}
              </p>
            </section>
          </div>
          {sample && (
            <div className="boundary-example">
              <strong>境界ケース</strong>
              <p>残高150円・在庫0個</p>
              <div>
                <span>想定要件：購入できる</span>
                <span>定理：購入できない</span>
              </div>
            </div>
          )}
          {claim && (
            <Button size="small" onClick={() => onInspect(claim.id)}>
              要件・定理の記録を確認
            </Button>
          )}
        </div>
      )}
      <div className="semantic-demo-footer">
        <Typography variant="caption" color="text.secondary">
          処理の進行と結果表示を試すGUIデモです。LLMやLeanによる解析は実行していません。
        </Typography>
        <Button
          variant="outlined"
          disabled={state === "running"}
          onClick={() => run(!sample)}
        >
          {sample ? "保存された要件で試す" : "差分サンプルを試す"}
        </Button>
      </div>
    </section>
  );
}
