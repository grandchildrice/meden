//! Bounded data decoding and typing, independent of solver execution and search settings.
use super::{Model, Predicate};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt::Write as _, io, path::Path};

/// Exact backend-owned data schema. It cannot contain executable SMT text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum Format {
    /// Version one of finite unsigned bit-vector transition contracts.
    #[serde(rename = "finite-bitvectors-v1")]
    FiniteBitvectors,
}
/// A named safety property over one original input and simultaneous output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Property {
    /// Stable short label.
    pub name: String,
    /// Reviewed intended behavior.
    pub description: String,
    /// Actual finite unsigned predicate checked by the backend.
    pub statement: Predicate,
}
/// Bounded proposal data, never authority to check or accept anything.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    /// Unambiguous host-registered contract format.
    pub format: Format,
    /// Reviewed title.
    pub name: String,
    /// Reviewed natural-language intent, not a proved equivalence claim.
    pub description: String,
    /// Workspace-relative JSON transition file.
    pub source_path: String,
    /// All fields and parameters have this unsigned width (1 through 8).
    #[schemars(range(min = 1, max = 8))]
    pub width: u8,
    /// Ordered simultaneous state fields.
    #[schemars(length(min = 1, max = 4))]
    pub fields: Vec<String>,
    /// Ordered finite transition arguments.
    #[schemars(length(max = 4))]
    pub parameters: Vec<String>,
    /// Nonempty exact property set.
    #[schemars(length(min = 1, max = 16))]
    pub properties: Vec<Property>,
    /// Environmental assumptions for host review; no hidden logical premises.
    #[serde(default)]
    #[schemars(length(max = 16))]
    pub assumptions: Vec<String>,
}
pub(super) fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
pub(super) fn require(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

impl Contract {
    /// Model-facing shape generated from the actual backend-owned data types.
    pub fn schema() -> io::Result<serde_json::Value> {
        serde_json::to_value(schemars::schema_for!(Self)).map_err(io::Error::other)
    }
    /// Decode only the bounded data schema; no tool is run.
    pub fn parse(text: &str) -> io::Result<Self> {
        require(text.len() <= 65_536, "Finite contract exceeds 64 KiB")?;
        let value: Self = serde_json::from_str(text)?;
        value.validate()?;
        Ok(value)
    }
    /// Check identifiers, finite scope, expression typing and computation bounds.
    pub fn validate(&self) -> io::Result<()> {
        require(
            !self.name.is_empty() && self.name.len() <= 256 && self.description.len() <= 4096,
            "Invalid finite contract metadata",
        )?;
        require(
            !self.source_path.is_empty()
                && self.source_path.len() <= 128
                && Path::new(&self.source_path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                && self.source_path.split('/').all(|p| {
                    !p.is_empty()
                        && p != "."
                        && p != ".."
                        && p.bytes()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
                }),
            "Finite source must be a normalized relative .json path",
        )?;
        require(
            (1..=8).contains(&self.width)
                && !self.fields.is_empty()
                && self.fields.len() <= 4
                && self.parameters.len() <= 4,
            "Finite words require width 1..8, 1..4 fields and 0..4 parameters",
        )?;
        for names in [&self.fields, &self.parameters] {
            require(
                names.iter().all(|s| name(s))
                    && names.iter().collect::<BTreeSet<_>>().len() == names.len(),
                "Finite field/parameter labels must be unique ASCII identifiers",
            )?;
        }
        require(self.total_bits()<=16,"Finite verification supports at most 16 total input bits; no partial domain is checked")?;
        require(
            !self.properties.is_empty()
                && self.properties.len() <= 16
                && self
                    .properties
                    .iter()
                    .map(|p| &p.name)
                    .collect::<BTreeSet<_>>()
                    .len()
                    == self.properties.len(),
            "Declare 1..16 uniquely named finite properties",
        )?;
        require(
            self.assumptions.len() <= 16 && self.assumptions.iter().all(|s| s.len() <= 1024),
            "Finite assumptions exceed bounds",
        )?;
        let mut nodes = 0;
        for p in &self.properties {
            require(
                name(&p.name) && p.description.len() <= 1024,
                "Invalid finite property metadata",
            )?;
            p.statement.validate(
                &self.fields,
                &self.parameters,
                self.width,
                true,
                0,
                &mut nodes,
            )?;
        }
        require(
            u64::try_from(nodes)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::from(self.cases()?))
                <= 4_194_304,
            "Finite property/domain work exceeds the complete-check budget",
        )
    }
    pub(super) fn total_bits(&self) -> usize {
        self.fields
            .len()
            .saturating_add(self.parameters.len())
            .saturating_mul(usize::from(self.width))
    }
    pub(super) fn cases(&self) -> io::Result<u32> {
        1u32.checked_shl(u32::try_from(self.total_bits()).map_err(io::Error::other)?)
            .ok_or_else(|| invalid("Invalid finite domain"))
    }
    /// Original canonical property and scope bytes used for host review.
    pub fn canonical(&self) -> io::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(io::Error::other)
    }
    /// Exact finite claim; it is not an unbounded-time or native-code proof.
    pub fn scope(&self) -> String {
        format!("Bounded one-step check of every input in the {}-bit unsigned domain ({} total input bits, {} assignments). Addition/subtraction wrap modulo 2^{}. No reachability, unbounded time, Rust correspondence or whole-application claim.",self.width,self.total_bits(),self.cases().unwrap_or(0),self.width)
    }
    /// Full unambiguous host review, independent of executable source.
    pub fn review_text(&self) -> String {
        let mut text=format!("{}\n\n{}\n\nSource: {}\n\nEvidence: bounded finite bit-vector checking\n{}\n\nState fields: {}\nParameters: {}\nAll output assignments are simultaneous. Every original state and argument is checked, not just sampled or reachable states.\n",self.name,self.description,self.source_path,self.scope(),self.fields.join(", "),self.parameters.join(", "));
        for p in &self.properties {
            let _ = writeln!(
                text,
                "\n{}: {}\n{}",
                p.name,
                p.description,
                p.statement.text()
            );
        }
        for assumption in &self.assumptions {
            let _ = writeln!(text, "\nReviewed environmental assumption: {assumption}");
        }
        text
    }
    /// Re-evaluate a concrete original-domain witness; never admits verification evidence.
    pub fn validates_witness(
        &self,
        model: &Model,
        witness: &super::Counterexample,
    ) -> io::Result<bool> {
        self.validate()?;
        let bound = 1u16 << self.width;
        if witness.before.len() != self.fields.len()
            || witness.arguments.len() != self.parameters.len()
            || witness.after.len() != self.fields.len()
            || witness
                .before
                .iter()
                .chain(&witness.arguments)
                .chain(&witness.after)
                .any(|v| *v >= bound)
        {
            return Ok(false);
        }
        let Some(property) = self.properties.iter().find(|p| p.name == witness.property) else {
            return Ok(false);
        };
        let mut context = super::terms::Context {
            fields: &self.fields,
            parameters: &self.parameters,
            width: self.width,
            before: &witness.before,
            arguments: &witness.arguments,
            after: &[],
        };
        let after = self
            .fields
            .iter()
            .map(|f| {
                model
                    .transition
                    .get(f)
                    .ok_or_else(|| invalid("Missing finite assignment"))?
                    .eval(&context)
            })
            .collect::<io::Result<Vec<_>>>()?;
        if after != witness.after {
            return Ok(false);
        }
        context.after = &after;
        Ok(!property.statement.eval(&context)?)
    }
    /// Decode and type-check a complete transition against this original finite contract.
    pub fn decode_model(&self, source: &str) -> io::Result<Model> {
        self.validate()?;
        require(source.len() <= 65_536, "Finite source exceeds 64 KiB")?;
        let model: Model = serde_json::from_str(source)?;
        require(
            model.version == 1
                && model.transition.keys().collect::<BTreeSet<_>>() == self.fields.iter().collect(),
            "Finite transition must assign exactly every declared field",
        )?;
        let mut nodes = 0;
        for expression in model.transition.values() {
            expression.validate(
                &self.fields,
                &self.parameters,
                self.width,
                false,
                0,
                &mut nodes,
            )?;
        }
        require(
            u64::try_from(nodes)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::from(self.cases()?))
                <= 4_194_304,
            "Finite source/domain work exceeds the complete-check budget",
        )?;
        Ok(model)
    }
}

/// Exact data decoder/schema implementation identity, independent of solver configuration.
pub fn identity() -> String {
    super::hash(
        &[
            include_bytes!("data.rs").as_slice(),
            include_bytes!("terms.rs").as_slice(),
        ]
        .concat(),
    )
}
