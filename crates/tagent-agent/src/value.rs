//! Labelled values.
//!
//! Every value the interpreter handles carries a [`Label`]. Containers label
//! their elements individually, so one field of a record can be more tainted
//! than its siblings; reading a field joins the container's own label onto
//! the field's, because knowing which container it came from is itself
//! information.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::label::Label;

/// The data part of a value, without its label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "of")]
pub enum Data {
    /// No value.
    Unit,
    /// A boolean.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// Text.
    Str(String),
    /// An ordered sequence, each element labelled.
    List(Vec<Value>),
    /// A record of named fields, each labelled.
    Map(BTreeMap<String, Value>),
}

impl Data {
    /// The name of this kind of data, for error messages.
    pub const fn kind(&self) -> &'static str {
        match self {
            Data::Unit => "unit",
            Data::Bool(_) => "bool",
            Data::Int(_) => "int",
            Data::Str(_) => "str",
            Data::List(_) => "list",
            Data::Map(_) => "map",
        }
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Data::Unit => f.write_str("()"),
            Data::Bool(b) => write!(f, "{b}"),
            Data::Int(i) => write!(f, "{i}"),
            Data::Str(s) => write!(f, "{s:?}"),
            Data::List(items) => {
                f.write_str("[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", v.data)?;
                }
                f.write_str("]")
            }
            Data::Map(fields) => {
                f.write_str("{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{k}: {}", v.data)?;
                }
                f.write_str("}")
            }
        }
    }
}

/// A piece of data together with where it came from and who may read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    /// The data.
    pub data: Data,
    /// Its provenance and confidentiality.
    pub label: Label,
}

impl Value {
    /// A value with an explicit label.
    pub fn new(data: Data, label: Label) -> Self {
        Value { data, label }
    }

    /// A literal written in a plan: runtime provenance, publicly readable.
    pub fn literal(data: Data) -> Self {
        Value::new(data, Label::runtime())
    }

    /// Text with an explicit label.
    pub fn str(s: &str, label: Label) -> Self {
        Value::new(Data::Str(s.to_owned()), label)
    }

    /// This value with `label` joined onto its own, recursively through
    /// containers so no element keeps a cleaner label than its container.
    #[must_use]
    pub fn tainted_with(&self, label: &Label) -> Value {
        let data = match &self.data {
            Data::List(items) => Data::List(items.iter().map(|v| v.tainted_with(label)).collect()),
            Data::Map(fields) => Data::Map(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.tainted_with(label)))
                    .collect(),
            ),
            other => other.clone(),
        };
        Value::new(data, self.label.join(label))
    }

    /// The text of this value, if it is text.
    pub fn as_str(&self) -> Option<&str> {
        match &self.data {
            Data::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A field of this value, if it is a map and has one.
    ///
    /// The field's label is joined with the container's.
    pub fn field(&self, key: &str) -> Option<Value> {
        match &self.data {
            Data::Map(fields) => fields.get(key).map(|v| v.tainted_with(&self.label)),
            _ => None,
        }
    }

    /// An element of this value, if it is a list and has one.
    ///
    /// The element's label is joined with the container's.
    pub fn index(&self, i: usize) -> Option<Value> {
        match &self.data {
            Data::List(items) => items.get(i).map(|v| v.tainted_with(&self.label)),
            _ => None,
        }
    }

    /// The label of this value joined with the labels of everything inside
    /// it, which is what disclosing the whole value would reveal.
    pub fn deep_label(&self) -> Label {
        match &self.data {
            Data::List(items) => items
                .iter()
                .fold(self.label.clone(), |acc, v| acc.join(&v.deep_label())),
            Data::Map(fields) => fields
                .values()
                .fold(self.label.clone(), |acc, v| acc.join(&v.deep_label())),
            _ => self.label.clone(),
        }
    }
}

/// Plain JSON for a value's data, with the labels dropped.
///
/// A host that executes tools wants ordinary JSON, not the tagged form
/// [`Data`] serialises to. Labels are not sent: the host does not need them,
/// and a label is a claim this runtime makes rather than one it accepts.
pub fn data_to_json(data: &Data) -> serde_json::Value {
    match data {
        Data::Unit => serde_json::Value::Null,
        Data::Bool(b) => serde_json::Value::Bool(*b),
        Data::Int(i) => serde_json::Value::Number((*i).into()),
        Data::Str(s) => serde_json::Value::String(s.clone()),
        Data::List(items) => {
            serde_json::Value::Array(items.iter().map(|v| data_to_json(&v.data)).collect())
        }
        Data::Map(fields) => serde_json::Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), data_to_json(&v.data)))
                .collect(),
        ),
    }
}

/// Read plain JSON back as data, labelling every part with `label`.
///
/// Everything a host returns carries the same label, because the runtime
/// knows only that it came from that tool.
pub fn data_from_json(json: &serde_json::Value, label: &Label) -> Data {
    match json {
        serde_json::Value::Null => Data::Unit,
        serde_json::Value::Bool(b) => Data::Bool(*b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| Data::Str(n.to_string()), Data::Int),
        serde_json::Value::String(s) => Data::Str(s.clone()),
        serde_json::Value::Array(items) => Data::List(
            items
                .iter()
                .map(|v| Value::new(data_from_json(v, label), label.clone()))
                .collect(),
        ),
        serde_json::Value::Object(fields) => Data::Map(
            fields
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        Value::new(data_from_json(v, label), label.clone()),
                    )
                })
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    // Tests fail by panicking; that is their job.
    #![allow(clippy::panic)]

    use super::*;
    use crate::label::{Readers, Source};

    fn tool_label(name: &str) -> Label {
        Label::public([Source::tool(name)])
    }

    fn page() -> Value {
        Value::new(
            Data::Map(
                [(
                    "recipient".to_owned(),
                    Value::str("attacker@evil.invalid", tool_label("fetch")),
                )]
                .into_iter()
                .collect(),
            ),
            tool_label("fetch"),
        )
    }

    #[test]
    fn reading_a_field_carries_the_container_label() {
        let v = Value::new(
            Data::Map(
                [("x".to_owned(), Value::str("hi", Label::user()))]
                    .into_iter()
                    .collect(),
            ),
            tool_label("fetch"),
        );
        let f = v.field("x").unwrap_or_else(|| Value::literal(Data::Unit));
        assert!(f.label.sources.contains(&Source::tool("fetch")));
        assert!(f.label.sources.contains(&Source::User));
        assert_eq!(v.field("missing"), None);
        assert_eq!(v.index(0), None);
    }

    #[test]
    fn tainting_reaches_inside_containers() {
        let tainted = page().tainted_with(&Label::user());
        let inner = tainted
            .field("recipient")
            .unwrap_or_else(|| Value::literal(Data::Unit));
        assert!(inner.label.sources.contains(&Source::User));
        assert!(inner.label.sources.contains(&Source::tool("fetch")));
    }

    #[test]
    fn deep_label_collects_everything_inside() {
        let v = Value::new(
            Data::List(vec![
                Value::str("a", Label::user()),
                Value::str("b", tool_label("fetch")),
            ]),
            Label::runtime(),
        );
        let d = v.deep_label();
        assert!(d.sources.contains(&Source::User));
        assert!(d.sources.contains(&Source::tool("fetch")));
    }

    #[test]
    fn deep_label_narrows_readers_to_the_intersection() {
        let v = Value::new(
            Data::List(vec![
                Value::str(
                    "a",
                    Label {
                        sources: [].into_iter().collect(),
                        readers: Readers::only(["alice", "bob"]),
                    },
                ),
                Value::str(
                    "b",
                    Label {
                        sources: [].into_iter().collect(),
                        readers: Readers::only(["bob"]),
                    },
                ),
            ]),
            Label::runtime(),
        );
        assert!(v.deep_label().allows_reader("bob"));
        assert!(!v.deep_label().allows_reader("alice"));
    }

    #[test]
    fn plain_json_round_trips() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"a":1,"b":["x",true,null],"c":{"d":"e"}}"#)
                .unwrap_or(serde_json::Value::Null);
        let label = Label::public([Source::tool("host")]);
        let data = data_from_json(&json, &label);
        assert_eq!(data_to_json(&data), json);

        // Everything inside carries the tool's label.
        let v = Value::new(data, label.clone());
        let inner = v
            .field("c")
            .and_then(|c| c.field("d"))
            .unwrap_or_else(|| Value::literal(Data::Unit));
        assert!(inner.label.sources.contains(&Source::tool("host")));
    }

    #[test]
    fn a_number_too_large_for_an_integer_becomes_text() {
        let json: serde_json::Value =
            serde_json::from_str("1e400").unwrap_or(serde_json::Value::Null);
        let data = data_from_json(&json, &Label::runtime());
        assert!(matches!(data, Data::Str(_) | Data::Unit), "{data:?}");
    }

    #[test]
    fn accessors_and_display() {
        assert_eq!(Value::str("x", Label::runtime()).as_str(), Some("x"));
        assert_eq!(Value::literal(Data::Int(1)).as_str(), None);
        assert_eq!(Data::Int(3).kind(), "int");
        assert_eq!(
            format!("{}", Data::List(vec![Value::literal(Data::Int(1))])),
            "[1]"
        );
    }
}
