import { Fragment, memo } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";

/** Display-only math islands from a stored observation. No shared macros or HTML input. */
export const MathText = memo(function MathText({ text }: { text: string }) {
  return (
    <>
      {text.split(/(\$[^$\n]+\$)/g).map((part, i) =>
        part.startsWith("$") && part.endsWith("$") ? (
          <span
            key={i}
            dangerouslySetInnerHTML={{
              __html: katex.renderToString(part.slice(1, -1), {
                throwOnError: false,
                trust: false,
                strict: "ignore",
                maxExpand: 100,
                maxSize: 20,
                macros: {},
              }),
            }}
          />
        ) : (
          <Fragment key={i}>{part}</Fragment>
        ),
      )}
    </>
  );
});
