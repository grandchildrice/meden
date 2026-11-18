import { useEffect, useState } from "react";
import { Button, CircularProgress, Typography } from "@mui/material";
import CheckCircleRounded from "@mui/icons-material/CheckCircleRounded";
import ErrorOutlineRounded from "@mui/icons-material/ErrorOutlineRounded";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import { diffChars } from "diff";
import type { Workspace } from "@meden/workspace-schema";
import { SemanticMethodInfo } from "./SemanticMethodInfo";
import { demoSnapshot } from "../../fixtures/meden-demo";
const steps = [
  "対象リビジョンの固定",
  "依存関係の確認",
  "定理の逆翻訳",
  "依頼との意味比較",
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
  const [sample, setSample] = useState(true);
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
      if (step === steps.length - 1) {
        setStep(steps.length);
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
  const expected = claim?.text || "要件が含まれていません。";
  const actual =
    archived ||
    workspace.analyses.find((item) => item.claimId === claim?.id)
      ?.backTranslation ||
    "定理の意味はまだ取得されていません。";
  const changed = sample || expected !== actual;
  const parts = sample ? [] : diffChars(expected, actual);
  return (
    <section className="semantic-run" aria-label="意味解析">
      <div className="semantic-run-heading">
        <Button size="small" startIcon={<ArrowBackRounded />} onClick={onBack}>
          グラフに戻る
        </Button>
      </div>
      <header className="semantic-title">
        <Typography variant="h2">意味の比較</Typography>
        <SemanticMethodInfo />
      </header>
      <div className="semantic-run-status">
        <Typography variant="body2" color="text.secondary">
          {sample ? "在庫条件が異なる比較例" : "保存された要件の比較例"}
        </Typography>
        {state !== "complete" && (
          <div className="semantic-state" role="status">
            {state === "running"
              ? "比較画面を準備しています…"
              : "表示を中止しました"}
          </div>
        )}
      </div>
      {state !== "complete" && (
        <>
          <ol className="check-pipeline" aria-label="解析の進行状況">
            {steps.map((name, index) => {
              const complete = index < step;
              const active = index === step && state === "running";
              return (
                <li key={name} aria-current={active ? "step" : undefined}>
                  {complete ? (
                    <CheckCircleRounded />
                  ) : active ? (
                    <CircularProgress size={18} />
                  ) : (
                    <span className="step-pending" />
                  )}
                  <span>{name}</span>
                  <small>
                    {complete ? "完了" : active ? "実行中" : "待機"}
                  </small>
                </li>
              );
            })}
          </ol>
          {state === "running" ? (
            <Button size="small" onClick={() => setState("cancelled")}>
              中止
            </Button>
          ) : (
            <Button size="small" onClick={() => run(sample)}>
              もう一度実行
            </Button>
          )}
        </>
      )}
      {state === "complete" && (
        <div className="semantic-results">
          <div
            className={
              "semantic-result-heading " + (changed ? "has-gap" : "no-gap")
            }
            role="status"
          >
            {changed ? <ErrorOutlineRounded /> : <CheckCircleRounded />}
            <div>
              <strong>
                {changed
                  ? sample
                    ? "NG · 在庫条件に差分があります"
                    : "文面に差分があります"
                  : "表示用テキストに差分はありません"}
              </strong>
              {sample && (
                <p className="boundary-example">
                  残高150円・在庫0個の場合、依頼では購入でき、定理の条件では購入できません。
                </p>
              )}
            </div>
          </div>
          <div className="semantic-diff" aria-label="意味の差分">
            <section>
              <Typography variant="h3">元の依頼</Typography>
              <p>
                {sample ? (
                  <>
                    残高が150円以上なら購入できる。
                    <mark className="expected-diff">
                      在庫は購入条件に含めない。
                    </mark>
                  </>
                ) : (
                  parts
                    .filter((part) => !part.added)
                    .map((part, index) =>
                      part.removed ? (
                        <mark className="expected-diff" key={index}>
                          {part.value}
                        </mark>
                      ) : (
                        part.value
                      ),
                    )
                )}
              </p>
            </section>
            <section>
              <Typography variant="h3">定理の比較文</Typography>
              <p>
                {sample ? (
                  <>
                    残高が150円以上で、
                    <mark className="actual-diff">在庫が1個以上なら</mark>
                    購入できる。
                  </>
                ) : (
                  parts
                    .filter((part) => !part.removed)
                    .map((part, index) =>
                      part.added ? (
                        <mark className="actual-diff" key={index}>
                          {part.value}
                        </mark>
                      ) : (
                        part.value
                      ),
                    )
                )}
              </p>
            </section>
          </div>
          <div className="semantic-result-actions">
            <Button
              size="small"
              variant="outlined"
              onClick={() => run(!sample)}
            >
              {sample ? "一致例を表示" : "NG例を表示"}
            </Button>
            {claim && (
              <Button size="small" onClick={() => onInspect(claim.id)}>
                要件・定理の記録を確認
              </Button>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
