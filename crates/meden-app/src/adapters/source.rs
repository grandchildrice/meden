//! Shared pure source navigation and explanation preview; no elaboration or proof.
use crate::navigation::collection::Indexed;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeSet;

/// Lexical navigation formats; source readability is independent of support.
pub fn supports(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("lean" | "rs")
    )
}

/// Cache identity of the actual lexical implementation, independent of its crate path.
pub fn identity() -> String {
    crate::atlas::digest(include_bytes!("source.rs"))
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub line: usize,
    pub kind: String,
    pub declaration: String,
    pub private: bool,
    pub references: Vec<String>,
    pub semantic: Option<crate::explanation::Explanation>,
    pub main_theorem: bool,
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

// Counts, import names and symbols share one complete lexical observation.
// No elaboration or proof status is inferred.
pub fn index(source: &str, lean: bool) -> Indexed<Symbol> {
    let clean = mask(source, lean);
    let mut declarations = 0usize;
    let mut theorems = 0usize;
    let mut imports = Vec::new();
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
        if declaration {
            declarations = declarations.saturating_add(1);
            if matches!(kind, "theorem" | "lemma") {
                theorems = theorems.saturating_add(1);
            }
        } else if lean && kind == "import" {
            imports.extend(words.by_ref().map(str::to_owned));
        }
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
                current = Some(Symbol {
                    name: qualified_name(&namespaces, local),
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
                    references.insert(if token.contains("::") {
                        Cow::Owned(token.replace("::", "."))
                    } else {
                        Cow::Borrowed(token)
                    });
                }
            }
        }
    }
    finish_symbol(current, &mut references, &mut result);
    Indexed {
        declarations,
        theorems,
        imports,
        symbols: result,
    }
}

fn qualified_name(namespaces: &[String], local: &str) -> String {
    let namespace = namespaces
        .iter()
        .filter(|x| !x.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(".");
    if namespace.is_empty() {
        local.into()
    } else {
        format!("{namespace}.{local}")
    }
}

fn finish_symbol(
    current: Option<Symbol>,
    references: &mut BTreeSet<Cow<'_, str>>,
    result: &mut Vec<Symbol>,
) {
    if let Some(mut symbol) = current {
        symbol.references = std::mem::take(references)
            .into_iter()
            .map(Cow::into_owned)
            .collect();
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
#[path = "source/tests.rs"]
mod tests;
