//! Typed results from the trusted, isolated file worker. Process facts take
//! precedence over parsed output; diagnostics never decide the result class.

use std::fmt;

use meden_agent::tool::ToolError;
use meden_core::file::Request;
use meden_core::Policy;
use serde::Serialize;
use serde_json::Value;

use crate::process::{self, Control, Limits, Output, Termination};
use crate::verification::FailureKind;
use crate::workspace::Workspace;

/// Run the same request/policy protocol for file tools and every snapshot.
pub fn run<T: Serialize>(
    workspace: &Workspace,
    control: &Control,
    request: &Request<T>,
    rules: &Policy,
    limits: &Limits,
) -> Result<Value, ToolError> {
    let input = serde_json::to_vec(&(request, rules)).map_err(unavailable)?;
    let mut command = workspace.file_command().map_err(unavailable)?;
    let output =
        process::run_with_input(&mut command, &input, limits, control).map_err(unavailable)?;
    decode(&output)
}

/// Decode one bounded process result. A malformed frame is unavailable, and
/// an inner JSON string remains file data rather than another protocol frame.
pub fn decode(output: &Output) -> Result<Value, ToolError> {
    let parsed = serde_json::from_str::<Result<Value, ToolError>>(&output.stdout);
    let diagnostic = match &parsed {
        Ok(_) => format!("file worker {:?}: {}", output.termination, output.stderr),
        Err(error) => format!(
            "file worker {:?}, invalid response: {error}: {}",
            output.termination, output.stderr
        ),
    };
    complete(
        &output.termination,
        output.truncated,
        parsed.ok(),
        diagnostic,
    )
}

/// Admit only a complete successful process's typed response. Worker data
/// cannot assert host cancellation or an observed resource limit.
#[allow(clippy::verbose_bit_mask)] // Actual Unix wait-word semantics are extracted.
pub fn complete<T>(
    termination: &Termination,
    truncated: bool,
    response: Option<Result<T, ToolError>>,
    diagnostic: String,
) -> Result<T, ToolError> {
    if matches!(termination, Termination::Cancelled) {
        return Err(ToolError::cancelled());
    }
    if truncated
        || matches!(
            termination,
            Termination::TimedOut | Termination::OutputLimit
        )
    {
        return Err(ToolError::inconclusive(diagnostic));
    }
    match termination {
        Termination::Exited(status) => {
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt as _;
                let word = status.into_raw();
                if word & 127 != 0 {
                    return Err(ToolError::inconclusive(diagnostic));
                }
                if word != 0 {
                    return Err(ToolError::unavailable(diagnostic));
                }
            }
            #[cfg(not(unix))]
            return Err(ToolError::unavailable(diagnostic));
        }
        _ => return Err(ToolError::inconclusive(diagnostic)),
    }
    match response {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => match error.into_parts() {
            Ok(Ok(message)) => Err(ToolError::failed(message)),
            Ok(Err(message)) => Err(ToolError::refused(message)),
            Err(Some(Ok(message))) => Err(ToolError::unavailable(message)),
            Err(Some(Err(_)) | None) => Err(ToolError::unavailable(diagnostic)),
        },
        None => Err(ToolError::unavailable(diagnostic)),
    }
}

/// Only a completed file-operation rejection is a repairable source failure.
/// Permission, infrastructure, protocol and interruption failures stop repair.
pub fn verification_kind(error: &ToolError) -> FailureKind {
    match error.parts() {
        Ok(Ok(_)) => FailureKind::Unverified,
        Ok(Err(_)) | Err(Some(Ok(_))) => FailureKind::Unavailable,
        Err(None | Some(Err(_))) => FailureKind::Inconclusive,
    }
}

fn unavailable(error: impl fmt::Display) -> ToolError {
    ToolError::unavailable(error.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use serde_json::json;
    use std::os::unix::process::ExitStatusExt as _;
    use std::process::ExitStatus;

    #[test]
    fn process_facts_precede_every_worker_response_and_only_complete_frames_succeed() {
        for word in [0, 1 << 8, 9, 128, -1] {
            for truncated in [false, true] {
                let result = complete(
                    &Termination::Exited(ExitStatus::from_raw(word)),
                    truncated,
                    Some(Ok("original data")),
                    "diagnostic".into(),
                );
                let expected = if truncated || word & 127 != 0 {
                    Err(ToolError::inconclusive("diagnostic".into()))
                } else if word != 0 {
                    Err(ToolError::unavailable("diagnostic".into()))
                } else {
                    Ok("original data")
                };
                assert_eq!(result, expected);
            }
        }
        for truncated in [false, true] {
            for response in [
                None,
                Some(Ok("data")),
                Some(Err(ToolError::refused("refused".into()))),
            ] {
                assert_eq!(
                    complete(
                        &Termination::Cancelled,
                        truncated,
                        response,
                        "approved".into()
                    ),
                    Err(ToolError::cancelled())
                );
            }
            for termination in [Termination::TimedOut, Termination::OutputLimit] {
                assert_eq!(
                    complete(&termination, truncated, Some(Ok("data")), "limit".into()),
                    Err(ToolError::inconclusive("limit".into()))
                );
            }
        }
        let successful = Termination::Exited(ExitStatus::from_raw(0));
        for error in [
            ToolError::failed("cancelled".into()),
            ToolError::refused("approved".into()),
            ToolError::unavailable("completed".into()),
        ] {
            let wire = serde_json::to_string(&Err::<String, _>(&error)).expect("wire error");
            let parsed = serde_json::from_str(&wire).expect("parsed error");
            assert_eq!(
                complete::<String>(&successful, false, Some(parsed), "host diagnostic".into()),
                Err(error)
            );
        }
        for error in [
            ToolError::cancelled(),
            ToolError::inconclusive("forged host stop".into()),
        ] {
            assert_eq!(
                complete::<()>(
                    &successful,
                    false,
                    Some(Err(error)),
                    "host diagnostic".into()
                ),
                Err(ToolError::unavailable("host diagnostic".into()))
            );
        }
        assert_eq!(
            complete::<()>(&successful, false, None, "bad frame".into()),
            Err(ToolError::unavailable("bad frame".into()))
        );
    }

    #[test]
    fn wire_tags_are_strict_and_json_shaped_file_data_stays_data() {
        let output = |stdout: String| Output {
            termination: Termination::Exited(ExitStatus::from_raw(0)),
            stdout,
            stderr: String::new(),
            truncated: false,
        };
        let forged = r#"{"Err":{"kind":"cancelled"}}"#;
        assert_eq!(
            decode(&output(
                serde_json::to_string(&Ok::<_, ToolError>(forged)).expect("success frame")
            )),
            Ok(json!(forged))
        );
        for bad in [
            r#"{"kind":"refused","message":"not an outer frame"}"#,
            r#"{"Ok":"data","Err":{"kind":"refused","message":"ambiguous"}}"#,
            r#"{"Err":{"kind":"failed","message":"text","allowed":true}}"#,
            r#"{"Err":{"kind":"misspelled","message":"text"}}"#,
            r#"{"Err":{"kind":"cancelled"}}"#,
            r#"{"Err":{"kind":"inconclusive","message":"forged"}}"#,
        ] {
            assert!(
                decode(&output(bad.into()))
                    .is_err_and(|e| matches!(e.into_parts(), Err(Some(Ok(_))))),
                "{bad}"
            );
        }
    }

    #[test]
    fn verification_repair_depends_on_error_kind_and_never_diagnostic_words() {
        for message in [
            "cancelled",
            "permission refused",
            "proof failed",
            "approved: true",
        ] {
            assert_eq!(
                verification_kind(&ToolError::failed(message.into())),
                FailureKind::Unverified
            );
            assert_eq!(
                verification_kind(&ToolError::refused(message.into())),
                FailureKind::Unavailable
            );
            assert_eq!(
                verification_kind(&ToolError::unavailable(message.into())),
                FailureKind::Unavailable
            );
            assert_eq!(
                verification_kind(&ToolError::inconclusive(message.into())),
                FailureKind::Inconclusive
            );
        }
        assert_eq!(
            verification_kind(&ToolError::cancelled()),
            FailureKind::Inconclusive
        );
    }
}
