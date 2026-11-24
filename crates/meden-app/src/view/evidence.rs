//! Backend-neutral reviewed intent and evidence observations. Neither decoded
//! references nor a published Current value construct a kernel receipt/grant.
use super::publication::{Envelope, Publication};
use crate::{project, verification::Contract};
use anyhow::{Context, Result};
use meden_runtime::verification::{artifact::Class, FailureKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Lossless transport of the original selected contract identity, never authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    #[serde(deserialize_with = "revision")]
    #[schemars(regex(pattern = r"^[1-9][0-9]{0,19}$"))]
    revision: String,
    #[schemars(length(min = 1, max = 8192))]
    session: String,
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    contract: String,
}
fn revision<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    let number = value.parse::<u64>().map_err(serde::de::Error::custom)?;
    if number == 0 || number.to_string() != value {
        return Err(serde::de::Error::custom(
            "Contract revision must be a canonical positive u64 decimal",
        ));
    }
    Ok(value)
}
impl Reference {
    /// Observe an original host/controller reference without changing its identity.
    pub fn new(original: &project::Reference) -> Result<Self> {
        anyhow::ensure!(
            original.revision > 0
                && !original.session.is_empty()
                && original.session.len() <= 8192
                && digest(&original.contract),
            "Invalid contract reference"
        );
        Ok(Self {
            revision: original.revision.to_string(),
            session: original.session.clone(),
            contract: original.contract.clone(),
        })
    }
    /// Recover only the exact original numbers/bytes. The existing controller
    /// must still match the selected host review before accepting any operation.
    pub fn original(self) -> Result<project::Reference> {
        let revision = self.revision.parse::<u64>()?;
        anyhow::ensure!(
            revision > 0
                && revision.to_string() == self.revision
                && !self.session.is_empty()
                && self.session.len() <= 8192
                && digest(&self.contract),
            "Invalid contract reference"
        );
        Ok(project::Reference {
            revision,
            session: self.session,
            contract: self.contract,
        })
    }
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Complete original reviewed data; backend syntax is opaque text to the viewer.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Agreement {
    reference: Reference,
    name: String,
    source_path: String,
    interface: String,
    backend: String,
    class: Class,
    scope: String,
    /// Exact canonical contract bytes as UTF-8, bound to reference.contract.
    #[schemars(length(max = 65_536))]
    canonical: String,
}
impl Agreement {
    fn new(contract: &Contract, reference: &project::Reference) -> Result<Self> {
        anyhow::ensure!(
            contract.identity()? == reference.contract,
            "Contract content identity changed"
        );
        let description = contract.description();
        let canonical = String::from_utf8(contract.canonical()?)?;
        anyhow::ensure!(canonical.len() <= 65_536, "Contract exceeds view bounds");
        Ok(Self {
            reference: Reference::new(reference)?,
            name: description.name.clone(),
            source_path: description.source.clone(),
            interface: description.interface.clone(),
            backend: contract.backend().to_owned(),
            class: description.class,
            scope: description.scope.clone(),
            canonical,
        })
    }
}
/// Retained checker output binding; its existence alone never establishes freshness.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    directory: String,
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    source_sha256: String,
    #[schemars(regex(pattern = r"^[a-f0-9]{64}$"))]
    artifact_sha256: String,
}
/// The original host classification, never inferred from diagnostic words.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    kind: FailureKind,
    message: String,
}
/// Observation of the accepted contract's evidence, independent of graph/meaning.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Evidence {
    /// The host has not established current coverage; old outputs remain inspectable.
    Unverified {
        /// Original retained artifact, without a current-coverage claim.
        retained: Option<Binding>,
        /// Independently attributed original failure.
        failure: Option<Failure>,
    },
    /// A fresh host/controller observation reported complete admitted coverage.
    /// This display value is not a receipt and cannot authorize delivery.
    Current {
        /// Original strength-specific completion text from the application.
        completion_label: String,
        /// Exact retained source/output binding for this accepted agreement.
        artifact: Binding,
    },
}
/// Impossible combinations such as current evidence without acceptance are absent.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Data {
    /// No contract has been proposed or accepted.
    Empty,
    /// Displayed proposal, with any separately retained original accepted meaning.
    Proposed {
        /// Exact selected proposal and its original contract identity.
        contract: Agreement,
        /// Accepted meaning is retained until an explicit host replacement acceptance.
        accepted: Option<Agreement>,
        /// Complete original review body, including replacement context.
        review: String,
    },
    /// Accepted meaning and a separately classified evidence observation.
    Accepted {
        /// Exact original accepted agreement.
        contract: Agreement,
        /// Complete original review body.
        review: String,
        /// Host observation, never authority decoded from the browser.
        evidence: Evidence,
    },
}
impl Data {
    fn new(summary: &project::Summary) -> Result<Self> {
        let accepted = match (&summary.accepted_contract, &summary.accepted_reference) {
            (Some(contract), Some(reference)) => Some(Agreement::new(contract, reference)?),
            (None, None) => None,
            _ => anyhow::bail!("Accepted contract/reference mismatch"),
        };
        let Some(contract) = &summary.contract else {
            anyhow::ensure!(
                !summary.proposed && !summary.accepted && !summary.verified && accepted.is_none(),
                "Missing displayed agreement"
            );
            return Ok(Self::Empty);
        };
        anyhow::ensure!(
            summary.proposed,
            "Displayed contract missing proposal observation"
        );
        let displayed = Agreement::new(contract, &summary.reference)?;
        anyhow::ensure!(
            summary.source_path == displayed.source_path
                && summary.scope == displayed.scope
                && summary.evidence_class == Some(displayed.class),
            "Displayed contract scope changed"
        );
        if !summary.accepted {
            anyhow::ensure!(
                !summary.verified,
                "Unaccepted contract cannot have current evidence"
            );
            return Ok(Self::Proposed {
                contract: displayed,
                accepted,
                review: summary.review.clone(),
            });
        }
        let accepted = accepted.context("Accepted agreement missing")?;
        anyhow::ensure!(
            accepted.reference == displayed.reference,
            "Accepted/displayed identity mismatch"
        );
        let artifact = match (
            &summary.artifact,
            &summary.source_sha256,
            &summary.artifact_sha256,
        ) {
            (Some(path), Some(source), Some(output)) => {
                anyhow::ensure!(
                    digest(source) && digest(output),
                    "Invalid evidence input/output binding"
                );
                Some(Binding {
                    directory: path
                        .to_str()
                        .context("Binding path is not UTF-8")?
                        .to_owned(),
                    source_sha256: source.clone(),
                    artifact_sha256: output.clone(),
                })
            }
            (None, None, None) => None,
            _ => anyhow::bail!("Incomplete evidence artifact binding"),
        };
        let failure = match (
            summary.verification_failure,
            &summary.verification_diagnostic,
        ) {
            (Some(kind), Some(message)) => Some(Failure {
                kind,
                message: message.clone(),
            }),
            (None, None) => None,
            _ => anyhow::bail!("Incomplete verification failure observation"),
        };
        let evidence = if summary.verified {
            anyhow::ensure!(
                failure.is_none(),
                "Current evidence has a failed observation"
            );
            Evidence::Current {
                completion_label: summary.completion_label.into(),
                artifact: artifact.context("Current evidence has no bound artifact")?,
            }
        } else {
            Evidence::Unverified {
                retained: artifact,
                failure,
            }
        };
        Ok(Self::Accepted {
            contract: accepted,
            review: summary.review.clone(),
            evidence,
        })
    }
}
/// Complete exact evidence body; it never transfers a kernel receipt.
pub type Snapshot = Envelope<Data>;
/// Frequent polling reports only exact prepared availability.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Status {
    /// Complete host observation is frozen by content identity.
    Ready {
        /// Exact original display body, not an authority token.
        #[schemars(regex(pattern = r"^sha256:[a-f0-9]{64}$"))]
        revision: String,
    },
    /// Original observation is retained but no partial/superseded body is admitted.
    Unavailable {
        /// Bounded host projection diagnostic.
        #[schemars(length(max = 8000))]
        message: String,
    },
}
/// One owner for original host observations and their immutable publication.
pub struct Project {
    original: project::Summary,
    publication: Option<Publication>,
    status: Status,
}
impl Project {
    /// Observe a host summary; no decoder or renderer establishes its authority.
    pub fn new(original: project::Summary) -> Self {
        let mut value = Self {
            original,
            publication: None,
            status: Status::Unavailable {
                message: "Project observation is not prepared".into(),
            },
        };
        value.refresh();
        value
    }
    /// Freeze at the original host observation boundary, never on a query.
    pub fn observe(&mut self, original: project::Summary) {
        self.original = original;
        self.refresh();
    }
    /// Source edits remove the display's current-coverage observation. Actual
    /// evidence eligibility and fresh delivery checks remain in the controller.
    pub fn invalidate(&mut self) {
        if self.original.verified {
            self.original.verified = false;
            self.refresh();
        }
    }
    /// Exact retained bytes only, without source inspection or a checker call.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        self.publication.as_ref()?.bytes(revision)
    }
    fn prepare(&self) -> Result<Publication> {
        let _timing = meden_runtime::metrics::Span::start("view.evidence.materialize");
        let data = Data::new(&self.original)?;
        match &self.publication {
            Some(old) => old.update(b"meden.project-evidence.v1\0", &data),
            None => Publication::new(b"meden.project-evidence.v1\0", &data),
        }
    }
    fn refresh(&mut self) {
        match self.prepare() {
            Ok(publication) => {
                self.status = Status::Ready {
                    revision: publication.reference().revision.clone(),
                };
                self.publication = Some(publication);
            }
            Err(error) => {
                self.publication = None;
                self.status = Status::Unavailable {
                    message: super::publication::diagnostic(
                        &format!("Project observation unavailable: {error:#}"),
                        "\n… (diagnostic shortened)",
                    ),
                };
            }
        }
    }
}
impl Serialize for Project {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.status.serialize(serializer)
    }
}

#[cfg(test)]
mod tests;
