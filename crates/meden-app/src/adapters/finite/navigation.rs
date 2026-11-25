//! Pure bounded JSON declaration locations; model-only navigation has no contract
//! context and confers no type-checking or search claim.
use super::positions;
use crate::adapters::source::Symbol;
use anyhow::Result;
use meden_runtime::verification::finite::{Contract, Model};
pub(in crate::adapters) fn supports(path: &str) -> bool {
    path.ends_with(".contract.json") || path.ends_with(".model.json")
}
pub(in crate::adapters) fn identity() -> String {
    use std::sync::LazyLock;
    static ID: LazyLock<String> = LazyLock::new(|| {
        super::digest(
            &[
                include_bytes!("navigation.rs").as_slice(),
                include_bytes!("positions.rs").as_slice(),
                meden_runtime::verification::finite::decoding_identity().as_bytes(),
            ]
            .concat(),
        )
    });
    ID.clone()
}
pub(in crate::adapters) fn symbols(path: &str, text: &str) -> Result<Vec<Symbol>> {
    anyhow::ensure!(
        text.len() <= 65_536,
        "Finite navigation source exceeds 64 KiB"
    );
    let entries = if path.ends_with(".contract.json") {
        Contract::parse(text)?
            .properties
            .into_iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    format!("property.{}", p.name),
                    format!("/properties/{i}"),
                    "theorem",
                    p.statement.text(),
                )
            })
            .collect::<Vec<_>>()
    } else {
        let model: Model = serde_json::from_str(text)?;
        anyhow::ensure!(
            model.version == 1 && model.transition.len() <= 4,
            "Unsupported finite model navigation"
        );
        model
            .transition
            .into_iter()
            .map(|(f, e)| {
                (
                    format!("transition.{f}"),
                    format!("/transition/{f}"),
                    "definition",
                    e.text(),
                )
            })
            .collect()
    };
    let positions = positions::values(text)?;
    entries
        .into_iter()
        .map(|(name, pointer, kind, declaration)| {
            Ok(Symbol {
                name,
                line: *positions.get(&pointer).ok_or_else(|| {
                    anyhow::anyhow!("Original finite declaration position unavailable")
                })?,
                kind: kind.into(),
                declaration,
                private: false,
                references: vec![],
                semantic: None,
                main_theorem: false,
            })
        })
        .collect()
}
