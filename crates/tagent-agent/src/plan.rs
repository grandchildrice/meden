//! The restricted language a plan is written in.
//!
//! The planning model does not call tools directly. It emits a [`Plan`]: a
//! short program of `let` bindings and calls, with no loops, no recursion, no
//! arithmetic and no way to name a tool that is not registered. The
//! interpreter runs it, and because every step is an explicit expression, the
//! provenance of every argument is known before anything happens.
//!
//! A plan is JSON, so a model can emit one:
//!
//! ```json
//! {"steps": [
//!   {"let": {"name": "page",
//!            "value": {"call": {"tool": "fetch",
//!                               "args": {"url": {"str": "https://example.com"}}}}}},
//!   {"do": {"call": {"tool": "summarise",
//!                    "args": {"text": {"field": {"base": {"var": "page"},
//!                                                "key": "body"}}}}}}
//! ]}
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// An expression in a plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expr {
    /// A text literal.
    Str(String),
    /// A whole-number literal.
    Int(i64),
    /// A boolean literal.
    Bool(bool),
    /// The value bound to a name by an earlier `let`.
    Var(String),
    /// A list of expressions.
    List(Vec<Expr>),
    /// A tool call.
    Call {
        /// The tool's name.
        tool: String,
        /// Its arguments, by parameter name.
        #[serde(default)]
        args: BTreeMap<String, Expr>,
    },
    /// A named field of a map.
    Field {
        /// The map.
        base: Box<Expr>,
        /// The field name.
        key: String,
    },
    /// An element of a list.
    Index {
        /// The list.
        base: Box<Expr>,
        /// The position, from zero.
        index: usize,
    },
}

impl Expr {
    /// A call with no arguments.
    pub fn call(tool: &str) -> Self {
        Expr::Call {
            tool: tool.to_owned(),
            args: BTreeMap::new(),
        }
    }

    /// This call with one more argument. Does nothing to a non-call.
    #[must_use]
    pub fn arg(mut self, name: &str, value: Expr) -> Self {
        if let Expr::Call { args, .. } = &mut self {
            args.insert(name.to_owned(), value);
        }
        self
    }

    /// A text literal.
    pub fn str(s: &str) -> Self {
        Expr::Str(s.to_owned())
    }

    /// A variable reference.
    pub fn var(name: &str) -> Self {
        Expr::Var(name.to_owned())
    }

    /// A field access.
    #[must_use]
    pub fn field(self, key: &str) -> Self {
        Expr::Field {
            base: Box::new(self),
            key: key.to_owned(),
        }
    }

    /// An element access.
    #[must_use]
    pub fn at(self, index: usize) -> Self {
        Expr::Index {
            base: Box::new(self),
            index,
        }
    }

    /// Every tool named anywhere in this expression.
    pub fn tools(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_tools(&mut out);
        out
    }

    fn collect_tools<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Expr::Call { tool, args } => {
                out.push(tool);
                for a in args.values() {
                    a.collect_tools(out);
                }
            }
            Expr::List(items) => {
                for i in items {
                    i.collect_tools(out);
                }
            }
            Expr::Field { base, .. } | Expr::Index { base, .. } => base.collect_tools(out),
            Expr::Str(_) | Expr::Int(_) | Expr::Bool(_) | Expr::Var(_) => {}
        }
    }
}

/// One step of a plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stmt {
    /// Evaluate an expression and bind it to a name.
    Let {
        /// The name to bind.
        name: String,
        /// The expression to evaluate.
        value: Expr,
    },
    /// Evaluate an expression for its effect.
    Do(Expr),
}

/// A program the interpreter can run.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// The steps, in order.
    pub steps: Vec<Stmt>,
}

impl Plan {
    /// An empty plan.
    pub fn new() -> Self {
        Plan::default()
    }

    /// This plan with a binding appended.
    #[must_use]
    pub fn bind(mut self, name: &str, value: Expr) -> Self {
        self.steps.push(Stmt::Let {
            name: name.to_owned(),
            value,
        });
        self
    }

    /// This plan with an effect appended.
    #[must_use]
    pub fn then(mut self, value: Expr) -> Self {
        self.steps.push(Stmt::Do(value));
        self
    }

    /// Every tool this plan would call.
    pub fn tools(&self) -> Vec<&str> {
        self.steps
            .iter()
            .flat_map(|s| match s {
                Stmt::Let { value, .. } | Stmt::Do(value) => value.tools(),
            })
            .collect()
    }

    /// Parse a plan from JSON.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Render this plan as JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn plans_round_trip_through_json() {
        let plan = Plan::new()
            .bind(
                "page",
                Expr::call("fetch").arg("url", Expr::str("https://example.com")),
            )
            .then(
                Expr::call("send_email")
                    .arg("to", Expr::str("bob@example.com"))
                    .arg("body", Expr::var("page").field("body")),
            );
        let json = plan.to_json().unwrap_or_default();
        assert_eq!(Plan::from_json(&json).ok(), Some(plan.clone()));
        assert_eq!(plan.tools(), vec!["fetch", "send_email"]);
    }

    #[test]
    fn the_documented_json_shape_parses() {
        let plan = Plan::from_json(
            r#"{"steps":[
                 {"let":{"name":"page","value":{"call":{"tool":"fetch","args":{"url":{"str":"u"}}}}}},
                 {"do":{"call":{"tool":"summarise","args":{"text":{"field":{"base":{"var":"page"},"key":"body"}}}}}}
               ]}"#,
        );
        let plan = plan.unwrap_or_default();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.tools(), vec!["fetch", "summarise"]);
    }

    #[test]
    fn a_call_with_no_arguments_parses() {
        let plan = Plan::from_json(r#"{"steps":[{"do":{"call":{"tool":"now"}}}]}"#);
        assert_eq!(plan.map(|p| p.tools().len()).ok(), Some(1));
    }

    #[test]
    fn expression_builders_nest() {
        let e = Expr::var("xs").at(2).field("name");
        assert_eq!(
            e,
            Expr::Field {
                base: Box::new(Expr::Index {
                    base: Box::new(Expr::var("xs")),
                    index: 2
                }),
                key: "name".to_owned()
            }
        );
        // `arg` on a non-call is a no-op rather than a panic.
        assert_eq!(Expr::str("x").arg("a", Expr::Int(1)), Expr::str("x"));
    }
}
