import { useRef, useState } from "react";
import { Tooltip, IconButton } from "@mui/material";
import InfoOutlined from "@mui/icons-material/InfoOutlined";
import type { Explanation } from "@meden/workspace-schema";
import { MathText } from "./MathText";

const outcomes = {
  counterexample: "Counterexample candidate",
  predicate_false_example: "Input where the defined predicate is false",
  not_found: "No counterexample found in the tested range",
  gave_up: "Could not generate inputs satisfying the assumptions",
  budget_exhausted: "Search preparation exceeded the computation budget",
  unavailable: "Search could not complete · no result available",
  unsupported: "This proposition is not supported by the installed generators",
  not_proposition: "Definition · no universal proposition asserted",
};

export type SourceExplanation = {
  name: string;
  line: number;
  semantic: Explanation;
  review: string;
  proofStatus?: string;
};

export function ExplanationHint({
  note,
  active,
}: {
  note: SourceExplanation;
  active: boolean;
}) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const boundary = anchor.current?.closest(
    ".native-code-editor",
  ) as HTMLElement | null;
  const { semantic, review } = note;
  const check = semantic.checker;
  const bindings = check.candidates.flatMap((candidate) => {
    const match = candidate.match(/^(.+?)\s*:=\s*([\s\S]*)$/);
    return match ? [{ name: match[1]!, value: match[2]! }] : [];
  });
  const issues = check.candidates.filter(
    (candidate) => !candidate.includes(":="),
  );
  return (
    <Tooltip
      describeChild
      disableInteractive={active}
      arrow
      placement="bottom-start"
      open={active || open}
      onOpen={() => setOpen(true)}
      onClose={() => setOpen(false)}
      slotProps={{
        tooltip: {
          className: "semantic-hover-card",
          style: {
            maxWidth: boundary ? Math.max(180, boundary.clientWidth - 32) : 520,
          },
        },
        popper: {
          modifiers: [
            {
              name: "preventOverflow",
              options: {
                boundary: boundary || "clippingParents",
                altAxis: true,
                padding: 8,
              },
            },
          ],
        },
      }}
      title={
        <section aria-label={"Meaning: " + note.name}>
          <header className="semantic-heading">
            <strong>Meaning</strong>
            <span>{note.name.split(".").at(-1)}</span>
          </header>
          <p className="semantic-sentence">
            <MathText text={semantic.sentence} />
          </p>
          {(review !== "proof-only" || check.status === "counterexample") && (
            <div className="semantic-refutation">
              <h4>
                {check.mode === "predicate"
                  ? "Boundary examples"
                  : "Counterexample search"}
              </h4>
              <p data-check-status={check.status}>
                {check.mode === "predicate" && check.status === "not_found"
                  ? "No false input found in the tested range"
                  : outcomes[check.status]}
              </p>
              {!!bindings.length && (
                <table className="semantic-bindings">
                  <thead>
                    <tr>
                      <th>Input</th>
                      <th>Candidate value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {bindings.map((binding, i) => (
                      <tr key={i}>
                        <th scope="row">
                          <code>{binding.name}</code>
                        </th>
                        <td>
                          <code>{binding.value}</code>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
              {issues.map((issue, i) => (
                <p className="semantic-witness-issue" key={i}>
                  {issue.replace(/^issue:\s*/, "")}
                </p>
              ))}
              <details className="semantic-falsification">
                <summary>
                  {check.mode === "predicate"
                    ? "What these examples mean"
                    : "How this statement could fail"}
                </summary>
                <p>{semantic.refutation}</p>
              </details>
            </div>
          )}
          <small>
            Compiled expression
            {` · ${check.engine}`}
            {` · ${note.proofStatus === "unproved" ? "Unfinished proof detected" : "Proof unchecked"}`}
            {review === "unknown" && " · Review scope unknown"}
          </small>
          <details className="semantic-provenance">
            <summary>Analysis details</summary>
            <p>{semantic.renderer || "Structural English"}</p>
            {semantic.rendering_error && <p>{semantic.rendering_error}</p>}
            {check.seed !== undefined && (
              <p>
                {check.engine} · seed {check.seed} · up to {check.numInst}{" "}
                samples · size {check.maxSize}
              </p>
            )}
            {check.diagnostic && <p>{check.diagnostic}</p>}
          </details>
        </section>
      }
    >
      <IconButton
        ref={anchor}
        size="small"
        className="semantic-inlay"
        aria-label={"Explain " + note.name}
        onClick={() => setOpen((value) => !value)}
      >
        <InfoOutlined fontSize="inherit" />
      </IconButton>
    </Tooltip>
  );
}
