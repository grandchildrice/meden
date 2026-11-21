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
  "Freeze target revision",
  "Inspect dependencies",
  "Theorem back-translation",
  "Compare meaning with the request",
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
  const expected = claim?.text || "No requirement included.";
  const actual =
    archived ||
    workspace.analyses.find((item) => item.claimId === claim?.id)
      ?.backTranslation ||
    "Theorem meaning is not available yet.";
  const changed = sample || expected !== actual;
  const parts = sample ? [] : diffChars(expected, actual);
  return (
    <section className="semantic-run" aria-label="Meaning analysis">
      <div className="semantic-run-heading">
        <Button size="small" startIcon={<ArrowBackRounded />} onClick={onBack}>
          Back to graph
        </Button>
      </div>
      <header className="semantic-title">
        <Typography variant="h2">Meaning comparison</Typography>
        <SemanticMethodInfo />
      </header>
      <div className="semantic-run-status">
        <Typography variant="body2" color="text.secondary">
          {sample
            ? "Purchase boundary comparison example"
            : "Saved requirement comparison example"}
        </Typography>
        {state !== "complete" && (
          <div className="semantic-state" role="status">
            {state === "running"
              ? "Preparing comparison…"
              : "Display cancelled"}
          </div>
        )}
      </div>
      {state !== "complete" && (
        <>
          <ol className="check-pipeline" aria-label="Analysis progress">
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
                    {complete ? "Complete" : active ? "Running" : "Waiting"}
                  </small>
                </li>
              );
            })}
          </ol>
          {state === "running" ? (
            <Button size="small" onClick={() => setState("cancelled")}>
              Cancel
            </Button>
          ) : (
            <Button size="small" onClick={() => run(sample)}>
              Run again
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
                    ? "NG · Difference at the 150-yen boundary"
                    : "The wording differs"
                  : "The displayed text matches"}
              </strong>
              {sample && (
                <p className="boundary-example">
                  At a balance of 150 yen and stock of one, the request permits
                  purchase but the example theorem does not.
                </p>
              )}
            </div>
          </div>
          <div className="semantic-diff" aria-label="Meaning differences">
            <section>
              <Typography variant="h3">
                {sample ? "Example request" : "Saved requirement"}
              </Typography>
              <p>
                {sample ? (
                  <>
                    Purchase is allowed only when the balance is{" "}
                    <mark className="expected-diff">at least 150 yen</mark> and
                    stock is at least one.
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
              <Typography variant="h3">
                {sample
                  ? "Example theorem translated into natural language"
                  : "Theorem comparison text"}
              </Typography>
              <p>
                {sample ? (
                  <>
                    The balance is{" "}
                    <mark className="actual-diff">
                      strictly greater than 150 yen
                    </mark>
                    and stock is at least one, purchase is allowed.
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
              {sample ? "Show matching example" : "Show mismatch example"}
            </Button>
            {claim && (
              <Button size="small" onClick={() => onInspect(claim.id)}>
                Inspect requirement and theorem records
              </Button>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
