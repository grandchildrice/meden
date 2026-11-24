export type ReferenceSymbol = {
  readonly id: string;
  readonly file: string;
  readonly name: string;
  readonly private: boolean;
  readonly references: readonly string[];
};
type Unique = Map<string, string | null>;
type Resolution = {
  file: string;
  namespace: string;
  targets: readonly string[];
};
const insert = (index: Unique, key: string, id: string) => {
  index.set(key, index.has(key) ? null : id);
};

/** One immutable candidate universe. The scene owner replaces it when an
 * ordered declaration identity/name/visibility changes. It retains only target
 * IDs, not old source/semantic records. Cached source readings have weak keys;
 * name-lookup tables and namespace contexts live only for one call.
 * These lexical display edges carry no compiler or proof authority.
 */
export class ReferenceIndex {
  #publicNames: Unique = new Map();
  #privateNames = new Map<string, Unique>();
  #local: Unique = new Map();
  #readings = new WeakMap<readonly string[], Resolution>();

  constructor(symbols: ReadonlyMap<string, ReferenceSymbol>) {
    for (const symbol of symbols.values()) {
      let index = this.#publicNames;
      if (symbol.private) {
        index = this.#privateNames.get(symbol.file) || new Map();
        this.#privateNames.set(symbol.file, index);
      }
      insert(index, symbol.name, symbol.id);
      insert(
        this.#local,
        symbol.file + ":" + symbol.name.split(".").at(-1),
        symbol.id,
      );
    }
  }

  /** Resolve current sources in their original Map order. Sources may acquire
   * new references while the candidate universe stays fixed. Suppression stays
   * global and ordered, including the original delimiter-key collision rule.
   */
  edges(symbols: ReadonlyMap<string, ReferenceSymbol>) {
    const edges: Array<{ source: string; target: string }> = [],
      seen = new Set<string>();
    const scopes = new Map<
      string,
      Map<string, { prefixes: string[]; resolved: Unique }>
    >();
    for (const symbol of symbols.values()) {
      if (!symbol.references.length) continue;
      const namespace = symbol.name.slice(0, symbol.name.lastIndexOf(".") + 1);
      let reading = this.#readings.get(symbol.references);
      if (reading?.file !== symbol.file || reading.namespace !== namespace) {
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
        const scope = this.#privateNames.get(symbol.file),
          { prefixes, resolved } = context,
          targets = new Set<string>();
        for (const reference of symbol.references) {
          let target = resolved.get(reference);
          if (target === undefined) {
            for (let i = 0; i <= prefixes.length; i++) {
              const name =
                  i < prefixes.length ? prefixes[i]! + reference : reference,
                published = this.#publicNames.get(name),
                hidden = scope?.get(name);
              // Two visible groups are ambiguous, including a public/private pair.
              target =
                published === undefined
                  ? hidden
                  : hidden === undefined
                    ? published
                    : undefined;
              if (target !== undefined && target !== null) break;
            }
            target =
              target ?? this.#local.get(symbol.file + ":" + reference) ?? null;
            // Resolve before source-specific self-edge suppression.
            resolved.set(reference, target);
          }
          if (target !== null) targets.add(target);
        }
        reading = { file: symbol.file, namespace, targets: [...targets] };
        this.#readings.set(symbol.references, reading);
      }
      for (const target of reading.targets) {
        if (target === symbol.id) continue;
        const key = symbol.id + "->" + target;
        if (!seen.has(key)) {
          seen.add(key);
          edges.push({ source: symbol.id, target });
        }
      }
    }
    return edges;
  }
}
