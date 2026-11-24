//! Producer-owned observation identities. These are not executable job tickets.
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Serialize, Serializer};
use sha2::{digest::Output, Sha256};
use std::{borrow::Cow, marker::PhantomData};

/// One owner's original job digest, distinct from another owner's observation
/// or a content publication. It has no external decoder or execution authority.
///
/// Preparation and verification observations cannot be interchanged:
/// ```compile_fail
/// use meden_app::view::{job_id::JobId, jobs, verification_jobs};
/// fn wrong(status: &jobs::Status) -> JobId<verification_jobs::Job> {
///     status.job.clone()
/// }
/// ```
/// Nor can an observation identity query a content publication:
/// ```compile_fail
/// use meden_app::view::jobs;
/// fn wrong(owner: &jobs::Jobs, status: &jobs::Status) {
///     owner.bytes(&status.job);
/// }
/// ```
/// A transport string cannot construct a producer observation:
/// ```compile_fail
/// use meden_app::view::{job_id::JobId, jobs};
/// let forged: JobId<jobs::Job> = serde_json::from_str("\"sha256:abc\"").unwrap();
/// ```
/// The matching producer's identity can be retained without decoding:
/// ```
/// use meden_app::view::{job_id::JobId, jobs};
/// fn retain(status: &jobs::Status) -> JobId<jobs::Job> {
///     status.job.clone()
/// }
/// ```
pub struct JobId<D> {
    value: String,
    domain: PhantomData<fn() -> D>,
}
impl<D> JobId<D> {
    /// Preserve the owner's already-computed digest and its original spelling.
    pub(super) fn from_digest(digest: Output<Sha256>) -> Self {
        Self {
            value: format!("sha256:{digest:x}"),
            domain: PhantomData,
        }
    }
}
impl<D> Clone for JobId<D> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            domain: PhantomData,
        }
    }
}
impl<D> std::fmt::Debug for JobId<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}
impl<D> PartialEq for JobId<D> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<D> Eq for JobId<D> {}
impl<D> Serialize for JobId<D> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.value)
    }
}
impl<D> JsonSchema for JobId<D> {
    fn schema_name() -> Cow<'static, str> {
        "JobId".into()
    }
    fn inline_schema() -> bool {
        true
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"string","pattern":"^sha256:[a-f0-9]{64}$"})
    }
}
