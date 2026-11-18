import type { Declaration, Workspace } from "@meden/workspace-schema";

/** Lexical project references only: no elaboration, imported library closure or proof admission. */
export function sourceIndex(snapshot: {
  revision: string;
  files: Record<string, string>;
}): Pick<Workspace, "declarations" | "edges"> {
  const entries = Object.entries(snapshot.files)
    .sort(([a], [b]) => a.localeCompare(b))
    .flatMap(([file, source], fileIndex) =>
      declarations(file, source, fileIndex),
    );
  const edges: Workspace["edges"] = [];
  for (const entry of entries) {
    const seen = new Set<string>();
    // Only the body/signature after the declaration's own name contributes references.
    for (const token of entry.clean
      .slice(entry.nameEnd)
      .matchAll(
        /[\p{L}_][\p{L}\p{N}_']*(?:(?:\.|::)[\p{L}_][\p{L}\p{N}_']*)*/gu,
      )) {
      const target = resolve(entries, entry, token[0]);
      if (!target || target === entry || seen.has(target.declaration.id))
        continue;
      seen.add(target.declaration.id);
      edges.push({
        source: entry.declaration.id,
        target: target.declaration.id,
        site: "unknown",
      });
    }
  }
  return { declarations: entries.map((entry) => entry.declaration), edges };
}

type Entry = {
  declaration: Declaration;
  namespace: string;
  local: string;
  file: string;
  private: boolean;
  clean: string;
  nameEnd: number;
};

function declarations(
  file: string,
  source: string,
  fileIndex: number,
): Entry[] {
  const lean = file.endsWith(".lean");
  const clean = maskSource(source, lean ? "lean" : "rust");
  const pattern = lean
    ? /^[ \t]*(?:(?:private|protected|noncomputable|unsafe|partial)\s+)*(def|theorem|lemma|structure|class|inductive|abbrev|opaque|axiom|constant)\s+([\p{L}_][\p{L}\p{N}_'.]*|«[^»\n]+»)/gmu
    : /^[ \t]*(?:(?:pub(?:\([^\n)]*\))?|async|unsafe|const|extern(?:\s+"[^"\n]*")?)\s+)*(fn|struct|enum|type|trait|const|static)\s+([\p{L}_][\p{L}\p{N}_]*)/gmu;
  const matches = [...clean.matchAll(pattern)];
  // Namespace/section commands delimit Lean declaration bodies as well.
  const commands = lean
    ? [...clean.matchAll(/^[ \t]*(namespace|section|end)(?:[ \t]+([^\s]+))?/gm)]
    : [];
  const namespaces: Array<{ name: string; kind: string }> = [];
  let cursor = 0;
  return matches.map((match, ordinal) => {
    while (cursor < commands.length && commands[cursor]!.index < match.index) {
      const command = commands[cursor++]!;
      if (command[1] === "end") namespaces.pop();
      else
        namespaces.push({
          kind: command[1]!,
          name: command[1] === "namespace" ? command[2] || "" : "",
        });
    }
    const namespace = namespaces
      .map((item) => item.name)
      .filter(Boolean)
      .join(".");
    const local = match[2]!.replace(/[«»]/g, "");
    const name = lean && namespace ? namespace + "." + local : local;
    const nextCommand =
      commands.find((command) => command.index > match.index)?.index ??
      source.length;
    const end = Math.min(
      matches[ordinal + 1]?.index ?? source.length,
      nextCommand,
    );
    const body = source.slice(match.index, end).trimEnd();
    const boundary = lean ? body.indexOf(":=") : body.indexOf("{");
    const statement = boundary < 0 ? body : body.slice(0, boundary).trimEnd();
    return {
      declaration: {
        id: `file${fileIndex}:declaration${ordinal}`,
        name,
        label: name,
        kind: ["theorem", "lemma"].includes(match[1]!)
          ? "theorem"
          : ["axiom", "constant"].includes(match[1]!)
            ? "axiom"
            : "definition",
        statement,
        source: { file, line: source.slice(0, match.index).split("\n").length },
        proofStatus: "unknown",
        evidenceRefs: [],
        axioms: [],
      },
      namespace,
      local,
      file,
      private: /\bprivate\b/.test(match[0]),
      clean: clean.slice(match.index, end),
      nameEnd: match[0].length,
    } satisfies Entry;
  });
}

function resolve(
  entries: Entry[],
  entry: Entry,
  token: string,
): Entry | undefined {
  const eligible = entries.filter(
    (other) => !other.private || other.file === entry.file,
  );
  const candidates: string[] = [];
  const parts = entry.namespace.split(".").filter(Boolean);
  while (parts.length) {
    candidates.push(parts.join(".") + "." + token);
    parts.pop();
  }
  candidates.push(token);
  for (const name of candidates) {
    const named = eligible.filter((other) => other.declaration.name === name);
    if (named.length === 1) return named[0];
  }
  const local = eligible.filter(
    (other) => other.file === entry.file && other.local === token,
  );
  return local.length === 1 ? local[0] : undefined;
}

/** Preserve offsets/newlines while excluding comments and literal text from reference matching. */
export function maskSource(source: string, language: "lean" | "rust"): string {
  const chars = source.split("");
  let position = 0;
  const blank = (start: number, end: number) => {
    for (let index = start; index < end; index++)
      if (chars[index] !== "\n" && chars[index] !== "\r") chars[index] = " ";
  };
  while (position < source.length) {
    const start = position;
    const lineComment = language === "lean" ? "--" : "//";
    const blockStart = language === "lean" ? "/-" : "/*";
    const blockEnd = language === "lean" ? "-/" : "*/";
    if (source.startsWith(lineComment, position)) {
      const end = source.indexOf("\n", position);
      position = end < 0 ? source.length : end;
      blank(start, position);
    } else if (source.startsWith(blockStart, position)) {
      let depth = 1;
      position += 2;
      while (position < source.length && depth > 0) {
        if (source.startsWith(blockStart, position)) {
          depth++;
          position += 2;
        } else if (source.startsWith(blockEnd, position)) {
          depth--;
          position += 2;
        } else position++;
      }
      blank(start, position);
    } else if (language === "rust" && /^r#+"/.test(source.slice(position))) {
      const delimiter = source.slice(position).match(/^r(#+)"/)![1]!;
      const end = source.indexOf(
        '"' + delimiter,
        position + delimiter.length + 2,
      );
      position = end < 0 ? source.length : end + delimiter.length + 1;
      blank(start, position);
    } else if (
      source[position] === '"' ||
      (language === "rust" &&
        /^'(?:[^'\\\n]|\\.)'/.test(source.slice(position)))
    ) {
      const quote = source[position];
      position++;
      while (position < source.length) {
        if (source[position] === "\\") position += 2;
        else if (source[position++] === quote) break;
      }
      blank(start, Math.min(position, source.length));
    } else position++;
  }
  return chars.join("");
}
