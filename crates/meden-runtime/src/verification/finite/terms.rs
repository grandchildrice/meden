//! Data-only finite bit-vector expressions. No strings are evaluated as commands.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

/// Unsigned word expression. Arithmetic is modulo the contract's fixed width.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Word {
    /// A literal within the declared finite domain.
    Constant {
        #[schemars(range(max = 255))]
        value: u16,
    },
    /// Original state field.
    State { field: String },
    /// Original transition argument.
    Parameter { name: String },
    /// Simultaneous result field, allowed only in a property.
    Output { field: String },
    /// Modular sum.
    Add { left: Box<Self>, right: Box<Self> },
    /// Modular difference.
    Sub { left: Box<Self>, right: Box<Self> },
    /// Conditional word.
    If {
        condition: Box<Predicate>,
        #[serde(rename = "then")]
        yes: Box<Self>,
        #[serde(rename = "else")]
        no: Box<Self>,
    },
}
/// Boolean expressions over the original input and simultaneous output.
#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    /// Boolean true.
    True,
    /// Boolean false.
    False,
    /// Exact word equality.
    Equal { left: Word, right: Word },
    /// Unsigned comparison.
    Less { left: Word, right: Word },
    /// Unsigned non-strict comparison.
    LessEqual { left: Word, right: Word },
    /// Boolean negation.
    Not { argument: Box<Self> },
    /// Boolean conjunction.
    And { left: Box<Self>, right: Box<Self> },
    /// Boolean disjunction.
    Or { left: Box<Self>, right: Box<Self> },
}

pub(super) struct Context<'a> {
    pub fields: &'a [String],
    pub parameters: &'a [String],
    pub width: u8,
    pub before: &'a [u16],
    pub arguments: &'a [u16],
    pub after: &'a [u16],
}
fn error(message: &str) -> io::Error {
    io::Error::other(message)
}
fn index(names: &[String], name: &str) -> io::Result<usize> {
    names
        .iter()
        .position(|n| n == name)
        .ok_or_else(|| error("Unknown finite expression variable"))
}
fn value(names: &[String], values: &[u16], name: &str) -> io::Result<u16> {
    values
        .get(index(names, name)?)
        .copied()
        .ok_or_else(|| error("Finite input vector mismatch"))
}
fn bound(depth: usize, nodes: &mut usize) -> io::Result<()> {
    *nodes = nodes.saturating_add(1);
    if depth > 32 || *nodes > 512 {
        return Err(error("Finite expressions exceed 32 levels/512 nodes"));
    }
    Ok(())
}
impl Word {
    pub(super) fn validate(
        &self,
        fields: &[String],
        parameters: &[String],
        width: u8,
        output: bool,
        depth: usize,
        nodes: &mut usize,
    ) -> io::Result<()> {
        bound(depth, nodes)?;
        match self {
            Self::Constant { value } => {
                if u32::from(*value) >= (1u32 << width) {
                    return Err(error("Finite literal exceeds its word width"));
                }
            }
            Self::State { field } => {
                index(fields, field)?;
            }
            Self::Parameter { name } => {
                index(parameters, name)?;
            }
            Self::Output { field } => {
                if !output {
                    return Err(error("Transition expressions cannot read output fields"));
                }
                index(fields, field)?;
            }
            Self::Add { left, right } | Self::Sub { left, right } => {
                left.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
                right.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
            }
            Self::If { condition, yes, no } => {
                condition.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
                yes.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
                no.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn eval(&self, c: &Context<'_>) -> io::Result<u16> {
        let modulus = 1u32
            .checked_shl(u32::from(c.width))
            .ok_or_else(|| error("Invalid finite word width"))?;
        Ok(match self {
            Self::Constant { value } => *value,
            Self::State { field } => value(c.fields, c.before, field)?,
            Self::Parameter { name } => value(c.parameters, c.arguments, name)?,
            Self::Output { field } => value(c.fields, c.after, field)?,
            Self::Add { left, right } => u16::try_from(
                u32::from(left.eval(c)?)
                    .checked_add(u32::from(right.eval(c)?))
                    .and_then(|n| n.checked_rem(modulus))
                    .ok_or_else(|| error("Invalid finite sum"))?,
            )
            .map_err(io::Error::other)?,
            Self::Sub { left, right } => u16::try_from(
                u32::from(left.eval(c)?)
                    .checked_add(modulus)
                    .and_then(|n| n.checked_sub(u32::from(right.eval(c).ok()?)))
                    .and_then(|n| n.checked_rem(modulus))
                    .ok_or_else(|| error("Invalid finite difference"))?,
            )
            .map_err(io::Error::other)?,
            Self::If { condition, yes, no } => {
                if condition.eval(c)? {
                    yes.eval(c)?
                } else {
                    no.eval(c)?
                }
            }
        })
    }
    pub(super) fn smt(
        &self,
        fields: &[String],
        parameters: &[String],
        width: u8,
    ) -> io::Result<String> {
        Ok(match self {
            Self::Constant { value } => format!("(_ bv{value} {width})"),
            Self::State { field } => format!("s{}", index(fields, field)?),
            Self::Parameter { name } => format!("p{}", index(parameters, name)?),
            Self::Output { field } => format!("o{}", index(fields, field)?),
            Self::Add { left, right } | Self::Sub { left, right } => format!(
                "({} {} {})",
                if matches!(self, Self::Add { .. }) {
                    "bvadd"
                } else {
                    "bvsub"
                },
                left.smt(fields, parameters, width)?,
                right.smt(fields, parameters, width)?
            ),
            Self::If { condition, yes, no } => format!(
                "(ite {} {} {})",
                condition.smt(fields, parameters, width)?,
                yes.smt(fields, parameters, width)?,
                no.smt(fields, parameters, width)?
            ),
        })
    }
    /// Unambiguous display of the restricted expression, never executable text.
    pub fn text(&self) -> String {
        match self {
            Self::Constant { value } => value.to_string(),
            Self::State { field } => format!("s.{field}"),
            Self::Parameter { name } => format!("arg.{name}"),
            Self::Output { field } => format!("out.{field}"),
            Self::Add { left, right } | Self::Sub { left, right } => format!(
                "({} {} {})",
                left.text(),
                if matches!(self, Self::Add { .. }) {
                    "+"
                } else {
                    "-"
                },
                right.text()
            ),
            Self::If { condition, yes, no } => format!(
                "(if {} then {} else {})",
                condition.text(),
                yes.text(),
                no.text()
            ),
        }
    }
}
impl Predicate {
    pub(super) fn validate(
        &self,
        fields: &[String],
        parameters: &[String],
        width: u8,
        output: bool,
        depth: usize,
        nodes: &mut usize,
    ) -> io::Result<()> {
        bound(depth, nodes)?;
        match self {
            Self::True | Self::False => {}
            Self::Equal { left, right }
            | Self::Less { left, right }
            | Self::LessEqual { left, right } => {
                left.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
                right.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
            }
            Self::Not { argument } => argument.validate(
                fields,
                parameters,
                width,
                output,
                depth.saturating_add(1),
                nodes,
            )?,
            Self::And { left, right } | Self::Or { left, right } => {
                left.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
                right.validate(
                    fields,
                    parameters,
                    width,
                    output,
                    depth.saturating_add(1),
                    nodes,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn eval(&self, c: &Context<'_>) -> io::Result<bool> {
        Ok(match self {
            Self::True => true,
            Self::False => false,
            Self::Equal { left, right } => left.eval(c)? == right.eval(c)?,
            Self::Less { left, right } => left.eval(c)? < right.eval(c)?,
            Self::LessEqual { left, right } => left.eval(c)? <= right.eval(c)?,
            Self::Not { argument } => !argument.eval(c)?,
            Self::And { left, right } => left.eval(c)? && right.eval(c)?,
            Self::Or { left, right } => left.eval(c)? || right.eval(c)?,
        })
    }
    pub(super) fn smt(
        &self,
        fields: &[String],
        parameters: &[String],
        width: u8,
    ) -> io::Result<String> {
        Ok(match self {
            Self::True => "true".into(),
            Self::False => "false".into(),
            Self::Equal { left, right }
            | Self::Less { left, right }
            | Self::LessEqual { left, right } => format!(
                "({} {} {})",
                match self {
                    Self::Equal { .. } => "=",
                    Self::Less { .. } => "bvult",
                    _ => "bvule",
                },
                left.smt(fields, parameters, width)?,
                right.smt(fields, parameters, width)?
            ),
            Self::Not { argument } => format!("(not {})", argument.smt(fields, parameters, width)?),
            Self::And { left, right } | Self::Or { left, right } => format!(
                "({} {} {})",
                if matches!(self, Self::And { .. }) {
                    "and"
                } else {
                    "or"
                },
                left.smt(fields, parameters, width)?,
                right.smt(fields, parameters, width)?
            ),
        })
    }
    /// Fully parenthesized unsigned predicate for exact host review.
    pub fn text(&self) -> String {
        match self {
            Self::True => "true".into(),
            Self::False => "false".into(),
            Self::Equal { left, right }
            | Self::Less { left, right }
            | Self::LessEqual { left, right } => format!(
                "({} {} {})",
                left.text(),
                match self {
                    Self::Equal { .. } => "=",
                    Self::Less { .. } => "<",
                    _ => "≤",
                },
                right.text()
            ),
            Self::Not { argument } => format!("(not {})", argument.text()),
            Self::And { left, right } | Self::Or { left, right } => format!(
                "({} {} {})",
                left.text(),
                if matches!(self, Self::And { .. }) {
                    "and"
                } else {
                    "or"
                },
                right.text()
            ),
        }
    }
}

/// Simultaneous finite transition assignments, not Rust or SMT-LIB source.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// Exact data-format version.
    pub version: u8,
    /// Each declared state field must occur exactly once.
    #[serde(deserialize_with = "assignments")]
    pub transition: BTreeMap<String, Word>,
}
fn assignments<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<BTreeMap<String, Word>, D::Error> {
    struct Assignments;
    impl<'de> serde::de::Visitor<'de> for Assignments {
        type Value = BTreeMap<String, Word>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("one assignment per finite state field")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut fields = BTreeMap::new();
            while let Some((field, expression)) = map.next_entry::<String, Word>()? {
                if fields.len() >= 4 || fields.insert(field, expression).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate or excessive finite assignments",
                    ));
                }
            }
            Ok(fields)
        }
    }
    decoder.deserialize_map(Assignments)
}
