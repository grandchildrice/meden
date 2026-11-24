export type ReferenceSymbol = {
  id: string;
  file: string;
  name: string;
  private: boolean;
  references: readonly string[];
};
type Unique = Map<string, ReferenceSymbol | null>;
const insert = (index: Unique, key: string, symbol: ReferenceSymbol) => {
  index.set(key, index.has(key) ? null : symbol);
};

/** Resolve display references, including ambiguity and file-local visibility.
 * A unique candidate is retained once; null records multiple candidates. Neither
 * case needs another filtered array for every reference to the same name.
 * These lexical display edges carry no compiler or proof authority.
 */
export function referenceEdges(symbols: ReadonlyMap<string, ReferenceSymbol>) {
  const publicNames: Unique = new Map(),
    privateNames = new Map<string, Unique>(),
    local: Unique = new Map();
  for (const symbol of symbols.values()) {
    let index = publicNames;
    if (symbol.private) {
      index = privateNames.get(symbol.file) || new Map();
      privateNames.set(symbol.file, index);
    }
    insert(index, symbol.name, symbol);
    insert(local, symbol.file + ":" + symbol.name.split(".").at(-1), symbol);
  }
  const edges: Array<{ source: string; target: string }> = [],
    seen = new Set<string>();
  const scopes = new Map<
    string,
    Map<string, { prefixes: string[]; resolved: Unique }>
  >();
  for (const symbol of symbols.values()) {
    const namespace = symbol.name.slice(0, symbol.name.lastIndexOf(".") + 1);
    let fileScopes = scopes.get(symbol.file);
    if (!fileScopes) scopes.set(symbol.file, (fileScopes = new Map()));
    let context = fileScopes.get(namespace);
    if (!context) {
      const parts = symbol.name.split(".").slice(0, -1);
      fileScopes.set(
        namespace,
        (context = {
          prefixes: parts.map(
            (_, i) => parts.slice(0, parts.length - i).join(".") + ".",
          ),
          resolved: new Map(),
        }),
      );
    }
    const scope = privateNames.get(symbol.file),
      { prefixes, resolved } = context;
    for (const reference of symbol.references) {
      let target = resolved.get(reference);
      if (target === undefined) {
        for (let i = 0; i <= prefixes.length; i++) {
          const name =
              i < prefixes.length ? prefixes[i]! + reference : reference,
            published = publicNames.get(name),
            hidden = scope?.get(name);
          // Two visible groups are ambiguous, including a public/private pair.
          target =
            published === undefined
              ? hidden
              : hidden === undefined
                ? published
                : undefined;
          if (target) break;
        }
        target = target ?? local.get(symbol.file + ":" + reference) ?? null;
        // This cache belongs only to this immutable symbol set and exact file/
        // namespace. Cache resolution, before per-source self-edge suppression.
        resolved.set(reference, target);
      }
      if (target && target.id !== symbol.id) {
        const key = symbol.id + "->" + target.id;
        if (!seen.has(key)) {
          seen.add(key);
          edges.push({ source: symbol.id, target: target.id });
        }
      }
    }
  }
  return edges;
}
