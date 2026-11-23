//! Versioned compiled presentation observations; no source-only surrogate.
use serde::{Deserialize, Serialize};
pub mod cache;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Explanation {
    pub version: u8,
    #[schemars(schema_with = "elaborated_schema")]
    pub origin: String,
    pub sentence: String,
    pub refutation: String,
    pub checker: Check,
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

impl Explanation {
    pub fn valid(&self) -> bool {
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
            && !self.checker.engine.is_empty()
            && self.checker.engine.len() <= 128
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
