import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { MathText } from "./MathText";

it("keeps program identifiers literal while retaining actual mathematical formulas", () => {
  const html = renderToStaticMarkup(
    createElement(MathText, {
      text: "N_DD_AA is $N_DD_AA$ or $N_{DD_{AA}}$; $x + 1 \\le 2$.",
    }),
  );
  expect(html.match(/class="math-identifier"/g)).toHaveLength(3);
  expect(html.match(/>N_DD_AA<\/code>/g)).toHaveLength(3);
  expect(html).toContain('class="katex"');
  const formula = renderToStaticMarkup(
    createElement(MathText, { text: "$N_DD_AA = 0$" }),
  );
  expect(formula).toContain("mathtt");
  expect(formula).not.toContain("msupsub");
});
