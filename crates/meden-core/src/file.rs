//! One owned file request supplies both policy requirements and worker input.

use serde::{Deserialize, Serialize};

use crate::{allowed, Capability, Policy, Requirement, Scope};

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

/// A resolved observation and its owned resource, not an execution permit.
/// The resource has no accessor before checking both paths against one policy.
///
/// ```compile_fail
/// use meden_core::file::{Operation, Request};
/// let observed = Request::new(Operation::Read, "/link".into(), ())
///     .bind("/target".into(), "file handle");
/// let unchecked = observed.resource;
/// ```
///
/// ```compile_fail
/// use meden_core::{Policy, file::{Operation, Request}};
/// let observed = Request::new(Operation::Read, "/link".into(), ())
///     .bind("/target".into(), "file handle");
/// let first = observed.consume(&Policy::default());
/// let replay = observed.consume(&Policy::default());
/// ```
pub struct Resolved<T, R> {
    request: Request<T>,
    canonical: Scope,
    resource: R,
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

    /// Bind the observed canonical path to its still-owned physical resource.
    /// The adapter is responsible for attributing that path to that resource.
    pub fn bind<R>(self, canonical: Scope, resource: R) -> Resolved<T, R> {
        Resolved {
            request: self,
            canonical,
            resource,
        }
    }
}

impl<T, R> Resolved<T, R> {
    /// Consume the observation; return the original request and resource only
    /// if every content effect is allowed at both its original and canonical path.
    /// No caller-supplied allowance bit participates in this decision.
    pub fn consume(self, rules: &Policy) -> Option<(Request<T>, Scope, R)> {
        let (request, declared) = self.request.prepare();
        let (_, resolved) = Request::new(request.operation, self.canonical.clone(), ()).prepare();
        if allowed(&declared, rules) && allowed(&resolved, rules) {
            Some((request, self.canonical, self.resource))
        } else {
            None
        }
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
            let mut invocation = Attempt::with_policy(Ok(prepared), &policy, "file effect denied");
            assert_eq!(matches!(invocation.announce(), Notice::Allowed), permitted);
            let result = invocation.dispatch(false, None::<()>);
            assert_eq!(result.is_ok(), permitted);
            if let Ok(call) = result {
                let (request, rules) = call.into_parts();
                assert!(std::ptr::eq(rules, std::ptr::from_ref(&policy)));
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

    #[test]
    fn resolved_resources_need_both_original_and_canonical_authority(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let both = [
            "fs.read[/logical]",
            "fs.write[/logical]",
            "fs.read[/canonical]",
            "fs.write[/canonical]",
        ];
        for (labels, denies, expected) in [
            (vec![], vec![], [false; 5]),
            (both[..2].to_vec(), vec![], [false; 5]),
            (both[2..].to_vec(), vec![], [false; 5]),
            (both.to_vec(), vec![], [true; 5]),
            (
                vec![both[0], both[2]],
                vec![],
                [true, true, false, false, true],
            ),
            (
                vec![both[1], both[3]],
                vec![],
                [false, false, true, false, false],
            ),
            (
                vec![both[0], both[1], both[2]],
                vec![],
                [true, true, false, false, true],
            ),
            (
                vec![both[0], both[1], both[3]],
                vec![],
                [false, false, true, false, false],
            ),
            (
                both.to_vec(),
                vec!["fs.read"],
                [false, false, true, false, false],
            ),
            (
                both.to_vec(),
                vec!["fs.write"],
                [true, true, false, false, true],
            ),
        ] {
            let rules = Policy::from_labels("resolved", &labels, &denies)?;
            for (operation, permitted) in [
                Operation::Read,
                Operation::List,
                Operation::Write,
                Operation::Edit,
                Operation::Snapshot,
            ]
            .into_iter()
            .zip(expected)
            {
                let result = Request::new(operation, "/logical".into(), ("original", 17))
                    .bind("/canonical".into(), ("original resource", 29))
                    .consume(&rules);
                assert_eq!(
                    result.is_some(),
                    permitted,
                    "{operation:?}: {labels:?} / {denies:?}"
                );
                if let Some((request, canonical, resource)) = result {
                    assert_eq!(
                        request.into_parts(),
                        (operation, "/logical".into(), ("original", 17))
                    );
                    assert_eq!(canonical, Scope::from("/canonical"));
                    assert_eq!(resource, ("original resource", 29));
                }
            }
        }
        Ok(())
    }
}
