//! Versioned compiled presentation observations; no source-only surrogate.
use serde::{Deserialize, Serialize};
pub mod cache;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Explanation<C = Check> {
    pub version: u8,
    #[schemars(schema_with = "elaborated_schema")]
    pub origin: String,
    pub sentence: String,
    pub refutation: String,
    pub checker: C,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Check {
    pub engine: String,
    #[schemars(schema_with = "status_schema")]
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

fn elaborated_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type":"string", "const":"elaborated"})
}
fn status_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type":"string", "enum":["counterexample","predicate_false_example","not_found","gave_up","budget_exhausted","unavailable","unsupported","not_proposition"]})
}

/// Compiler description without a search observation. Publication requires a
/// separately completed check; `None` is never a negative search verdict.
pub type Description = Explanation<Option<Check>>;

impl<C> Explanation<C> {
    fn valid_description(&self) -> bool {
        self.version == 1
            && self.origin == "elaborated"
            && !self.sentence.is_empty()
            && self.sentence.len() <= 65_536
            && self.refutation.len() <= 65_536
            && self.intermediate.as_ref().is_none_or(|s| s.len() <= 65_536)
            && self.renderer.as_ref().is_none_or(|s| s.len() <= 256)
            && self
                .rendering_error
                .as_ref()
                .is_none_or(|s| s.len() <= 65_536)
    }
    fn with_checker<D>(self, checker: D) -> Explanation<D> {
        Explanation {
            version: self.version,
            origin: self.origin,
            sentence: self.sentence,
            refutation: self.refutation,
            checker,
            intermediate: self.intermediate,
            renderer: self.renderer,
            rendering_error: self.rendering_error,
        }
    }
}
impl Check {
    pub fn valid(&self) -> bool {
        !self.engine.is_empty()
            && self.engine.len() <= 128
            && matches!(
                self.status.as_str(),
                "counterexample"
                    | "predicate_false_example"
                    | "not_found"
                    | "gave_up"
                    | "budget_exhausted"
                    | "unavailable"
                    | "unsupported"
                    | "not_proposition"
            )
            && self.candidates.len() <= 64
            && self.candidates.iter().all(|s| s.len() <= 8192)
            && self.diagnostic.as_ref().is_none_or(|s| s.len() <= 65_536)
    }
}
impl Explanation {
    pub fn valid(&self) -> bool {
        self.valid_description() && self.checker.valid()
    }
}
impl Description {
    pub fn valid(&self) -> bool {
        self.valid_description() && self.checker.as_ref().is_none_or(Check::valid)
    }
    pub fn complete(mut self) -> anyhow::Result<Explanation> {
        use anyhow::Context as _;
        let checker = self
            .checker
            .take()
            .context("Search observation has not been prepared")?;
        anyhow::ensure!(
            self.valid_description() && checker.valid(),
            "Invalid prepared explanation"
        );
        Ok(self.with_checker(checker))
    }
}
