//! One owned file request supplies both policy requirements and worker input.

use serde::{Deserialize, Serialize};

use crate::{Capability, Requirement, Scope};

/// The worker's operation, independent of its model-facing description.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    /// Read UTF-8 lines.
    #[serde(rename = "read_file")]
    Read,
    /// List directory entries.
    #[serde(rename = "list_files")]
    List,
    /// Replace a file.
    #[serde(rename = "write_file")]
    Write,
    /// Read and replace one matching text occurrence.
    #[serde(rename = "edit_file")]
    Edit,
    /// Read a bounded immutable byte snapshot.
    #[serde(rename = "snapshot")]
    Snapshot,
}

/// Transport data, not an execution permit. Only the invocation controller
/// can authorize the prepared request. Path decoding and filesystem resolution
/// remain adapter obligations.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request<T> {
    operation: Operation,
    path: Scope,
    payload: T,
}

impl<T> Request<T> {
    /// Own the exact operation, decoded path and operation-specific data.
    pub fn new(operation: Operation, path: Scope, payload: T) -> Self {
        Self {
            operation,
            path,
            payload,
        }
    }

    /// Derive every declared file effect from the same request that executes.
    pub fn prepare(self) -> (Self, Vec<Requirement>) {
        let read = matches!(
            self.operation,
            Operation::Read | Operation::List | Operation::Edit | Operation::Snapshot
        );
        let write = matches!(self.operation, Operation::Write | Operation::Edit);
        let mut requirements = Vec::new();
        if read {
            requirements.push(Requirement {
                capability: Capability::FsRead,
                scope: Some(self.path.clone()),
                evidence: None,
            });
        }
        if write {
            requirements.push(Requirement {
                capability: Capability::FsWrite,
                scope: Some(self.path.clone()),
                evidence: None,
            });
        }
        (self, requirements)
    }

    /// Consume precisely the prepared request at the worker boundary.
    pub fn into_parts(self) -> (Operation, Scope, T) {
        (self.operation, self.path, self.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invocation::{Attempt, Notice};
    use crate::Policy;

    #[test]
    fn editing_cannot_read_with_only_write_authority() -> Result<(), Box<dyn std::error::Error>> {
        for (labels, permitted) in [
            (vec!["fs.write[/workspace/*]"], false),
            (vec!["fs.read[/workspace/*]"], false),
            (
                vec!["fs.read[/workspace/*]", "fs.write[/workspace/*]"],
                true,
            ),
        ] {
            let policy = Policy::from_labels("edit", &labels, &[])?;
            let prepared = Request::new(
                Operation::Edit,
                "/workspace/private.rs".into(),
                ("old", "new"),
            )
            .prepare();
            let mut invocation = Attempt::new(Ok(prepared), &policy, "file effect denied");
            assert_eq!(matches!(invocation.announce(), Notice::Allowed), permitted);
            let result = invocation.dispatch(false, None::<()>);
            assert_eq!(result.is_ok(), permitted);
            if let Ok(request) = result {
                assert_eq!(
                    request.into_parts(),
                    (
                        Operation::Edit,
                        "/workspace/private.rs".into(),
                        ("old", "new")
                    )
                );
            }
            assert!(invocation.dispatch(false, None::<()>).is_err());
        }
        Ok(())
    }

    #[test]
    fn exact_operation_path_and_payload_determine_all_file_effects() -> serde_json::Result<()> {
        for (operation, capabilities) in [
            (Operation::Read, vec![Capability::FsRead]),
            (Operation::List, vec![Capability::FsRead]),
            (Operation::Write, vec![Capability::FsWrite]),
            (
                Operation::Edit,
                vec![Capability::FsRead, Capability::FsWrite],
            ),
            (Operation::Snapshot, vec![Capability::FsRead]),
        ] {
            let path = Scope::from("/workspace/日🦀.rs");
            let payload = ("original text", "replacement text");
            let (request, requirements) = Request::new(operation, path.clone(), payload).prepare();
            assert_eq!(
                requirements
                    .iter()
                    .map(|r| r.capability)
                    .collect::<Vec<_>>(),
                capabilities
            );
            assert!(requirements.iter().all(|r| r.scope.as_ref() == Some(&path)));
            let wire = serde_json::to_vec(&request)?;
            let decoded: Request<(String, String)> = serde_json::from_slice(&wire)?;
            assert_eq!(request.into_parts(), (operation, path.clone(), payload));
            assert_eq!(
                decoded.into_parts(),
                (operation, path, (payload.0.into(), payload.1.into()))
            );
        }
        Ok(())
    }
}
