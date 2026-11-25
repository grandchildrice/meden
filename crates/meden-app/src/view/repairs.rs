//! Original host patch review. The immutable patch reference retains authority
//! binding in the existing repair controller; this projection grants nothing.
use crate::analysis::repair;
use anyhow::Result;
use schemars::JsonSchema;
use serde::Serialize;

/// Required fields and original diagnostics, with a lossless accepted reference.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Data<'a> {
    reference: &'a repair::Reference,
    model: &'a str,
    processor: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_definition: Option<&'a serde_json::Value>,
    finding_id: &'a str,
    requirement_id: &'a str,
    input: &'a crate::semantic::Input,
    file: &'a str,
    accepted_reference: Option<&'a super::evidence::Reference>,
    explanation: &'a str,
    diff: &'a str,
}
/// Retains the actual original host selection, independently of its display.
pub struct Review {
    original: repair::Review,
    accepted: Option<super::evidence::Reference>,
}
impl Review {
    /// Convert only the reference representation, preserving the original queue data.
    pub fn new(original: repair::Review) -> Self {
        let accepted = original
            .accepted_reference
            .as_ref()
            .map(super::evidence::Reference::new);
        Self { original, accepted }
    }
    /// Original host selection used by existing apply/reject checks.
    pub fn original(&self) -> &repair::Review {
        &self.original
    }
}
impl Serialize for Review {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let original = &self.original;
        Data {
            reference: &original.reference,
            model: &original.model,
            processor: &original.processor,
            model_definition: original.model_definition.as_ref(),
            finding_id: &original.finding_id,
            requirement_id: &original.requirement_id,
            input: &original.input,
            file: &original.file,
            accepted_reference: self.accepted.as_ref(),
            explanation: &original.explanation,
            diff: &original.diff,
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{Input, Target};
    #[test]
    fn presentation_preserves_original_patch_and_journal_reference() -> Result<()> {
        let original = repair::Review {
            reference: repair::Reference {
                id: format!("repair-{}", "a".repeat(64)),
                sha256: format!("sha256:{}", "b".repeat(64)),
            },
            model: "original-model".into(),
            processor: "original-processor".into(),
            model_definition: Some(serde_json::json!({"opaque": [null, true, "λ"]})),
            finding_id: "finding".into(),
            requirement_id: "requirement".into(),
            input: Input {
                source_revision: crate::identity::snapshots::SourceRevision::parse(&format!(
                    "sha256:{}",
                    "c".repeat(64)
                ))?,
                graph_revision: None,
                target: Target {
                    file: "Spec.lean".into(),
                    line: 1,
                    name: "original".into(),
                },
                requirement: "Original intent".into(),
            },
            file: "Spec.lean".into(),
            accepted_reference: Some(crate::project::Reference {
                revision: crate::identity::RequirementRevision::new(u64::MAX)?,
                session: crate::identity::SessionId::parse("host")?,
                contract: crate::identity::ContractId::parse(&"d".repeat(64))?,
            }),
            explanation: "Original explanation\nλ".into(),
            diff: "-old\n+new\n".into(),
        };
        let journal = serde_json::to_value(&original)?;
        let view = Review::new(original);
        assert_eq!(serde_json::to_value(view.original())?, journal);
        let mut expected = journal;
        *expected
            .get_mut("accepted_reference")
            .and_then(|value| value.get_mut("revision"))
            .ok_or_else(|| anyhow::anyhow!("Original accepted revision missing"))? =
            u64::MAX.to_string().into();
        assert_eq!(serde_json::to_value(&view)?, expected);
        Ok(())
    }
}
