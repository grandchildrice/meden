import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import lean from "shiki/langs/lean4.mjs";
import rust from "shiki/langs/rust.mjs";
import toml from "shiki/langs/toml.mjs";
import markdown from "shiki/langs/markdown.mjs";
import theme from "shiki/themes/github-light-high-contrast.mjs";
// Keep -- comments lexical: the embedded Markdown rule can re-tokenize their
// contents as Lean under the browser JavaScript regex engine.
const leanGrammar = structuredClone(lean);
leanGrammar[0]!.repository!.dashComment = {
  match: "--.*$",
  name: "comment.line.double-dash.lean4",
};
const highlighter = createHighlighterCore({
  themes: [theme],
  langs: [leanGrammar, rust, toml, markdown],
  engine: createJavaScriptRegexEngine(),
});
export async function highlightSource(source: string, file: string) {
  const lang = file.endsWith(".lean")
    ? "lean4"
    : file.endsWith(".rs")
      ? "rust"
      : file.endsWith(".toml")
        ? "toml"
        : file.endsWith(".md")
          ? "markdown"
          : "text";
  return (await highlighter).codeToTokens(source, {
    lang,
    theme: "github-light-high-contrast",
  }).tokens;
}
