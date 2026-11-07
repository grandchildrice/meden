//! Reading and writing the plan language.
//!
//! The shape of this syntax is chosen for the models it has to come out of.
//! An early version wrapped every literal (`{"str": "x"}`) and nested every
//! call twice (`{"call": {"tool": ..., "args": ...}}`), which put a simple
//! "fetch, summarise, send" plan seven levels deep. A capable model wrote
//! that plan correctly and dropped one closing brace, and a correct plan was
//! thrown away over its punctuation.
//!
//! So the surface forms are as flat as they can be while staying
//! unambiguous:
//!
//! | written | means |
//! | --- | --- |
//! | `"text"`, `42`, `true` | a literal |
//! | `"$page"` | the value bound to `page` |
//! | `"$page.body"`, `"$items.0"` | a field or element of it |
//! | `[EXPR, ...]` | a list |
//! | `{"call": "tool", "args": {...}}` | a tool call |
//! | `{"ask": "question", "text": EXPR}` | read untrusted text |
//! | `{"str": "$literal"}` | text that really does start with `$` |
//!
//! The older nested forms still parse, so nothing that worked stops working.

use std::collections::BTreeMap;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};

use crate::plan::{Expr, Stmt};

/// The sigil that marks a reference rather than a literal.
pub const REFERENCE_SIGIL: char = '$';

/// Split `name.field.0` into its parts.
fn path_parts(path: &str) -> Option<(String, Vec<String>)> {
    let mut parts = path.split('.');
    let head = parts.next()?;
    if head.is_empty() {
        return None;
    }
    Ok::<_, ()>((head.to_owned(), parts.map(str::to_owned).collect())).ok()
}

/// Build the expression a `$path` reference stands for.
fn reference(path: &str) -> Option<Expr> {
    let (head, rest) = path_parts(path)?;
    let mut expr = Expr::Var(head);
    for part in rest {
        if part.is_empty() {
            return None;
        }
        expr = match part.parse::<usize>() {
            Ok(index) => Expr::Index {
                base: Box::new(expr),
                index,
            },
            Err(_) => Expr::Field {
                base: Box::new(expr),
                key: part,
            },
        };
    }
    Some(expr)
}

/// The `$path` form of an expression, when it has one.
pub fn as_reference(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Var(name) => (!name.contains('.')).then(|| format!("{REFERENCE_SIGIL}{name}")),
        Expr::Field { base, key } => {
            if key.contains('.') || key.is_empty() || key.parse::<usize>().is_ok() {
                return None;
            }
            Some(format!("{}.{key}", as_reference(base)?))
        }
        Expr::Index { base, index } => Some(format!("{}.{index}", as_reference(base)?)),
        _ => None,
    }
}

/// The object forms an expression may be written in.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ExprObject {
    /// `{"call": "tool", "args": {...}}`
    FlatCall {
        call: String,
        #[serde(default)]
        args: BTreeMap<String, Expr>,
    },
    /// `{"ask": "question", "text": EXPR}`
    FlatAsk { ask: String, text: Box<Expr> },
    /// The older nested forms.
    Nested(Nested),
}

/// The nested forms, kept so earlier plans still parse.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Nested {
    Str(String),
    Int(i64),
    Bool(bool),
    Var(String),
    List(Vec<Expr>),
    Call {
        tool: String,
        #[serde(default)]
        args: BTreeMap<String, Expr>,
    },
    Field {
        base: Box<Expr>,
        key: String,
    },
    Ask {
        question: String,
        text: Box<Expr>,
    },
    Index {
        base: Box<Expr>,
        index: usize,
    },
}

impl From<Nested> for Expr {
    fn from(n: Nested) -> Self {
        match n {
            Nested::Str(s) => Expr::Str(s),
            Nested::Int(i) => Expr::Int(i),
            Nested::Bool(b) => Expr::Bool(b),
            Nested::Var(v) => Expr::Var(v),
            Nested::List(l) => Expr::List(l),
            Nested::Call { tool, args } => Expr::Call { tool, args },
            Nested::Field { base, key } => Expr::Field { base, key },
            Nested::Ask { question, text } => Expr::Ask { question, text },
            Nested::Index { base, index } => Expr::Index { base, index },
        }
    }
}

impl Serialize for Expr {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // A reference goes out in its short form.
        if let Some(path) = as_reference(self) {
            return s.serialize_str(&path);
        }
        match self {
            Expr::Str(v) => {
                if v.starts_with(REFERENCE_SIGIL) {
                    // Would otherwise read back as a reference.
                    Nested::Str(v.clone()).serialize(s)
                } else {
                    s.serialize_str(v)
                }
            }
            Expr::Int(v) => s.serialize_i64(*v),
            Expr::Bool(v) => s.serialize_bool(*v),
            Expr::List(items) => items.serialize(s),
            Expr::Call { tool, args } => ExprObject::FlatCall {
                call: tool.clone(),
                args: args.clone(),
            }
            .serialize(s),
            Expr::Ask { question, text } => ExprObject::FlatAsk {
                ask: question.clone(),
                text: text.clone(),
            }
            .serialize(s),
            // A field or index whose base is not a plain reference.
            Expr::Var(v) => Nested::Var(v.clone()).serialize(s),
            Expr::Field { base, key } => Nested::Field {
                base: base.clone(),
                key: key.clone(),
            }
            .serialize(s),
            Expr::Index { base, index } => Nested::Index {
                base: base.clone(),
                index: *index,
            }
            .serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for Expr {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Bool(bool),
            Int(i64),
            Str(String),
            List(Vec<Expr>),
            Object(ExprObject),
        }
        Ok(match Repr::deserialize(d)? {
            Repr::Bool(b) => Expr::Bool(b),
            Repr::Int(i) => Expr::Int(i),
            Repr::Str(s) => match s.strip_prefix(REFERENCE_SIGIL) {
                Some(path) => reference(path)
                    .ok_or_else(|| D::Error::custom(format!("{s:?} is not a usable reference")))?,
                None => Expr::Str(s),
            },
            Repr::List(l) => Expr::List(l),
            Repr::Object(ExprObject::FlatCall { call, args }) => Expr::Call { tool: call, args },
            Repr::Object(ExprObject::FlatAsk { ask, text }) => Expr::Ask {
                question: ask,
                text,
            },
            Repr::Object(ExprObject::Nested(n)) => n.into(),
        })
    }
}

/// The forms a step may be written in.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum StmtRepr {
    /// `{"let": "name", "value": EXPR}`
    FlatLet {
        #[serde(rename = "let")]
        name: String,
        value: Expr,
    },
    /// `{"let": {"name": "x", "value": EXPR}}`
    NestedLet {
        #[serde(rename = "let")]
        binding: Binding,
    },
    /// `{"do": EXPR}`
    Do {
        #[serde(rename = "do")]
        value: Expr,
    },
    /// A bare EXPR, meaning the same as `{"do": EXPR}`.
    ///
    /// Models drop the wrapper, and a step written `{"call": ...}` means
    /// exactly what `{"do": {"call": ...}}` means. Accepting it changes no
    /// rule: the expression is the same expression, checked by the same gate.
    /// Refusing it would only throw away correct plans over punctuation.
    ///
    /// It must come last: an untagged enum tries its variants in order, and
    /// this one would otherwise swallow the `let` and `do` forms.
    Bare(Expr),
}

#[derive(Serialize, Deserialize)]
struct Binding {
    name: String,
    value: Expr,
}

impl Serialize for Stmt {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Stmt::Let { name, value } => StmtRepr::FlatLet {
                name: name.clone(),
                value: value.clone(),
            }
            .serialize(s),
            Stmt::Do(value) => StmtRepr::Do {
                value: value.clone(),
            }
            .serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for Stmt {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match StmtRepr::deserialize(d)? {
            StmtRepr::FlatLet { name, value } => Stmt::Let { name, value },
            StmtRepr::NestedLet { binding } => Stmt::Let {
                name: binding.name,
                value: binding.value,
            },
            StmtRepr::Do { value } | StmtRepr::Bare(value) => Stmt::Do(value),
        })
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use crate::plan::Plan;

    fn parse(json: &str) -> Plan {
        match Plan::from_json(json) {
            Ok(p) => p,
            Err(e) => panic!("should parse: {e}\n{json}"),
        }
    }

    #[test]
    fn the_flat_forms_parse() {
        let plan = parse(
            r#"{"steps":[
                 {"let":"page","value":{"call":"fetch","args":{"url":"https://example.com"}}},
                 {"do":{"call":"send_email","args":{
                    "to":"bob@example.com",
                    "body":{"ask":"summarise this","text":"$page.body"}}}}
               ]}"#,
        );
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.tools(), vec!["fetch", "send_email"]);
    }

    #[test]
    fn the_nested_forms_still_parse() {
        let flat = parse(r#"{"steps":[{"let":"x","value":{"call":"f","args":{"a":"b"}}}]}"#);
        let nested = parse(
            r#"{"steps":[{"let":{"name":"x","value":{"call":{"tool":"f","args":{"a":{"str":"b"}}}}}}]}"#,
        );
        assert_eq!(flat, nested);
    }

    #[test]
    fn references_cover_fields_and_elements() {
        let plan = parse(r#"{"steps":[{"do":"$page.items.2.name"}]}"#);
        let expected = Expr::Var("page".into()).field("items").at(2).field("name");
        assert_eq!(plan.steps.first(), Some(&Stmt::Do(expected)));
    }

    #[test]
    fn a_reference_round_trips_in_its_short_form() {
        let expr = Expr::Var("page".into()).field("body");
        assert_eq!(as_reference(&expr).as_deref(), Some("$page.body"));
        let json = serde_json::to_string(&expr).unwrap_or_default();
        assert_eq!(json, "\"$page.body\"");
        assert_eq!(serde_json::from_str::<Expr>(&json).ok(), Some(expr));
    }

    #[test]
    fn text_that_starts_with_the_sigil_survives_a_round_trip() {
        let expr = Expr::Str("$5.00 please".into());
        let json = serde_json::to_string(&expr).unwrap_or_default();
        assert!(json.contains("\"str\""), "{json}");
        assert_eq!(serde_json::from_str::<Expr>(&json).ok(), Some(expr));
    }

    #[test]
    fn a_malformed_reference_is_an_error_not_a_literal() {
        assert!(serde_json::from_str::<Expr>("\"$\"").is_err());
        assert!(serde_json::from_str::<Expr>("\"$a..b\"").is_err());
    }

    #[test]
    fn plain_literals_need_no_wrapper() {
        let plan =
            parse(r#"{"steps":[{"do":{"call":"f","args":{"s":"x","n":2,"b":true,"l":[1,2]}}}]}"#);
        let Some(Stmt::Do(Expr::Call { args, .. })) = plan.steps.first() else {
            panic!("expected a call")
        };
        assert_eq!(args.get("s"), Some(&Expr::Str("x".into())));
        assert_eq!(args.get("n"), Some(&Expr::Int(2)));
        assert_eq!(args.get("b"), Some(&Expr::Bool(true)));
        assert_eq!(
            args.get("l"),
            Some(&Expr::List(vec![Expr::Int(1), Expr::Int(2)]))
        );
    }

    #[test]
    fn a_whole_plan_round_trips_through_the_flat_forms() {
        let plan = Plan::new()
            .bind("page", Expr::call("fetch").arg("url", Expr::str("u")))
            .then(
                Expr::call("send_email")
                    .arg("to", Expr::str("bob@example.com"))
                    .arg(
                        "body",
                        Expr::var("page").field("body").ask("summarise this"),
                    ),
            );
        let json = plan.to_json().unwrap_or_default();
        assert!(json.contains("\"$page.body\""), "{json}");
        assert!(!json.contains("\"tool\""), "still nesting calls: {json}");
        assert_eq!(Plan::from_json(&json).ok(), Some(plan));
    }
}

#[cfg(test)]
mod leniency_tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use crate::plan::{Expr, Plan, Stmt};

    #[test]
    fn a_step_may_omit_the_do_wrapper() {
        let plan =
            Plan::from_json(r#"{"steps":[{"call":"fetch","args":{"url":"u"}}],"continues":true}"#);
        let plan = match plan {
            Ok(p) => p,
            Err(e) => panic!("a bare expression should parse as a step: {e}"),
        };
        assert!(plan.continues, "continues was dropped");
        assert_eq!(
            plan.steps.first(),
            Some(&Stmt::Do(Expr::call("fetch").arg("url", Expr::str("u"))))
        );
    }

    #[test]
    fn the_wrapped_forms_still_win() {
        let plan = Plan::from_json(r#"{"steps":[{"let":"x","value":"hi"}]}"#).unwrap_or_default();
        assert_eq!(
            plan.steps.first(),
            Some(&Stmt::Let {
                name: "x".to_owned(),
                value: Expr::str("hi"),
            }),
            "a let step was swallowed by the bare form"
        );
    }
}
