import { memo } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";

/** Display-only math islands from a stored observation. No shared macros or HTML input. */
export const MathText = memo(function MathText({ text }: { text: string }) {
  return (
    <>
      {text.split(/(\$[^$\n]+\$)/g).map((part, i) => {
        const math = part.startsWith("$") && part.endsWith("$");
        const literal = math ? part.slice(1, -1).trim() : part;
        const identifier = literal
          .replace(/\\_/g, "_")
          .replace(/\\(?:mathrm|mathit|mathtt|text)\{([^{}]*)\}/g, "$1")
          .replace(/[{}]/g, "");
        if (math && /^[\p{L}\p{N}_'.]+$/u.test(identifier))
          return (
            <code className="math-identifier" key={i}>
              {identifier}
            </code>
          );
        return math ? (
          <span
            key={i}
            dangerouslySetInnerHTML={{
              __html: katex.renderToString(
                literal.replace(
                  /\b[A-Za-z][A-Za-z0-9]*(?:_(?:[A-Za-z0-9]+|\{[A-Za-z0-9_{}]+\}))+/g,
                  (token) => {
                    const name = token.replace(/[{}]/g, "");
                    return (name.match(/_/g)?.length || 0) >= 2
                      ? `\\mathtt{${name.replace(/_/g, "\\_")}}`
                      : token;
                  },
                ),
                {
                  throwOnError: false,
                  trust: false,
                  strict: "ignore",
                  maxExpand: 100,
                  maxSize: 20,
                  macros: {},
                },
              ),
            }}
          />
        ) : (
          <span key={i}>
            {part
              .split(/(\b[A-Za-z][A-Za-z0-9]*(?:_[A-Za-z0-9]+)+\b)/g)
              .map((token, j) =>
                /^[A-Za-z][A-Za-z0-9]*(?:_[A-Za-z0-9]+)+$/.test(token) ? (
                  <code className="math-identifier" key={j}>
                    {token}
                  </code>
                ) : (
                  token
                ),
              )}
          </span>
        );
      })}
    </>
  );
});
