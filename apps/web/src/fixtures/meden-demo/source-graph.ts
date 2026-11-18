import type { Declaration, Workspace } from "@meden/workspace-schema";
import snapshot from "./snapshot.json" with { type: "json" };

const prefixes = {
  "Spec.lean": "KernelContract",
  "Check.lean": "KernelProof",
  "Candidate.lean": "KernelImpl",
} as const;
export function recordedSourceGraph(
  labels: Record<string, string>,
): Pick<Workspace, "declarations" | "edges"> {
  const entries = Object.entries(prefixes).flatMap(([file, prefix]) => {
    const source = snapshot.files[file as keyof typeof prefixes];
    const matches = [
      ...source.matchAll(
        /^(?:(private|protected|noncomputable)\s+)?(def|theorem|structure)\s+([^\s({:]+)/gm,
      ),
    ];
    return matches.map((match, index) => {
      const local = match[3].replace(/[«»]/g, "");
      const id =
        match[1] === "private" ? file + ":" + local : prefix + "." + local;
      const end = matches[index + 1]?.index ?? source.length;
      const body = source
        .slice(match.index, end)
        .replace(/\nend\s+\w+[\s\S]*$/, "")
        .trimEnd();
      const split = body.indexOf(":=");
      const statement = split < 0 ? body : body.slice(0, split).trimEnd();
      const property = local.match(/^(?:law_|property_)(.*)/)?.[1];
      const contract = snapshot.contract.properties.find(
        (item) => item.name === property,
      );
      const audit = snapshot.report.audit.find((line) =>
        line.startsWith("'" + id + "'"),
      );
      const axioms =
        audit
          ?.match(/\[([^\]]*)\]/)?.[1]
          .split(",")
          .map((item) => item.trim())
          .filter(Boolean) || [];
      const label = property
        ? labels[property] || local
        : {
            actual_spec: "実装と仕様の対応",
            actual_refines: "すべての性質の保存",
            abstractState: "Rustと仕様の状態対応",
            transition: "仕様の状態遷移",
            State: "仕様の状態",
            "kernel.step": "Rustの状態遷移",
            "kernel.Machine": "Rustの状態",
            output_bounds: "出力値の範囲",
            success: "状態遷移の成功",
          }[local] || local.replace(/^example_(\d+)$/, "具体例 $1");
      const declaration: Declaration = {
        id,
        name: match[1] === "private" ? local : id,
        label,
        kind: match[2] === "theorem" ? "theorem" : "definition",
        statement,
        summary: contract?.description,
        source: { file, line: source.slice(0, match.index).split("\n").length },
        proofStatus: audit ? "proved" : "unknown",
        ...(audit
          ? {
              proofSourceRevision: "sha256:" + snapshot.sourceSha256,
              proofInputRevision: 1,
            }
          : {}),
        evidenceRefs: audit ? ["recorded-verification"] : [],
        axioms,
      };
      return { declaration, local, prefix, file, body, split };
    });
  });
  const edges: Workspace["edges"] = [];
  // A source-reference index, NOT Lean elaboration or full Atlas extraction.
  // Only exact known project names are resolved; imports/stdlib remain outside this view.
  for (const entry of entries) {
    const clean = entry.body
      .replace(/\/-[\s\S]*?-\//g, "")
      .replace(/--[^\n]*/g, "")
      .replace(/[«»]/g, "");
    const boundary = clean.indexOf(":=");
    const seen = new Set<string>();
    for (const token of clean.matchAll(
      /[A-Za-z_][A-Za-z_0-9]*(?:\.[A-Za-z_][A-Za-z_0-9]*)*/g,
    )) {
      const target =
        entries.find((other) => other.declaration.id === token[0]) ||
        entries.find(
          (other) => other.file === entry.file && other.local === token[0],
        );
      if (!target || target === entry) continue;
      const site = boundary < 0 || token.index < boundary ? "type" : "value";
      const key = target.declaration.id + ":" + site;
      if (seen.has(key)) continue;
      seen.add(key);
      edges.push({
        source: entry.declaration.id,
        target: target.declaration.id,
        site,
      });
    }
  }
  return { declarations: entries.map((entry) => entry.declaration), edges };
}
