//! Shared pure source navigation and explanation preview; no elaboration or proof.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Symbol {
    pub name: String,
    pub line: usize,
    pub kind: String,
    pub declaration: String,
    pub private: bool,
    pub references: Vec<String>,
    pub semantic: Option<crate::explanation::Explanation>,
    pub main_theorem: bool,
}
// Mask nested comments and string literals before counting declarations or
// reading Lean import lines. Names and dependencies are lexical candidates.
pub(crate) fn headers(source: &str, lean: bool) -> (usize, usize, Vec<String>) {
    let text = mask(source, lean);
    let mut declarations = 0usize;
    let mut theorems = 0usize;
    let mut imports = Vec::new();
    for line in text.lines() {
        let mut words = line.split_whitespace().peekable();
        while words.peek().is_some_and(|word| {
            matches!(
                *word,
                "public"
                    | "private"
                    | "protected"
                    | "noncomputable"
                    | "unsafe"
                    | "partial"
                    | "async"
            ) || word.starts_with("pub")
        }) {
            words.next();
        }
        let Some(kind) = words.next() else {
            continue;
        };
        if lean && kind == "import" {
            imports.extend(words.map(str::to_owned));
        }
        if matches!(
            kind,
            "theorem"
                | "lemma"
                | "def"
                | "structure"
                | "class"
                | "inductive"
                | "abbrev"
                | "opaque"
                | "axiom"
                | "constant"
                | "fn"
                | "struct"
                | "enum"
                | "trait"
                | "type"
        ) {
            declarations = declarations.saturating_add(1);
            if matches!(kind, "theorem" | "lemma") {
                theorems = theorems.saturating_add(1);
            }
        }
    }
    (declarations, theorems, imports)
}

fn is_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "theorem"
            | "lemma"
            | "def"
            | "structure"
            | "class"
            | "inductive"
            | "abbrev"
            | "opaque"
            | "axiom"
            | "constant"
            | "fn"
            | "struct"
            | "enum"
            | "trait"
            | "type"
    )
}

// Precomputed lexical navigation only. No elaboration or proof status is inferred.
pub(crate) fn symbols(source: &str, lean: bool) -> Vec<Symbol> {
    let clean = mask(source, lean);
    let mut result = Vec::new();
    let mut current: Option<Symbol> = None;
    let mut references = BTreeSet::new();
    let mut namespaces = Vec::<String>::new();
    for (index, line) in clean.lines().enumerate() {
        let mut words = line.split_whitespace().peekable();
        while words.peek().is_some_and(|w| {
            matches!(
                *w,
                "public"
                    | "private"
                    | "protected"
                    | "noncomputable"
                    | "unsafe"
                    | "partial"
                    | "async"
            ) || w.starts_with("pub")
        }) {
            words.next();
        }
        let kind = words.next().unwrap_or("");
        let command = lean && matches!(kind, "namespace" | "section" | "end");
        let declaration = is_declaration(kind);
        if command || declaration {
            finish_symbol(current.take(), &mut references, &mut result);
        }
        if command {
            match kind {
                "end" => {
                    namespaces.pop();
                }
                "namespace" => namespaces.push(words.next().unwrap_or("").into()),
                _ => namespaces.push(String::new()),
            }
        } else if declaration {
            let local = words.next().unwrap_or("").trim_end_matches(':');
            if !local.is_empty() {
                let namespace = namespaces
                    .iter()
                    .filter(|x| !x.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(".");
                current = Some(Symbol {
                    name: if namespace.is_empty() {
                        local.into()
                    } else {
                        format!("{namespace}.{local}")
                    },
                    line: index.saturating_add(1),
                    declaration: kind.into(),
                    kind: (if matches!(kind, "theorem" | "lemma") {
                        "theorem"
                    } else if matches!(kind, "axiom" | "constant") {
                        "axiom"
                    } else {
                        "definition"
                    })
                    .into(),
                    private: line.split_whitespace().any(|w| w == "private"),
                    references: Vec::new(),
                    semantic: None,
                    main_theorem: false,
                });
            }
        }
        if current.is_some() {
            for token in
                line.split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '\'' | '.' | ':')))
            {
                let token = token.trim_matches(':');
                if token
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
                {
                    references.insert(token.replace("::", "."));
                }
            }
        }
    }
    finish_symbol(current, &mut references, &mut result);
    result
}

fn finish_symbol(
    current: Option<Symbol>,
    references: &mut BTreeSet<String>,
    result: &mut Vec<Symbol>,
) {
    if let Some(mut symbol) = current {
        symbol.references = std::mem::take(references).into_iter().collect();
        result.push(symbol);
    }
}

fn mask(source: &str, lean: bool) -> String {
    let mut chars = source.chars().peekable();
    let mut output = String::new();
    let mut depth = 0usize;
    let mut line = false;
    let mut quoted = false;
    while let Some(c) = chars.next() {
        let next = chars.peek().copied();
        if line {
            output.push(if c == '\n' { '\n' } else { ' ' });
            if c == '\n' {
                line = false;
            }
        } else if depth > 0 {
            if (c, next)
                == if lean {
                    ('/', Some('-'))
                } else {
                    ('/', Some('*'))
                }
            {
                depth = depth.saturating_add(1);
                chars.next();
                output.push_str("  ");
            } else if (c, next)
                == if lean {
                    ('-', Some('/'))
                } else {
                    ('*', Some('/'))
                }
            {
                depth = depth.saturating_sub(1);
                chars.next();
                output.push_str("  ");
            } else {
                output.push(if c == '\n' { '\n' } else { ' ' });
            }
        } else if quoted {
            output.push(if c == '\n' { '\n' } else { ' ' });
            if c == '\\' {
                if let Some(escaped) = chars.next() {
                    output.push(if escaped == '\n' { '\n' } else { ' ' });
                }
            } else if c == '"' {
                quoted = false;
            }
        } else if (c, next)
            == if lean {
                ('-', Some('-'))
            } else {
                ('/', Some('/'))
            }
        {
            line = true;
            chars.next();
            output.push_str("  ");
        } else if (c, next)
            == if lean {
                ('/', Some('-'))
            } else {
                ('/', Some('*'))
            }
        {
            depth = 1;
            chars.next();
            output.push_str("  ");
        } else if c == '"' {
            quoted = true;
            output.push(' ');
        } else {
            output.push(c);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_navigation_does_not_count_imports_or_theorems_in_comments_and_literals() {
        let source = "/- import Fake\n/- theorem fake : False -/\n-/\npublic import Real.Module\n-- theorem fake : False\ndef text := \"theorem fake : False\"\npublic theorem actual (n : Nat) : n = n := rfl\n";
        assert_eq!(
            headers(source, true),
            (2, 1, vec!["Real.Module".to_owned()])
        );
    }
    #[test]
    fn cached_symbols_keep_original_lines_names_and_masked_reference_candidates() {
        let source = "namespace Demo\nprivate def keep : Nat := 1\n/- theorem fake : False := keep -/\npublic theorem actual : keep = keep := rfl\nend Demo\n";
        let found = symbols(source, true);
        assert_eq!(found.len(), 2);
        assert!(found
            .first()
            .is_some_and(|symbol| symbol.name == "Demo.keep" && symbol.private));
        assert!(found
            .get(1)
            .is_some_and(|symbol| symbol.name == "Demo.actual"
                && symbol.declaration == "theorem"
                && symbol.line == 4
                && symbol.references.contains(&"keep".into())
                && !symbol.references.contains(&"fake".into())));
    }
    #[test]
    fn declaration_keywords_distinguish_lemmas_from_theorems_and_data() {
        let found = symbols("lemma helper : True := True.intro\ntheorem result : True := helper\nstructure Record where\n  field : Nat\n", true);
        assert_eq!(
            found
                .iter()
                .map(|item| item.declaration.as_str())
                .collect::<Vec<_>>(),
            vec!["lemma", "theorem", "structure"]
        );
    }
}
