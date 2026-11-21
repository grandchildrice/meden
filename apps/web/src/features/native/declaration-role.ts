export type DeclarationRole = {
  id: string;
  label: string;
  color: string;
  glyph: string;
  shape: "star" | "diamond" | "circle" | "square" | "triangle";
};
const main: DeclarationRole = {
  id: "main_theorem",
  label: "主定理",
  color: "#ffd166",
  glyph: "★",
  shape: "star",
};
const theorem: DeclarationRole = {
  id: "theorem",
  label: "定理",
  color: "#a4cfff",
  glyph: "◆",
  shape: "diamond",
};
const lemma: DeclarationRole = {
  id: "lemma",
  label: "補題",
  color: "#a5e4bf",
  glyph: "●",
  shape: "circle",
};
const definition: DeclarationRole = {
  id: "definition",
  label: "定義",
  color: "#e0eafa",
  glyph: "■",
  shape: "square",
};
const axiom: DeclarationRole = {
  id: "axiom",
  label: "公理",
  color: "#ffb6cb",
  glyph: "▲",
  shape: "triangle",
};
export const declarationRoles = [main, theorem, lemma, definition, axiom];
/** Main marks explicitly tracked proof targets; it is not inferred proof status. */
export function declarationRole(
  keyword: string,
  tracked = false,
): DeclarationRole {
  if (keyword === "theorem") return tracked ? main : theorem;
  if (keyword === "lemma") return tracked ? main : lemma;
  if (keyword === "axiom" || keyword === "constant") return axiom;
  if (
    ["structure", "class", "inductive", "enum", "struct", "trait"].includes(
      keyword,
    )
  )
    return {
      ...definition,
      id: keyword,
      label:
        keyword === "class"
          ? "クラス"
          : keyword === "inductive"
            ? "帰納型"
            : "構造体",
      color: "#d7c3ff",
    };
  return definition;
}
