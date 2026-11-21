import { useState } from "react";
import { IconButton, Popover, Typography } from "@mui/material";
import InfoOutlined from "@mui/icons-material/InfoOutlined";
import CloseRounded from "@mui/icons-material/CloseRounded";
const method = [
  [
    "Theorem back-translation",
    "A translator LLM receives the Lean theorem and describes its assumptions and conclusions without seeing the original requirement.",
  ],
  [
    "Compare meaning with the request",
    "A separate judge receives the original requirement and back-translation to assess missing conditions, extra assumptions, numerical differences and scope.",
  ],
  [
    "Show differences and their effects",
    "Show differing conditions and examples of their effects, such as whether a purchase is allowed at exactly 150 yen when stock is available.",
  ],
];
export function SemanticMethodInfo() {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  return (
    <>
      <IconButton
        aria-label="How meaning analysis works"
        aria-haspopup="dialog"
        aria-expanded={Boolean(anchor)}
        onClick={(event) => setAnchor(event.currentTarget)}
        className="semantic-info-button"
      >
        <InfoOutlined fontSize="small" />
      </IconButton>
      <Popover
        open={Boolean(anchor)}
        anchorEl={anchor}
        onClose={() => setAnchor(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "right" }}
        slotProps={{
          paper: {
            role: "dialog",
            "aria-labelledby": "semantic-method-title",
            className: "semantic-method-info",
          },
        }}
      >
        <header>
          <Typography variant="h3" id="semantic-method-title">
            Meaning analysis algorithm
          </Typography>
          <IconButton
            aria-label="Close explanation"
            onClick={() => setAnchor(null)}
          >
            <CloseRounded fontSize="small" />
          </IconButton>
        </header>
        <Typography variant="caption" color="text.secondary">
          A design based on claimcheck's two-stage approach. The demo displays
          prepared comparison examples.
        </Typography>
        <ol>
          {method.map(([title, text]) => (
            <li key={title}>
              <strong>{title}</strong>
              <p>{text}</p>
            </li>
          ))}
        </ol>
        <a
          className="semantic-method-link"
          href="https://github.com/metareflection/claimcheck"
          target="_blank"
          rel="noreferrer"
        >
          claimcheck's two-stage approach
        </a>
      </Popover>
    </>
  );
}
