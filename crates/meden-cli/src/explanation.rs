//! Versioned presentation observations. Source previews preserve logical structure;
//! elaborated observations come from the fixed Lean/Plausible build stage.
use serde::{Deserialize, Serialize};
pub(crate) mod cache;
pub(crate) mod source;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Explanation {
    pub version: u8,
    pub origin: String,
    pub sentence: String,
    pub refutation: String,
    pub checker: Check,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Check {
    pub engine: String,
    pub status: String,
    pub candidates: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_inst: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_retries: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl Explanation {
    pub fn valid(&self) -> bool {
        self.version == 1
            && matches!(self.origin.as_str(), "source" | "elaborated")
            && !self.sentence.is_empty()
            && self.sentence.len() <= 65_536
            && self.refutation.len() <= 65_536
            && matches!(self.checker.engine.as_str(), "Plausible" | "none")
            && matches!(
                self.checker.status.as_str(),
                "counterexample"
                    | "predicate_false_example"
                    | "not_found"
                    | "gave_up"
                    | "budget_exhausted"
                    | "unavailable"
                    | "unsupported"
                    | "not_proposition"
                    | "not_run"
            )
            && self.checker.candidates.len() <= 64
            && self.checker.candidates.iter().all(|s| s.len() <= 8192)
            && self
                .checker
                .diagnostic
                .as_ref()
                .is_none_or(|s| s.len() <= 65_536)
    }
}

pub(crate) fn source(kind: &str, source: &str) -> Explanation {
    let source = if matches!(kind, "theorem" | "lemma" | "axiom" | "constant") {
        source.split_once(":=").map_or(source, |(ty, _)| ty)
    } else {
        source
    };
    let tokens = tokenize(source);
    let parsed = tokens
        .and_then(|tokens| declaration(kind, &tokens))
        .filter(|s| s.len() <= 60_000);
    let (sentence, diagnostic) = match parsed {
        Some(sentence) => (sentence, "Source-structure preview. Imports, notation resolution, implicit section parameters and type checking require compilation."),
        None => ("Meaning is unavailable for this source form until compilation.".into(), "This source form is outside the preview grammar. Compile to interpret its elaborated type and defining expression."),
    };
    let refutation = if matches!(kind, "theorem" | "lemma" | "axiom" | "constant") {
        format!("A refutation would satisfy the assumptions while making this statement false: {sentence}")
    } else {
        "A definition specifies an object rather than asserting a universal theorem. Review its inputs, output and defining expression.".into()
    };
    Explanation {
        version: 1,
        origin: "source".into(),
        sentence,
        refutation,
        checker: Check {
            engine: "none".into(),
            status: "not_run".into(),
            candidates: vec![],
            diagnostic: Some(diagnostic.into()),
            seed: None,
            num_inst: None,
            num_retries: None,
            max_size: None,
            mode: None,
        },
    }
}

fn tokenize(source: &str) -> Option<Vec<String>> {
    let mut result = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        let mut token = c.to_string();
        if c.is_alphanumeric() || c == '_' || c == '«' {
            while let Some(next) = chars.peek() {
                if next.is_alphanumeric() || matches!(next, '_' | '\'' | '.' | '»') {
                    token.push(chars.next()?);
                } else {
                    break;
                }
            }
        } else if matches!(
            (c, chars.peek()),
            (':' | '>' | '!', Some('=')) | ('-' | '=', Some('>')) | ('<', Some('=' | '>'))
        ) {
            token.push(chars.next()?);
        }
        result.push(token);
        if result.len() > 2048 {
            return None;
        }
    }
    Some(result)
}

fn declaration(kind: &str, tokens: &[String]) -> Option<String> {
    let start = tokens.iter().position(|t| t == kind)?.checked_add(2)?;
    let mut parser = Parser {
        tokens,
        position: start,
        fuel: 128,
    };
    let mut binders = Vec::new();
    while matches!(parser.peek(), Some("(" | "{" | "[")) {
        let close = match parser.next()? {
            "(" => ")",
            "{" => "}",
            _ => "]",
        };
        let begin = parser.position;
        let end = parser.matching_end(close)?;
        let (names, ty) = binder(tokens.get(begin..end)?)?;
        binders.push((names, ty));
        parser.position = end.checked_add(1)?;
    }
    let proposition = matches!(kind, "theorem" | "lemma" | "axiom" | "constant");
    if proposition {
        parser.take(":")?;
        let mut statement = parser.expr(0)?;
        if statement.starts_with('“') && statement.ends_with('”') {
            statement.push_str(" holds");
        }
        if !matches!(parser.peek(), None | Some(":=" | "where")) {
            return None;
        }
        for (names, ty) in binders.into_iter().rev() {
            statement = if names.iter().all(|name| name.starts_with('h')) && !is_type(&ty) {
                format!("if {ty}, then {statement}")
            } else {
                format!("for every {} that is {ty}, {statement}", names.join(", "))
            };
        }
        Some(statement)
    } else {
        let body_start = tokens
            .iter()
            .enumerate()
            .skip(parser.position)
            .find(|(_, t)| t.as_str() == ":=")?
            .0
            .checked_add(1)?;
        parser.position = body_start;
        let mut body = parser.expr(0)?;
        if parser.peek().is_some() {
            return None;
        }
        for (names, _) in binders.into_iter().rev() {
            body = format!("the function taking {} to {body}", names.join(", "));
        }
        Some(format!("Defined as {body}"))
    }
}

fn binder(tokens: &[String]) -> Option<(Vec<String>, String)> {
    let colon = tokens.iter().position(|t| t == ":")?;
    let names = tokens.get(..colon)?.to_vec();
    if names.is_empty() {
        return None;
    }
    let mut parser = Parser {
        tokens: tokens.get(colon.checked_add(1)?..)?,
        position: 0,
        fuel: 128,
    };
    let ty = parser.expr(0)?;
    (parser.peek().is_none()).then_some((names, ty))
}

fn is_type(text: &str) -> bool {
    matches!(
        text,
        "a natural number"
            | "an integer"
            | "a boolean"
            | "a real number"
            | "a complex number"
            | "a type"
            | "a proposition"
    ) || text.starts_with('“')
}

struct Parser<'a> {
    tokens: &'a [String],
    position: usize,
    fuel: usize,
}
impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.position).map(String::as_str)
    }
    fn next(&mut self) -> Option<&'a str> {
        let token = self.peek()?;
        self.position = self.position.checked_add(1)?;
        Some(token)
    }
    fn take(&mut self, token: &str) -> Option<()> {
        (self.next()? == token).then_some(())
    }
    fn matching_end(&self, close: &str) -> Option<usize> {
        let mut depth = 0usize;
        for (index, token) in self.tokens.iter().enumerate().skip(self.position) {
            if token == close && depth == 0 {
                return Some(index);
            }
            match token.as_str() {
                "(" | "[" | "{" => depth = depth.checked_add(1)?,
                ")" | "]" | "}" => depth = depth.checked_sub(1)?,
                _ => {}
            }
        }
        None
    }
    fn expr(&mut self, minimum: u8) -> Option<String> {
        self.fuel = self.fuel.checked_sub(1)?;
        let mut left = match self.next()? {
            "(" => {
                let value = self.expr(0)?;
                self.take(")")?;
                value
            }
            "∀" | "forall" | "∃" => {
                let exists = self.tokens.get(self.position.checked_sub(1)?)?.as_str() == "∃";
                let begin = self.position;
                let comma = self
                    .tokens
                    .get(begin..)?
                    .iter()
                    .position(|t| t == ",")?
                    .checked_add(begin)?;
                let (names, ty) = binder(self.tokens.get(begin..comma)?)?;
                self.position = comma.checked_add(1)?;
                let body = self.expr(0)?;
                format!(
                    "{} {} that is {ty}, {body}",
                    if exists { "there exists" } else { "for every" },
                    names.join(", ")
                )
            }
            "¬" => format!("it is not the case that {}", self.expr(35)?),
            token if token.chars().all(char::is_numeric) => token.into(),
            "Nat" | "ℕ" => "a natural number".into(),
            "Int" | "ℤ" => "an integer".into(),
            "Real" | "ℝ" => "a real number".into(),
            "Complex" | "ℂ" => "a complex number".into(),
            "Bool" => "a boolean".into(),
            "Type" => "a type".into(),
            "Prop" => "a proposition".into(),
            "True" | "true" => "true".into(),
            "False" | "false" => "false".into(),
            token if identifier(token) => {
                if token.chars().count() <= 3 && !token.contains('.') {
                    token.into()
                } else {
                    format!("“{}”", human_name(token))
                }
            }
            _ => return None,
        };
        while let Some(token) = self.peek() {
            let binary = match token {
                "→" | "->" => Some((5, "implies")),
                "↔" => Some((6, "if and only if")),
                "∨" => Some((10, "or")),
                "∧" => Some((15, "and")),
                "=" => Some((20, "equals")),
                "≠" | "!=" => Some((20, "differs from")),
                "<" => Some((20, "is less than")),
                "≤" | "<=" => Some((20, "is at most")),
                ">" => Some((20, "is greater than")),
                "≥" | ">=" => Some((20, "is at least")),
                "∈" => Some((20, "belongs to")),
                "+" => Some((30, "plus")),
                "-" => Some((30, "minus")),
                "*" => Some((40, "times")),
                "/" => Some((40, "divided by")),
                "^" => Some((50, "raised to")),
                _ => None,
            };
            if let Some((priority, verb)) = binary {
                if priority < minimum {
                    break;
                }
                self.next();
                let right = self.expr(if priority == 5 {
                    priority
                } else {
                    priority.saturating_add(1)
                })?;
                left = format!("({left}) {verb} ({right})");
            } else if (identifier(token) || token == "(" || token.chars().all(char::is_numeric))
                && minimum <= 60
            {
                let argument = self.expr(61)?;
                left = format!("{left} applied to {argument}");
            } else {
                break;
            }
        }
        Some(left)
    }
}

fn identifier(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || matches!(c, '_' | '«'))
        && !matches!(token, "where" | "by" | "fun" | "match" | "with" | "end")
}

fn human_name(name: &str) -> String {
    let mut result = String::new();
    let mut previous = ' ';
    for c in name.chars() {
        if c == '_' || c == '.' {
            result.push(' ');
        } else {
            if c.is_uppercase() && previous.is_lowercase() {
                result.push(' ');
            }
            result.extend(c.to_lowercase());
        }
        previous = c;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_quantification_assumptions_and_definition_bodies() {
        let text = source(
            "theorem",
            "theorem bound (n : Nat) (h : n > 0) : n ≥ 1 := by omega",
        );
        assert_eq!(text.sentence, "for every n that is a natural number, if (n) is greater than (0), then (n) is at least (1)");
        assert_eq!(text.checker.status, "not_run");
        assert_eq!(
            source("def", "def twice (n : Nat) := n + n").sentence,
            "Defined as the function taking n to (n) plus (n)"
        );
        assert!(source(
            "theorem",
            "theorem exists : ∃ n : Nat, n = 1 := by exact ⟨1,rfl⟩"
        )
        .sentence
        .starts_with("there exists n"));
        assert!(source(
            "theorem",
            "theorem unsupported : match true with | true => True | false => False := by simp"
        )
        .sentence
        .contains("unavailable"));
    }
}
