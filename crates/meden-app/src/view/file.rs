//! Complete bounded file observations, independent of compilation.
use super::SourceText;
use anyhow::Result;
use meden_agent::tool::{Args, ToolError};
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};

/// Every admitted text input is complete within this byte budget.
pub const MAX_BYTES: usize = 1_048_576;

/// Worker-owned content facts. Host stops and failures are separate outcomes.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Content {
    /// Complete UTF-8 text.
    Text {
        /// Identity of every original input byte.
        #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
        revision: String,
        /// Original byte count, bounded before publication.
        #[schemars(range(max = 1_048_576))]
        bytes: u32,
        /// Complete original UTF-8 input.
        text: SourceText,
    },
    /// NUL-containing input, excluding recognized unsupported text BOMs.
    Binary {
        /// Complete observed byte count.
        #[schemars(range(max = 1_048_576))]
        bytes: u32,
    },
    /// Invalid UTF-8 or a recognized unsupported UTF-16 BOM.
    UnsupportedEncoding {
        /// Complete observed byte count.
        #[schemars(range(max = 1_048_576))]
        bytes: u32,
    },
    /// A pinned file exceeds the reader budget; no prefix is published.
    SizeLimit {
        /// Canonical decimal preserves large sizes across JSON/JavaScript.
        #[schemars(regex(pattern = r"^(0|[1-9][0-9]{0,19})$"))]
        bytes: String,
    },
}
impl Content {
    /// Readable text retains its full input identity after projection.
    pub fn revision(&self) -> Option<&str> {
        match self {
            Self::Text { revision, .. } => Some(revision),
            _ => None,
        }
    }
    /// Display classification is a bounded policy, not a general MIME detector.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        anyhow::ensure!(bytes.len() <= MAX_BYTES, "Incomplete file observation");
        let count = u32::try_from(bytes.len())?;
        if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
            return Ok(Self::UnsupportedEncoding { bytes: count });
        }
        if bytes.contains(&0) {
            return Ok(Self::Binary { bytes: count });
        }
        let revision = format!("sha256:{:x}", Sha256::digest(&bytes));
        let Ok(source) = String::from_utf8(bytes) else {
            return Ok(Self::UnsupportedEncoding { bytes: count });
        };
        Ok(Self::Text {
            revision,
            bytes: count,
            text: SourceText::new(source)?,
        })
    }
    /// Validate the complete wire input before indexing or projecting it.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Text {
                revision,
                bytes,
                text,
            } => anyhow::ensure!(
                text.len() <= MAX_BYTES
                    && usize::try_from(*bytes)? == text.len()
                    && !text.as_bytes().contains(&0)
                    && *revision == format!("sha256:{:x}", Sha256::digest(text.as_bytes())),
                "Invalid complete text observation"
            ),
            Self::Binary { bytes } | Self::UnsupportedEncoding { bytes } => anyhow::ensure!(
                *bytes <= u32::try_from(MAX_BYTES)?,
                "Invalid file byte count"
            ),
            Self::SizeLimit { bytes } => {
                let count = bytes.parse::<u64>()?;
                anyhow::ensure!(
                    count > u64::try_from(MAX_BYTES)? && count.to_string() == *bytes,
                    "Invalid file size-limit observation"
                );
            }
        }
        Ok(())
    }
}
/// Host-observed failure class; diagnostics do not determine it.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// A completed filesystem operation failed.
    Operation,
    /// Original or resolved file authority was refused.
    Refused,
    /// Required protocol/infrastructure is unavailable.
    Unavailable,
    /// A host limit or incomplete process prevented a complete observation.
    Interrupted,
    /// The host observed cancellation.
    Cancelled,
}
/// A reader outcome has content or a host failure, never both.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reading {
    /// Successful complete input, retained for display.
    Ready {
        /// Original data facts and complete text, when readable.
        content: Content,
    },
    /// No incomplete or failed bytes were admitted.
    Unavailable {
        /// Host class, independent of diagnostic text.
        failure: FailureKind,
        /// Bounded diagnostic; not a verdict or authority.
        #[schemars(length(max = 8000))]
        message: String,
    },
}
/// Source readability and lexical navigation support are independent.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Navigation {
    /// A registered lexical format can provide declaration navigation.
    Declarations,
    /// Text is readable, but rich navigation is unsupported.
    SourceOnly,
}
/// Shared generated read model for a complete explicitly observed file.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileView {
    /// Original logical workspace path, not a worker-selected filename.
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
    /// Capability only; not a proof or compilation result.
    pub navigation: Navigation,
    /// Complete content or an explicit host failure.
    pub reading: Reading,
}
impl FileView {
    /// Only readable text can start a separate graph preparation use case.
    pub fn is_text(&self) -> bool {
        matches!(
            &self.reading,
            Reading::Ready {
                content: Content::Text { .. }
            }
        )
    }
}
/// Uses the original/resolved policy-bound offline file worker. Rendering and
/// serializing the returned data performs no filesystem/compiler/model I/O.
pub fn observe(workspace: &Workspace, control: &Control, path: &str) -> Result<FileView> {
    anyhow::ensure!(
        !path.is_empty() && path.len() <= 4096 && !path.contains('\0'),
        "Invalid source view path"
    );
    let result = (|| -> Result<Content, ToolError> {
        let resolved = crate::workspace::logical_path(path)
            .ok_or_else(|| ToolError::refused("Source path leaves /workspace".into()))?;
        let request = Request::new(
            Operation::Snapshot,
            resolved.to_string_lossy().as_ref().into(),
            Args::from_iter([("observation".into(), true.into())]),
        );
        let value = meden_runtime::file::run(
            workspace,
            control,
            &request,
            &crate::workspace::file_policy().map_err(|e| ToolError::unavailable(e.to_string()))?,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 8_388_608,
                output_bytes: 8_388_608,
            },
        )?;
        let content: Content =
            serde_json::from_value(value).map_err(|e| ToolError::unavailable(e.to_string()))?;
        content
            .validate()
            .map_err(|e| ToolError::unavailable(e.to_string()))?;
        Ok(content)
    })();
    let reading = match result {
        Ok(content) => Reading::Ready { content },
        Err(error) => {
            let failure = match error.parts() {
                Ok(Ok(_)) => FailureKind::Operation,
                Ok(Err(_)) => FailureKind::Refused,
                Err(Some(Ok(_))) => FailureKind::Unavailable,
                Err(Some(Err(_))) => FailureKind::Interrupted,
                Err(None) => FailureKind::Cancelled,
            };
            Reading::Unavailable {
                failure,
                message: error.to_string().chars().take(8000).collect(),
            }
        }
    };
    Ok(FileView {
        path: path.into(),
        navigation: if crate::adapters::source::supports(Path::new(path)) {
            Navigation::Declarations
        } else {
            Navigation::SourceOnly
        },
        reading,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context as _;

    #[test]
    fn complete_bytes_classify_without_publishing_partial_or_invalid_text() -> Result<()> {
        for (input, kind) in [
            (b"\"literal JSON string\"".to_vec(), "text"),
            (b"1: literal source\n".to_vec(), "text"),
            (Vec::new(), "text"),
            (b"a\0b".to_vec(), "binary"),
            (vec![0xff, 0xfe, 0x61, 0], "unsupported_encoding"),
            (vec![0xfe, 0xff, 0, 0x61], "unsupported_encoding"),
            (vec![0xff], "unsupported_encoding"),
        ] {
            let content = Content::from_bytes(input.clone())?;
            content.validate()?;
            let value = serde_json::to_value(&content)?;
            assert_eq!(value.get("kind"), Some(&serde_json::json!(kind)));
            let roundtrip: Content = serde_json::from_value(value)?;
            roundtrip.validate()?;
            if let Content::Text { text, .. } = roundtrip {
                assert_eq!(text.as_bytes(), input);
            }
        }
        let content = Content::from_bytes(vec![b'a'; MAX_BYTES])?;
        content.validate()?;
        assert!(Content::from_bytes(vec![b'a'; MAX_BYTES + 1]).is_err());
        Ok(())
    }

    #[test]
    fn original_byte_identity_and_canonical_size_are_checked() -> Result<()> {
        let original = Content::from_bytes("日本語".as_bytes().to_vec())?;
        let mut value = serde_json::to_value(original)?;
        *value.get_mut("text").context("text")? = "日本人".into();
        assert!(serde_json::from_value::<Content>(value.clone())?
            .validate()
            .is_err());
        *value.get_mut("bytes").context("bytes")? = 3.into();
        assert!(serde_json::from_value::<Content>(value)?
            .validate()
            .is_err());
        for size in ["0", "1048576", "01048577", "-1", "18446744073709551616"] {
            assert!(Content::SizeLimit { bytes: size.into() }
                .validate()
                .is_err());
        }
        Content::SizeLimit {
            bytes: "18446744073709551615".into(),
        }
        .validate()?;
        Ok(())
    }
}
