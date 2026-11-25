//! Deterministic realization of the backend's validated finite expression AST.
use anyhow::{Context as _, Result};
use meden_runtime::{
    plugin::{Definition, Descriptor, Role},
    process::Control,
    verification::finite::{Contract, Format, Model, Predicate, Word},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Term {
    Predicate { value: Predicate },
    Word { value: Word },
}
#[derive(Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Expression {
    pub width: u8,
    pub fields: Vec<String>,
    pub parameters: Vec<String>,
    pub term: Term,
}
pub(super) struct Renderer;
impl crate::meaning::Renderer for Renderer {
    type Expression = Expression;
    fn descriptor(&self) -> Result<Descriptor> {
        Ok(Descriptor::new(Definition {
            id: "finite.english",
            role: Role::MeaningRenderer,
            version: "1",
            formats: vec!["finite-expression-v1".into()],
            implementation: &[
                include_bytes!("render.rs").as_slice(),
                include_bytes!("../../meaning.rs").as_slice(),
            ]
            .concat(),
            configuration: serde_json::json!({"decoder":meden_runtime::verification::finite::decoding_identity()}),
            input: serde_json::to_value(schemars::schema_for!(Expression))?,
            output: serde_json::to_value(schemars::schema_for!(crate::meaning::Reading))?,
        })?)
    }
    fn validate(&self) -> Result<()> {
        Ok(())
    }
    fn render(&self, control: &Control, e: Expression) -> Result<crate::meaning::Reading> {
        let _span = meden_runtime::metrics::Span::start("finite.render");
        control.with_active(|| Ok(()))?;
        let c = Contract {
            format: Format::FiniteBitvectors,
            name: "renderer".into(),
            description: String::new(),
            source_path: "input.model.json".into(),
            width: e.width,
            fields: e.fields,
            parameters: e.parameters,
            properties: vec![meden_runtime::verification::finite::Property {
                name: "expression".into(),
                description: String::new(),
                statement: match &e.term {
                    Term::Predicate { value } => value.clone(),
                    Term::Word { .. } => Predicate::True,
                },
            }],
            assumptions: vec![],
        };
        c.validate()?;
        if let Term::Word { value } = &e.term {
            let mut transition = c
                .fields
                .iter()
                .map(|f| (f.clone(), Word::Constant { value: 0 }))
                .collect::<BTreeMap<_, _>>();
            transition.insert(
                c.fields
                    .first()
                    .context("Finite renderer requires a state field")?
                    .clone(),
                value.clone(),
            );
            c.decode_model(&serde_json::to_string(&Model {
                version: 1,
                transition,
            })?)?;
        }
        let statement = match &e.term {
            Term::Predicate { value } => predicate(value),
            Term::Word { value } => format!("the next value is {}", word(value)),
        };
        Ok(crate::meaning::Reading{sentence:format!("For one simultaneous transition of unsigned {}-bit words, {statement}. Addition and subtraction wrap modulo 2^{}.",c.width,c.width),renderer:"finite.english".into()})
    }
}
fn word(w: &Word) -> String {
    match w {
        Word::Constant { value } => value.to_string(),
        Word::State { field } => format!("input `{field}`"),
        Word::Output { field } => format!("output `{field}`"),
        Word::Parameter { name } => format!("argument `{name}`"),
        Word::Add { left, right } => format!("({} plus {})", word(left), word(right)),
        Word::Sub { left, right } => format!("({} minus {})", word(left), word(right)),
        Word::If { condition, yes, no } => format!(
            "(if {}, then {}, otherwise {})",
            predicate(condition),
            word(yes),
            word(no)
        ),
    }
}
fn predicate(p: &Predicate) -> String {
    match p {
        Predicate::True => "true".into(),
        Predicate::False => "false".into(),
        Predicate::Equal { left, right } => format!("{} equals {}", word(left), word(right)),
        Predicate::Less { left, right } => format!("{} is less than {}", word(left), word(right)),
        Predicate::LessEqual { left, right } => {
            format!("{} is at most {}", word(left), word(right))
        }
        Predicate::Not { argument } => format!("not ({})", predicate(argument)),
        Predicate::And { left, right } => {
            format!("({}) and ({})", predicate(left), predicate(right))
        }
        Predicate::Or { left, right } => format!("({}) or ({})", predicate(left), predicate(right)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::AtomicBool, Arc};
    #[test]
    fn readings_use_only_typed_terms_and_keep_unsigned_modular_scope() -> Result<()> {
        let control = Control::new(Arc::new(AtomicBool::new(false)));
        let expression = Expression {
            width: 4,
            fields: vec!["value".into()],
            parameters: vec!["amount".into()],
            term: Term::Predicate {
                value: Predicate::LessEqual {
                    left: Word::State {
                        field: "value".into(),
                    },
                    right: Word::Output {
                        field: "value".into(),
                    },
                },
            },
        };
        let bound = crate::meaning::Bound::new(Renderer)?;
        let first = bound.batch(&control, |batch| batch.render(expression.clone()))?;
        let second = bound.batch(&control, |batch| batch.render(expression.clone()))?;
        assert_eq!(first.sentence, second.sentence);
        assert_eq!(first.sentence,"For one simultaneous transition of unsigned 4-bit words, input `value` is at most output `value`. Addition and subtraction wrap modulo 2^4.");
        for expression in [
            Expression {
                width: 0,
                ..expression.clone()
            },
            Expression {
                fields: vec![],
                ..expression.clone()
            },
            Expression {
                term: Term::Predicate {
                    value: Predicate::Equal {
                        left: Word::State {
                            field: "foreign".into(),
                        },
                        right: Word::Constant { value: 0 },
                    },
                },
                ..expression.clone()
            },
        ] {
            assert!(bound
                .batch(&control, |batch| batch.render(expression))
                .is_err());
        }
        control.cancel();
        assert!(bound
            .batch(&control, |batch| batch.render(expression))
            .is_err());
        Ok(())
    }
}
