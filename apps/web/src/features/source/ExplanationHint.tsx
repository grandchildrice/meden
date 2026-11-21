import { useRef, useState } from "react";
import { Tooltip, IconButton } from "@mui/material";
import InfoOutlined from "@mui/icons-material/InfoOutlined";
import type { Explanation } from "@meden/workspace-schema";

const outcomes = {
  counterexample: "Counterexample candidate",
  predicate_false_example: "Input where the defined predicate is false",
  not_found: "No counterexample found in the tested range",
  gave_up: "Could not generate inputs satisfying the assumptions",
  budget_exhausted: "Search preparation exceeded the computation budget",
  unavailable: "Search could not complete · no result available",
  unsupported: "This proposition is not supported by the installed generators",
  not_proposition: "Definition · no universal proposition asserted",
  not_run: "Counterexample search has not run",
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
          <p className="semantic-sentence">{semantic.sentence}</p>
          {(review !== "proof-only" || check.status === "counterexample") && (
            <div className="semantic-refutation">
              <p>{semantic.refutation}</p>
              <p data-check-status={check.status}>
                {check.mode === "predicate" && check.status === "not_found"
                  ? "No false input found in the tested range"
                  : outcomes[check.status]}
              </p>
              {!!check.candidates.length && (
                <ul>
                  {check.candidates.map((candidate, i) => (
                    <li key={i}>{candidate}</li>
                  ))}
                </ul>
              )}
            </div>
          )}
          <small>
            {semantic.origin === "elaborated"
              ? "Compiled expression"
              : "Source preview · not elaborated"}
            {check.seed !== undefined && ` · Plausible · seed ${check.seed}`}
            {` · ${note.proofStatus === "unproved" ? "Unfinished proof detected" : "Proof unchecked"}`}
            {review === "unknown" && " · Review scope unknown"}
          </small>
          {check.diagnostic && (
            <details>
              <summary>Details</summary>
              <p>{check.diagnostic}</p>
            </details>
          )}
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
