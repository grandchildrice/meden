//! The complete native session read model. Borrowing original publication owners
//! keeps status small and avoids cloning or materializing their retained bodies.
//! These fields are observations, never approval, job or evidence capabilities.
use crate::command::Operation;
use schemars::JsonSchema;
use serde::Serialize;

/// Host-discovered public labels; credentials and runnable providers stay private.
#[derive(Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    /// Original host connection key, not a runnable provider.
    pub id: String,
    /// Public display label.
    pub label: String,
    /// Current selected model label.
    pub model: String,
    /// Public host diagnostic; no credential material.
    pub note: String,
}

/// Read-only compiler availability. This metadata schema stays outside the
/// compiler's captured implementation sources, so GUI codec edits do not
/// invalidate otherwise identical compiler/search observations.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Availability<'a> {
    available: bool,
    prepared: bool,
    detail: &'a str,
}
impl<'a> From<&'a crate::atlas::Status> for Availability<'a> {
    fn from(original: &'a crate::atlas::Status) -> Self {
        Self {
            available: original.available,
            prepared: original.prepared,
            detail: &original.detail,
        }
    }
}

/// Exact serialized session view. Schema substitutions below name each owner's
/// actual lightweight serialization contract; they do not decode its internals.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct State<'a> {
    #[schemars(with = "super::history::Reference")]
    /// Lightweight original transcript/work publication reference.
    pub history: &'a super::history::History,
    /// Host workspace display path.
    pub workspace: &'a str,
    /// Current frozen processor display destination, empty before selection.
    pub endpoint: &'a str,
    /// Current selected model label.
    pub model: &'a str,
    /// Selected host connection key, absent before selection.
    pub provider: Option<&'a str>,
    /// Public discovery labels; original provider implementations stay with the host.
    pub connections: &'a [Choice],
    /// Exact positive selection counter, absent before a connection is selected.
    pub connection_revision: Option<super::sequence::Sequence>,
    /// Last host-observed model list.
    pub models: &'a [String],
    /// Original discovery failure, independent of inference or approval.
    pub models_error: Option<&'a str>,
    /// Original host key associated with the observed model list.
    pub models_provider: Option<&'a str>,
    #[schemars(with = "super::inventory::Status")]
    /// Prepared complete inventory availability.
    pub overview: &'a super::inventory::Status,
    /// Original optional second-model review target.
    pub judge: Option<&'a crate::model::Binding>,
    /// Observed gateway status, never an authority token.
    pub authorized: bool,
    /// Whether a host-prepared processor can be reviewed.
    pub approval_available: bool,
    /// Whether the host operation queue is occupied.
    pub running: bool,
    /// Category of the original host-selected operation.
    pub operation: Operation,
    /// Retained conversation stop diagnostic, independent of file inspection.
    pub chat_stopped: Option<&'a str>,
    /// Current original operation label.
    pub activity: &'a str,
    /// Current streamed assistant text.
    pub partial: &'a str,
    /// Current streamed reasoning observation, separate from evidence.
    pub reasoning: &'a str,
    #[schemars(with = "super::evidence::Status")]
    /// Original accepted/proposed meaning and evidence publication availability.
    pub project: &'a super::evidence::Reader,
    #[schemars(with = "super::verification_jobs::Status")]
    /// Actual project or historical-work check progress availability.
    pub verification_job: &'a super::verification_jobs::Reader,
    #[schemars(with = "super::verification_jobs::Status")]
    /// Explicit host work progress is separate from conversation activity.
    pub work_job: &'a super::verification_jobs::Reader,
    /// Last original operation diagnostic.
    pub stopped: Option<&'a str>,
    #[schemars(with = "super::directory::Index")]
    /// Exact complete folder-listing references.
    pub directories: &'a super::directory::Directories,
    #[schemars(with = "super::readers::Index")]
    /// Exact complete source-availability references.
    pub readers: &'a super::readers::Readers,
    #[schemars(with = "super::sources::Index")]
    /// Exact prepared graph/source publication references.
    pub sources: &'a super::sources::Sources,
    #[schemars(with = "super::jobs::Index")]
    /// Resource-local preparation observations and bounded log references.
    pub source_preparation: &'a super::jobs::Jobs,
    #[schemars(with = "super::analyses::Status")]
    /// Current and historical model-assessment availability, never proof.
    pub analyses: &'a super::analyses::Analyses,
    #[schemars(with = "Option<super::repairs::Data<'static>>")]
    /// Original host-selected patch review projection; no patch authority.
    pub repair_review: Option<&'a super::repairs::Review>,
    #[schemars(with = "super::preview::Status")]
    /// Description availability of the actual captured static application.
    pub preview: &'a super::preview::Reader,
    #[schemars(length(max = 8))]
    /// At most eight original immutable session proposals.
    pub exports: &'a [super::delivery::Summary],
    /// Original host-selected delivery page with its exact counter.
    pub export_review: Option<&'a super::delivery::Review>,
    /// Last original rejection or transport observation.
    pub export_outcome: Option<&'a super::delivery::Outcome>,
    /// Host-observed installed compiler availability; no extraction runs here.
    pub atlas: Availability<'a>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::{project::Project, records};
    use anyhow::Result;

    #[test]
    fn full_session_projection_keeps_large_history_outside_polling_and_counters_exact() -> Result<()>
    {
        let root = tempfile::tempdir()?;
        let project = Project::new(root.path()).expect("valid fixture workspace");
        project.stored_status()?;
        let evidence = project.evidence_view()?;
        let jobs = project.verification_view();
        let history = super::super::history::History::new(super::super::history::Data {
            portable: records::View {
                document: None,
                error: None,
                lean_available: false,
                lean: String::new(),
            },
            messages: vec![records::Message {
                speaker: records::Speaker::User,
                text: "ORIGINAL-HISTORY-BODY\n".repeat(65_536),
            }],
            logs: Vec::new(),
        });
        let overview = super::super::inventory::Status::default();
        let directories = super::super::directory::Directories::default();
        let readers = super::super::readers::Readers::default();
        let sources = super::super::sources::Sources::default();
        let preparation = super::super::jobs::Jobs::new(b"original-host");
        let analyses = super::super::analyses::Analyses::new(super::super::analyses::Records {
            current: None,
            history: Vec::new(),
        });
        let previews = super::super::preview::Previews::default();
        let preview = previews.reader();
        let atlas = crate::atlas::Status {
            available: false,
            prepared: false,
            detail: "fixture observation only".into(),
        };
        let mut state = State {
            history: &history,
            workspace: "/original/workspace",
            endpoint: "",
            model: "",
            provider: None,
            connections: &[],
            connection_revision: None,
            models: &[],
            models_error: None,
            models_provider: None,
            overview: &overview,
            judge: None,
            authorized: false,
            approval_available: false,
            running: false,
            operation: Operation::Workspace,
            chat_stopped: None,
            activity: "Ready",
            partial: "",
            reasoning: "",
            project: &evidence,
            verification_job: &jobs,
            work_job: &jobs,
            stopped: None,
            directories: &directories,
            readers: &readers,
            sources: &sources,
            source_preparation: &preparation,
            analyses: &analyses,
            repair_review: None,
            preview: &preview,
            exports: &[],
            export_review: None,
            export_outcome: None,
            atlas: (&atlas).into(),
        };
        assert_eq!(
            serde_json::to_value(&state)?.get("connection_revision"),
            Some(&serde_json::Value::Null)
        );
        state.connection_revision = Some(super::super::sequence::Sequence::new(u64::MAX)?);
        let bytes = serde_json::to_vec(&state)?;
        assert!(bytes.len() < 8192);
        assert!(!String::from_utf8(bytes.clone())?.contains("ORIGINAL-HISTORY-BODY"));
        let observation: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            observation.get("connection_revision"),
            Some(&serde_json::json!(u64::MAX.to_string()))
        );
        assert_eq!(observation.get("judge"), Some(&serde_json::Value::Null));
        assert_eq!(
            observation.pointer("/verification_job/state"),
            Some(&serde_json::json!("idle"))
        );
        assert_eq!(
            observation.get("history"),
            Some(&serde_json::to_value(&history)?)
        );
        for _ in 0..20 {
            assert_eq!(serde_json::to_vec(&state)?, bytes);
        }
        assert_eq!(history.data().messages.len(), 1);
        Ok(())
    }
}
