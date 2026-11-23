//! Bounded file inputs and paged presentation, independent of compilation.
use super::SourceText;
use anyhow::{Context, Result};
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
/// Presentation page size, independent of a compiler's graph scope.
pub const PAGE_LINES: u32 = 200;

/// Worker-owned content facts. Host stops and failures are separate outcomes.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Content<T = SourceText> {
    /// Complete UTF-8 text, or a pure projection of that complete input.
    Text {
        /// Identity of every original input byte, not just a page.
        #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
        revision: String,
        /// Original byte count, bounded before publication.
        #[schemars(range(max = 1_048_576))]
        bytes: u32,
        /// Complete input or its bounded presentation page.
        text: T,
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
impl<T> Content<T> {
    /// Readable text retains its full input identity after projection.
    pub fn revision(&self) -> Option<&str> {
        match self {
            Self::Text { revision, .. } => Some(revision),
            _ => None,
        }
    }
    /// Preserve metadata and non-text states while deriving a view.
    pub fn try_map_text<U, E>(
        self,
        project: impl FnOnce(T) -> Result<U, E>,
    ) -> Result<Content<U>, E> {
        Ok(match self {
            Self::Text {
                revision,
                bytes,
                text,
            } => Content::Text {
                revision,
                bytes,
                text: project(text)?,
            },
            Self::Binary { bytes } => Content::Binary { bytes },
            Self::UnsupportedEncoding { bytes } => Content::UnsupportedEncoding { bytes },
            Self::SizeLimit { bytes } => Content::SizeLimit { bytes },
        })
    }
}
impl Content {
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
                usize::try_from(*bytes)? == text.len()
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
/// Bounded page derived from a complete UTF-8 input; EOF is explicit.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Page {
    /// Actual first line, clamped when a requested line no longer exists.
    #[schemars(range(min = 1, max = 1_048_577))]
    pub offset: u32,
    /// Includes the empty last editor line after a trailing newline.
    #[schemars(range(min = 1, max = 1_048_577))]
    pub line_count: u32,
    /// Absent at EOF; an empty extra page is never offered.
    #[schemars(range(min = 1, max = 1_048_577))]
    pub next_offset: Option<u32>,
    /// Original text only; line numbers are presentation, not parsed strings.
    pub text: SourceText,
}
impl Page {
    fn new(source: &SourceText, requested: u32) -> Result<Self> {
        // A complete SourceText is at most 1 MiB, so these conversions and
        // source slices remain bounded independently of the requested offset.
        let line_count = u32::try_from(source.split('\n').count())?;
        let offset = requested.max(1).min(line_count);
        let rows: Vec<_> = source
            .split('\n')
            .skip(usize::try_from(offset.saturating_sub(1))?)
            .take(usize::try_from(PAGE_LINES)?)
            .collect();
        let end = offset
            .checked_add(u32::try_from(rows.len())?)
            .context("Source page overflow")?;
        Ok(Self {
            offset,
            line_count,
            next_offset: (end <= line_count).then_some(end),
            text: SourceText(rows.join("\n")),
        })
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
    /// Successful complete input, projected for display.
    Ready {
        /// Original data facts and the requested page.
        content: Content<Page>,
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
/// Shared generated read model for an explicitly requested source page.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileView {
    /// Original logical workspace path, not a worker-selected filename.
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
    /// Capability only; not a proof or compilation result.
    pub navigation: Navigation,
    /// Complete projected content or an explicit host failure.
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
pub fn observe(
    workspace: &Workspace,
    control: &Control,
    path: &str,
    offset: u32,
) -> Result<FileView> {
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
    let result = result.and_then(|content| {
        content.try_map_text(|source| {
            Page::new(&source, offset).map_err(|e| ToolError::unavailable(e.to_string()))
        })
    });
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

    #[test]
    fn pages_keep_literal_unicode_lines_and_stop_at_the_actual_end() -> Result<()> {
        let source = SourceText::new(
            (1..=400)
                .map(|i| format!("{i}: λ日本語"))
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
        let first = Page::new(&source, 0)?;
        assert_eq!(
            (first.offset, first.line_count, first.next_offset),
            (1, 400, Some(201))
        );
        assert!(first.text.starts_with("1: λ日本語\n2: λ日本語"));
        let last = Page::new(&source, 201)?;
        assert_eq!((last.offset, last.next_offset), (201, None));
        assert_eq!(last.text.split('\n').count(), 200);
        let clamped = Page::new(&source, u32::MAX)?;
        assert_eq!((clamped.offset, clamped.next_offset), (400, None));
        assert_eq!(&*clamped.text, "400: λ日本語");
        let empty = Page::new(&SourceText::new(String::new())?, 1)?;
        assert_eq!((empty.line_count, empty.next_offset), (1, None));
        let crlf = Page::new(&SourceText::new("λ\r\n".into())?, 1)?;
        assert_eq!((crlf.line_count, crlf.next_offset), (2, None));
        assert_eq!(&*crlf.text, "λ\r\n");
        Ok(())
    }
}
