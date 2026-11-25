//! Candidate meaning assessment, separate from proof/contract authority.
//! Prompt framing/output fields adapted from claimcheck; see semantic/README.md.
use crate::identity::snapshots::GraphRevision;
use crate::identity::snapshots::SourceRevision;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use meden_agent::conversation::{ChatModel, Reply};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const ENGINE: &str = "meden-claimcheck-roundtrip-v2";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Target {
    #[schemars(length(min = 1, max = 4096))]
    pub file: String,
    #[schemars(range(min = 1, max = 1_048_577))]
    pub line: usize,
    #[schemars(length(min = 1, max = 1000))]
    pub name: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Input {
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    pub source_revision: SourceRevision,
    #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_revision: Option<GraphRevision>,
    pub target: Target,
    #[schemars(length(min = 1, max = 16_384))]
    pub requirement: String,
}
/// Backend-owned decoded meaning, separate from source/graph presentation.
/// Context is serialized only at the final model-message boundary; the shared
/// workflow does not interpret a universal AST or execute a compiler here.
pub trait Observation: crate::analysis::sources::Source {
    type Context: Serialize;
    fn graph_revision(&self) -> Option<&GraphRevision>;
    fn context(&self, target: &Target) -> Result<Option<Self::Context>>;
}

/// Frozen model messages only. No AST, approval or verification authority.
pub struct Prepared {
    input: Input,
    translation: Vec<Value>,
}
impl Prepared {
    pub fn new<O: Observation>(input: Input, original: &O) -> Result<Self> {
        input.validate()?;
        original.sources().validate()?;
        let files = &original.sources().files;
        anyhow::ensure!(
            !files.is_empty() && files.len() <= 32,
            "semantic context needs 1–32 files"
        );
        anyhow::ensure!(
            files.values().all(|file| file.len() <= 65_536),
            "semantic source exceeds 64 KiB"
        );
        anyhow::ensure!(
            files.values().map(String::len).sum::<usize>() <= 1_048_576,
            "semantic context exceeds 1 MiB"
        );
        let revision = SourceRevision::from_digest(Sha256::digest(serde_json::to_vec(&files)?));
        anyhow::ensure!(
            revision == input.source_revision,
            "semantic source revision changed; reread the graph"
        );
        let source = files
            .get(&input.target.file)
            .context("selected declaration is outside the observed files")?;
        anyhow::ensure!(
            source
                .lines()
                .nth(input.target.line.saturating_sub(1))
                .is_some(),
            "selected source line is unavailable"
        );
        anyhow::ensure!(
            input.graph_revision.as_ref() == original.graph_revision(),
            "Semantic graph revision changed; reread and select the declaration again"
        );
        let elaborated = original.context(&input.target)?;
        anyhow::ensure!(
            elaborated.is_some() == input.graph_revision.is_some(),
            "Backend meaning context does not match its original graph binding"
        );
        // Freeze once before any model call. Later backend mutation cannot
        // rewrite these messages or introduce the independently held intent.
        let elaborated = elaborated.map(serde_json::to_value).transpose()?;
        if let Some(context) = &elaborated {
            anyhow::ensure!(
                serde_json::to_vec(context)?.len() <= 1_048_576,
                "Elaborated meaning context exceeds 1 MiB; select a smaller target/module set"
            );
        }
        let translation = messages(
            include_str!("semantic/informalize.md"),
            &elaborated.as_ref().map_or_else(
                || json!({"target":input.target,"files":files}),
                |context| json!({"target":input.target,"elaborated":context}),
            ),
        );
        Ok(Self { input, translation })
    }
    fn translation_messages(&self) -> &[Value] {
        &self.translation
    }
}
impl Input {
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            !self.requirement.trim().is_empty() && self.requirement.len() <= 16_384,
            "requirement must contain 1–16384 bytes"
        );
        anyhow::ensure!(
            self.target.line > 0
                && !self.target.file.is_empty()
                && self.target.file.len() <= 4096
                && !self.target.name.is_empty()
                && self.target.name.len() <= 1000,
            "invalid semantic target"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Translation {
    pub lemma_name: String,
    pub natural_language: String,
    pub preconditions: String,
    pub postcondition: String,
    pub scope: String,
    pub strength: Strength,
    #[schemars(range(min = 0, max = 1))]
    pub confidence: f64,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Strength {
    Trivial,
    Weak,
    Moderate,
    Strong,
}
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Comparison {
    pub lemma_name: String,
    #[serde(rename = "match")]
    pub matches: bool,
    pub discrepancy: String,
    pub weakening_type: Weakening,
    pub explanation: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Weakening {
    None,
    Tautology,
    WeakenedPostcondition,
    NarrowedScope,
    MissingCase,
    WrongProperty,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Match,
    Mismatch,
    Unclear,
}
#[derive(Clone, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub translation: Translation,
    pub comparison: Comparison,
    pub verdict: Verdict,
}
#[derive(Clone)]
pub enum Stage {
    Translating,
    Comparing(Translation),
}

pub fn analyze(
    translator: &dyn ChatModel,
    judge: &dyn ChatModel,
    prepared: &Prepared,
    cancelled: &AtomicBool,
    progress: &dyn Fn(Stage),
) -> Result<Assessment> {
    ensure_active(cancelled)?;
    progress(Stage::Translating);
    ensure_active(cancelled)?;
    let translation: Translation = decode(
        translator.reply(prepared.translation_messages(), &[translation_tool()])?,
        "record_informalization",
    )?;
    translation.validate(&prepared.input.target.name)?;
    ensure_active(cancelled)?;
    progress(Stage::Comparing(translation.clone()));
    ensure_active(cancelled)?;
    let comparison: Comparison = decode(
        judge.reply(
            &messages(
                include_str!("semantic/compare.md"),
                &json!({
                    "requirement":prepared.input.requirement, "target":prepared.input.target,
                    "backTranslation":translation,
                }),
            ),
            &[comparison_tool()],
        )?,
        "record_comparison",
    )?;
    ensure_active(cancelled)?;
    comparison.validate(&prepared.input.target.name)?;
    let verdict = if translation.confidence < 0.5
        || comparison.matches
            && (comparison.weakening_type != Weakening::None
                || !comparison.discrepancy.trim().is_empty())
    {
        Verdict::Unclear
    } else if comparison.matches {
        Verdict::Match
    } else {
        Verdict::Mismatch
    };
    Ok(Assessment {
        translation,
        comparison,
        verdict,
    })
}
fn ensure_active(cancelled: &AtomicBool) -> Result<()> {
    anyhow::ensure!(
        !cancelled.load(Ordering::SeqCst),
        "Meaning analysis cancelled; no result admitted"
    );
    Ok(())
}
impl Translation {
    fn validate(&self, name: &str) -> Result<()> {
        anyhow::ensure!(self.lemma_name == name, "translation target changed");
        anyhow::ensure!(
            self.confidence.is_finite() && (0.0..=1.0).contains(&self.confidence),
            "invalid translation confidence"
        );
        anyhow::ensure!(
            !self.natural_language.trim().is_empty()
                && !self.postcondition.trim().is_empty()
                && !self.scope.trim().is_empty(),
            "translation omitted its meaning or scope"
        );
        validate_text(&[
            &self.natural_language,
            &self.preconditions,
            &self.postcondition,
            &self.scope,
        ])
    }
}
impl Comparison {
    fn validate(&self, name: &str) -> Result<()> {
        anyhow::ensure!(self.lemma_name == name, "comparison target changed");
        anyhow::ensure!(
            !self.explanation.trim().is_empty(),
            "comparison omitted its explanation"
        );
        validate_text(&[&self.discrepancy, &self.explanation])
    }
}
fn validate_text(values: &[&String]) -> Result<()> {
    anyhow::ensure!(
        values.iter().all(|value| value.len() <= 16_384),
        "semantic result field exceeds 16 KiB"
    );
    Ok(())
}
pub fn decode<T: DeserializeOwned>(reply: Reply, expected: &str) -> Result<T> {
    let mut calls = reply.tool_calls.into_iter();
    let call = calls
        .next()
        .context("meaning analysis needs exactly one structured result; no tools were executed")?;
    anyhow::ensure!(
        calls.next().is_none(),
        "meaning analysis needs exactly one structured result; no tools were executed"
    );
    anyhow::ensure!(
        call.function.name == expected && call.function.arguments.len() <= 65_536,
        "invalid meaning-analysis result function or size; no tools were executed"
    );
    serde_json::from_str(&call.function.arguments)
        .context("invalid structured meaning-analysis result")
}
fn messages(system: &str, data: &Value) -> Vec<Value> {
    vec![
        json!({"role":"system","content":system}),
        json!({"role":"user","content":data.to_string()}),
    ]
}
fn result_tool(name: &str, properties: &Value, required: &[&str]) -> Value {
    json!({"type":"function","function":{"name":name,"description":"Record candidate analysis data only; no side effects or authority", "parameters":{"type":"object","properties":properties,"required":required,"additionalProperties":false}}})
}
fn translation_tool() -> Value {
    result_tool(
        "record_informalization",
        &json!({
            "lemmaName":{"type":"string"}, "naturalLanguage":{"type":"string"},
            "preconditions":{"type":"string"}, "postcondition":{"type":"string"},
            "scope":{"type":"string"}, "strength":{"type":"string","enum":["trivial","weak","moderate","strong"]},
            "confidence":{"type":"number","minimum":0,"maximum":1},
        }),
        &[
            "lemmaName",
            "naturalLanguage",
            "preconditions",
            "postcondition",
            "scope",
            "strength",
            "confidence",
        ],
    )
}
fn comparison_tool() -> Value {
    result_tool(
        "record_comparison",
        &json!({
            "lemmaName":{"type":"string"}, "match":{"type":"boolean"},
            "discrepancy":{"type":"string"}, "explanation":{"type":"string"},
            "weakeningType":{"type":"string","enum":["none","tautology","weakened-postcondition","narrowed-scope","missing-case","wrong-property"]},
        }),
        &[
            "lemmaName",
            "match",
            "discrepancy",
            "weakeningType",
            "explanation",
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use meden_agent::conversation::{FunctionCall, ModelError, ToolCall};
    use std::cell::RefCell;
    use std::collections::{BTreeMap, VecDeque};

    struct Model<'a> {
        replies: RefCell<VecDeque<Reply>>,
        requests: RefCell<Vec<Vec<Value>>>,
        cancel: Option<&'a AtomicBool>,
    }
    impl ChatModel for Model<'_> {
        fn reply(
            &self,
            messages: &[Value],
            _tools: &[Value],
        ) -> std::result::Result<Reply, ModelError> {
            self.requests.borrow_mut().push(messages.to_vec());
            if let Some(cancel) = self.cancel {
                cancel.store(true, Ordering::SeqCst);
            }
            self.replies
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| ModelError::Unavailable("unexpected request".into()))
        }
    }
    fn output(name: &str, value: &Value) -> Reply {
        Reply {
            content: None,
            tool_calls: vec![ToolCall {
                id: "record".into(),
                function: FunctionCall {
                    name: name.into(),
                    arguments: value.to_string(),
                },
            }],
        }
    }
    fn translation() -> Reply {
        output(
            "record_informalization",
            &json!({"lemmaName":"Demo.bound","naturalLanguage":"Stock never increases.","preconditions":"stock is natural","postcondition":"out <= stock","scope":"single transition","strength":"moderate","confidence":0.9}),
        )
    }
    fn comparison(matches: bool, weakening: &str, discrepancy: &str) -> Reply {
        output(
            "record_comparison",
            &json!({"lemmaName":"Demo.bound","match":matches,"discrepancy":discrepancy,"weakeningType":weakening,"explanation":"exact requested comparison"}),
        )
    }
    fn source() -> Result<crate::analysis::sources::Snapshot> {
        let files = BTreeMap::from([(
            "Spec.lean".into(),
            "namespace Demo\ntheorem bound : FORMAL_ONLY_SENTINEL := by sorry\nend Demo".into(),
        )]);
        let source_revision =
            SourceRevision::from_digest(Sha256::digest(serde_json::to_vec(&files)?));
        Ok(crate::analysis::sources::Snapshot {
            revision: source_revision,
            files,
        })
    }
    fn prepared() -> Result<Prepared> {
        let source = source()?;
        Prepared::new(
            Input {
                source_revision: source.revision.clone(),
                graph_revision: None,
                target: Target {
                    file: "Spec.lean".into(),
                    line: 2,
                    name: "Demo.bound".into(),
                },
                requirement: "ORIGINAL_REQUIREMENT_SENTINEL".into(),
            },
            &source,
        )
    }
    #[test]
    fn requests_separate_requirement_and_formal_context_and_preserve_observed_mismatch(
    ) -> Result<()> {
        let model = Model {
            replies: RefCell::new(VecDeque::from([
                translation(),
                comparison(false, "missing-case", "The purchase condition is missing."),
            ])),
            requests: RefCell::default(),
            cancel: None,
        };
        let assessment = analyze(
            &model,
            &model,
            &prepared()?,
            &AtomicBool::new(false),
            &|_| {},
        )?;
        assert_eq!(assessment.verdict, Verdict::Mismatch);
        let requests = model.requests.borrow();
        let first = serde_json::to_string(requests.first().context("first request")?)?;
        let second = serde_json::to_string(requests.get(1).context("second request")?)?;
        assert!(first.contains("FORMAL_ONLY_SENTINEL"));
        assert!(!first.contains("ORIGINAL_REQUIREMENT_SENTINEL"));
        assert!(second.contains("ORIGINAL_REQUIREMENT_SENTINEL"));
        assert!(!second.contains("FORMAL_ONLY_SENTINEL"));
        assert!(second.contains("Stock never increases"));
        Ok(())
    }
    #[test]
    fn separate_judge_receives_only_the_requirement_and_blind_translation() -> Result<()> {
        let translator = Model {
            replies: RefCell::new(VecDeque::from([translation()])),
            requests: RefCell::default(),
            cancel: None,
        };
        let judge = Model {
            replies: RefCell::new(VecDeque::from([comparison(
                false,
                "missing-case",
                "The requested bound is missing.",
            )])),
            requests: RefCell::default(),
            cancel: None,
        };
        let result = analyze(
            &translator,
            &judge,
            &prepared()?,
            &AtomicBool::new(false),
            &|_| {},
        )?;
        assert_eq!(result.verdict, Verdict::Mismatch);
        assert_eq!(translator.requests.borrow().len(), 1);
        assert_eq!(judge.requests.borrow().len(), 1);
        let translation_input = serde_json::to_string(&*translator.requests.borrow())?;
        let judge_input = serde_json::to_string(&*judge.requests.borrow())?;
        assert!(translation_input.contains("FORMAL_ONLY_SENTINEL"));
        assert!(!translation_input.contains("ORIGINAL_REQUIREMENT_SENTINEL"));
        assert!(judge_input.contains("ORIGINAL_REQUIREMENT_SENTINEL"));
        assert!(!judge_input.contains("FORMAL_ONLY_SENTINEL"));
        assert!(judge_input.contains("Stock never increases"));
        Ok(())
    }
    #[test]
    fn cancellation_after_reply_prevents_comparison_and_never_admits_a_result() -> Result<()> {
        let cancel = AtomicBool::new(false);
        let model = Model {
            replies: RefCell::new(VecDeque::from([translation()])),
            requests: RefCell::default(),
            cancel: Some(&cancel),
        };
        assert!(analyze(&model, &model, &prepared()?, &cancel, &|_| {}).is_err());
        assert_eq!(model.requests.borrow().len(), 1);
        Ok(())
    }
    #[test]
    fn cancellation_in_progress_notification_prevents_even_the_first_request() -> Result<()> {
        let cancel = AtomicBool::new(false);
        let model = Model {
            replies: RefCell::default(),
            requests: RefCell::default(),
            cancel: None,
        };
        assert!(analyze(&model, &model, &prepared()?, &cancel, &|_| cancel
            .store(true, Ordering::SeqCst))
        .is_err());
        assert!(model.requests.borrow().is_empty());
        Ok(())
    }
    #[test]
    fn unknown_functions_wrong_targets_and_extra_authority_fields_are_not_results() -> Result<()> {
        for reply in [
            output(
                "write_file",
                &json!({"path":"outside","content":"not executed"}),
            ),
            output(
                "record_informalization",
                &json!({"lemmaName":"another-target"}),
            ),
            output(
                "record_informalization",
                &json!({"lemmaName":"Demo.bound","naturalLanguage":"fine","preconditions":"none","postcondition":"all","scope":"all","strength":"strong","confidence":1.0,"verified":true}),
            ),
        ] {
            let model = Model {
                replies: RefCell::new(VecDeque::from([reply])),
                requests: RefCell::default(),
                cancel: None,
            };
            assert!(analyze(
                &model,
                &model,
                &prepared()?,
                &AtomicBool::new(false),
                &|_| {}
            )
            .is_err());
            assert_eq!(model.requests.borrow().len(), 1);
        }
        Ok(())
    }
    #[test]
    fn a_match_with_a_reported_missing_case_is_unclear_not_a_match() -> Result<()> {
        let model = Model {
            replies: RefCell::new(VecDeque::from([
                translation(),
                comparison(true, "missing-case", "Missing purchase behavior"),
            ])),
            requests: RefCell::default(),
            cancel: None,
        };
        assert_eq!(
            analyze(
                &model,
                &model,
                &prepared()?,
                &AtomicBool::new(false),
                &|_| {}
            )?
            .verdict,
            Verdict::Unclear
        );
        Ok(())
    }
    #[test]
    fn replacement_source_or_an_unavailable_target_cannot_reuse_a_preparation() -> Result<()> {
        let prepared = prepared()?;
        assert!(Prepared::new(
            prepared.input.clone(),
            &crate::analysis::sources::Snapshot {
                revision: prepared.input.source_revision.clone(),
                files: BTreeMap::from([("Spec.lean".into(), "replacement".into())])
            },
        )
        .is_err());
        let mut changed = prepared.input.clone();
        changed.target.line = 100;
        assert!(Prepared::new(changed, &source()?).is_err());
        Ok(())
    }

    #[test]
    fn backend_context_is_serialized_once_and_later_mutation_cannot_rewrite_model_messages(
    ) -> Result<()> {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        struct Mutable {
            source: crate::analysis::sources::Snapshot,
            graph: GraphRevision,
            value: Arc<AtomicUsize>,
        }
        struct Context(Arc<AtomicUsize>);
        impl Serialize for Context {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                self.0.load(Ordering::SeqCst).serialize(serializer)
            }
        }
        impl crate::analysis::sources::Source for Mutable {
            fn sources(&self) -> &crate::analysis::sources::Snapshot {
                &self.source
            }
        }
        impl Observation for Mutable {
            type Context = Context;
            fn graph_revision(&self) -> Option<&GraphRevision> {
                Some(&self.graph)
            }
            fn context(&self, _: &Target) -> Result<Option<Context>> {
                Ok(Some(Context(self.value.clone())))
            }
        }
        let mut original = Mutable {
            source: source()?,
            graph: GraphRevision::from_digest(Sha256::digest(b"original graph")),
            value: Arc::new(AtomicUsize::new(12345)),
        };
        let mut input = prepared()?.input;
        input.graph_revision = Some(original.graph.clone());
        let frozen = Prepared::new(input, &original)?;
        let before = frozen.translation_messages();
        original.value.store(98765, Ordering::SeqCst);
        original.source.files.clear();
        assert_eq!(frozen.translation_messages(), before);
        let body = serde_json::to_string(&before)?;
        assert!(
            body.contains("12345")
                && !body.contains("98765")
                && !body.contains("ORIGINAL_REQUIREMENT_SENTINEL")
        );
        Ok(())
    }
}
