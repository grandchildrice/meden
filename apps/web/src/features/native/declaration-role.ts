export type DeclarationRole = {
  id: string;
  label: string;
  color: string;
  glyph: string;
  shape: "star" | "diamond" | "circle" | "square" | "triangle";
};
const main: DeclarationRole = {
  id: "main_theorem",
  label: "Main theorem",
  color: "#ffd166",
  glyph: "★",
  shape: "star",
};
const theorem: DeclarationRole = {
  id: "theorem",
  label: "Theorem",
  color: "#a4cfff",
  glyph: "◆",
  shape: "diamond",
};
const lemma: DeclarationRole = {
  id: "lemma",
  label: "Lemma",
  color: "#a5e4bf",
  glyph: "●",
  shape: "circle",
};
const definition: DeclarationRole = {
  id: "definition",
  label: "Definition",
  color: "#e0eafa",
  glyph: "■",
  shape: "square",
};
const axiom: DeclarationRole = {
  id: "axiom",
  label: "Axiom",
  color: "#ffb6cb",
  glyph: "▲",
  shape: "triangle",
};
export const declarationRoles = [main, theorem, lemma, definition, axiom];
/** Main marks saved/attributed targets or entry-module theorems, never proof status. */
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
          ? "Class"
          : keyword === "inductive"
            ? "Inductive type"
            : "Structure",
      color: "#d7c3ff",
    };
  return definition;
}
