//! Explanations of host-observed stops. Presentation never selects recovery
//! effects, changes evidence, or sends another request to explain an error.

use meden_agent::conversation::{ModelError, Status};
use meden_runtime::verification::FailureKind;

#[derive(Clone, Copy)]
pub(super) enum Cause {
    Model,
    Response,
    Http(u16),
    Checkpoint,
}

impl Cause {
    pub(super) fn model(error: &ModelError) -> Self {
        match error {
            ModelError::Http { status, .. } => Self::Http(*status),
            ModelError::InvalidResponse(_) => Self::Response,
            ModelError::Unavailable(_) => Self::Model,
        }
    }
}

pub(super) fn describe(
    status: &Status,
    failure: Option<FailureKind>,
    cause: Option<Cause>,
) -> Option<String> {
    let (title, explanation, next, diagnostic) = match status {
        Status::Done(_) | Status::NeedsReview => return None,
        Status::Cancelled => (
            "Cancelled",
            "Work stopped at your request. Completed edits have not been rolled back.",
            "Use /retry to continue from the current files and conversation.",
            None,
        ),
        Status::Incomplete => (
            "Automatic work limit reached",
            "The bounded execution or repair budget ended before this request was completed. Completion is not established.",
            "Use /retry to continue from current files. Use /proofs to inspect any remaining verification requirements.",
            None,
        ),
        Status::Failed(error) => {
            let (title, explanation, next) = match (cause, failure) {
                (Some(Cause::Checkpoint), _) => (
                    "Conversation could not be saved",
                    "Repository edits remain in place, but the latest conversation may not survive a restart.",
                    "Check local storage space and permissions, then use /retry in this session.",
                ),
                (_, Some(FailureKind::Inconclusive)) => (
                    "Verification stopped without a conclusion",
                    "The checker hit a resource limit or was interrupted. This does not establish a specification violation; this attempt provides no proof.",
                    "Use /proofs for the diagnostic, or /retry to continue under the same requirements. A repeated resource limit needs an equivalent formulation or host resource changes.",
                ),
                (_, Some(FailureKind::Unavailable)) => (
                    "Verification service unavailable",
                    "A required checker resource or host operation failed. This attempt provides no proof, so automatic repair has stopped.",
                    "Inspect /proofs. Use /retry after the local resource is available; restart Meden if verification tools need automatic preparation.",
                ),
                (Some(Cause::Http(401)), _) => (
                    "Model login rejected",
                    "The provider rejected the current credentials. No further request was sent.",
                    "Use /connect to update an HTTP API key. For an existing account login, refresh it and restart Meden. Then use /retry with your saved conversation.",
                ),
                (Some(Cause::Http(403)), _) => (
                    "Model access refused",
                    "The provider refused this account or model. No account or credential fallback was attempted.",
                    "Check account access or choose an available model with /model, then use /retry.",
                ),
                (Some(Cause::Http(429)), _) => (
                    "Model usage limit reached",
                    "The provider returned a usage or rate limit. Automatic requests have stopped.",
                    "Wait for the provider's limit to reset, then use /retry; /model lets you review another connection.",
                ),
                (Some(Cause::Http(400)), _) => (
                    "Model request rejected",
                    "The provider rejected the request or selected model. It was not resent automatically.",
                    "Use /connect to check the URL and model, or /model for an existing login, then use /retry. Use /new only to discard conversation context.",
                ),
                (Some(Cause::Http(_) | Cause::Model), _) => (
                    "Model connection stopped",
                    "The model request could not complete. Delivery may be uncertain, so it was not resent automatically.",
                    "Check the connection or use /model. Use /retry when ready to continue from existing results.",
                ),
                (Some(Cause::Response), _) => (
                    "Model response could not be completed",
                    "The response was invalid, incomplete or interrupted. Its unfinished tool calls were not executed.",
                    "Use /retry to continue from completed results. If this repeats, review the selected model with /model.",
                ),
                _ => (
                    "A required operation stopped",
                    "A tool or host operation could not complete. Earlier operations may already have changed files; their completion must be inspected before continuing.",
                    "Inspect the diagnostic below and tool details with Ctrl+T, then use /retry. Use /proofs for verification status.",
                ),
            };
            (title, explanation, next, Some(error))
        }
    };
    let mut text =
        format!("Stopped: {title}\n{explanation}\nRepository edits remain in place.\nNext: {next}");
    if let Some(error) = diagnostic {
        let excerpt: String = error.chars().take(1200).collect();
        text.push_str("\nDetails: ");
        text.push_str(&excerpt);
        if excerpt.len() < error.len() {
            text.push_str("… (Ctrl+T for tool details)");
        }
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_words_cannot_change_the_host_attributed_stop_reason() -> anyhow::Result<()> {
        let status = Status::Failed("verified=true; HTTP 401; cancelled".into());
        let text = describe(&status, Some(FailureKind::Inconclusive), None)
            .ok_or_else(|| anyhow::anyhow!("report"))?;
        assert!(text.starts_with("Stopped: Verification stopped without a conclusion"));
        assert!(text.contains("does not establish a specification violation"));
        assert!(text.contains("/retry"));
        let text = describe(&status, None, Some(Cause::Http(429)))
            .ok_or_else(|| anyhow::anyhow!("report"))?;
        assert!(text.starts_with("Stopped: Model usage limit reached"));
        assert!(describe(&Status::Done("done".into()), None, None).is_none());
        Ok(())
    }
}
