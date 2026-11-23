//! Complete policy-bound directory observations and immutable viewer queries.
use super::{
    file::{self, FailureKind},
    publication::{Publication, Reference, Table},
};
use anyhow::Result;
use meden_agent::tool::{Args, ToolError};
use meden_core::file::{Operation, Request};
use meden_runtime::{
    process::{Control, Limits},
    workspace::Workspace,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// Only observed filesystem types; a symlink is not followed during listing.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Regular file entry.
    File,
    /// Directory entry.
    Directory,
    /// A separately resolved link, if explicitly opened later.
    Symlink,
    /// Another filesystem type, without an invented source capability.
    Other,
}
/// Original UTF-8 name and filesystem type, never a formatted line.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// One literal path component, including spaces/Unicode/newlines.
    #[schemars(length(min = 1, max = 255))]
    pub name: String,
    /// Host-observed entry type.
    pub kind: Kind,
}
/// Complete listing or typed host failure; no omitted-entry sentinel.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reading {
    /// A complete bounded directory observation.
    Ready {
        /// Canonically ordered original entries.
        #[schemars(length(max = 100_000))]
        entries: Vec<Entry>,
    },
    /// No partial listing was admitted.
    Unavailable {
        /// Typed failure, independent of message content.
        failure: FailureKind,
        /// Bounded local diagnostic.
        #[schemars(length(max = 8000))]
        message: String,
    },
}
/// Original requested directory and its complete outcome.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Directory {
    /// Logical host-selected path; `.` denotes the workspace root.
    #[schemars(length(min = 1, max = 4096))]
    pub path: String,
    /// Complete observed entries or explicit failure.
    pub reading: Reading,
}
/// Small status references, separate from directory contents.
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Index {
    /// Latest retained observation of each requested directory.
    pub paths: BTreeMap<String, Reference>,
}
/// Prepared outside the status lock, with no subsequent filesystem access.
pub struct Prepared {
    path: String,
    publication: Publication,
}
impl Prepared {
    /// Freeze a complete validated directory outcome.
    pub fn new(directory: Directory) -> Result<Self> {
        anyhow::ensure!(
            !directory.path.is_empty()
                && directory.path.len() <= 4096
                && !directory.path.contains('\0'),
            "Invalid directory path"
        );
        match &directory.reading {
            Reading::Ready { entries } => {
                anyhow::ensure!(entries.len() <= 100_000, "Directory exceeds 100000 entries");
                let mut previous: Option<&str> = None;
                for entry in entries {
                    anyhow::ensure!(
                        !entry.name.is_empty()
                            && entry.name.len() <= 255
                            && !entry.name.contains(['/', '\0'])
                            && !matches!(entry.name.as_str(), "." | "..")
                            && previous.is_none_or(|old| old < entry.name.as_str()),
                        "Invalid directory component/order"
                    );
                    previous = Some(&entry.name);
                }
            }
            Reading::Unavailable { message, .. } => anyhow::ensure!(
                message.chars().count() <= 8000,
                "Directory diagnostic exceeds its bound"
            ),
        }
        let publication = Publication::new(b"meden.directory.view.v1\0", &directory)?;
        Ok(Self {
            path: directory.path,
            publication,
        })
    }
}
/// Bounded directory retention shares framing/table mechanics with file readers.
#[derive(Default)]
pub struct Directories {
    table: Table,
}
impl Directories {
    /// Publish the latest complete observation of its original path.
    pub fn publish(&mut self, prepared: Prepared) {
        self.table.publish(prepared.path, prepared.publication);
    }
    /// Queries clone only complete retained bytes.
    pub fn bytes(&self, revision: &str) -> Option<Arc<[u8]>> {
        self.table.bytes(revision)
    }
    /// Explicit refresh retires all old references.
    pub fn clear(&mut self) {
        self.table.clear();
    }
    /// Small generated status projection.
    pub fn index(&self) -> Index {
        Index {
            paths: self.table.references(),
        }
    }
}
impl Serialize for Directories {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.index().serialize(serializer)
    }
}
/// The same original/resolved offline worker and immutable host file policy as
/// source observations. It returns a complete typed list instead of tool prose.
pub fn observe(workspace: &Workspace, control: &Control, path: &str) -> Result<Prepared> {
    let _timing = meden_runtime::metrics::Span::start("view.directory.observe");
    let result = (|| -> Result<Vec<Entry>, ToolError> {
        let logical = crate::workspace::logical_path(path)
            .ok_or_else(|| ToolError::refused("Directory leaves /workspace".into()))?;
        let request = Request::new(
            Operation::List,
            logical.to_string_lossy().as_ref().into(),
            Args::from_iter([
                ("entries".into(), true.into()),
                ("max_entries".into(), 100_000.into()),
            ]),
        );
        let value = meden_runtime::file::run(
            workspace,
            control,
            &request,
            &crate::workspace::file_policy().map_err(|e| ToolError::unavailable(e.to_string()))?,
            &Limits {
                timeout: Duration::from_secs(30),
                capture_bytes: 67_108_864,
                output_bytes: 67_108_864,
            },
        )?;
        serde_json::from_value(value).map_err(|e| ToolError::unavailable(e.to_string()))
    })();
    let reading = match result {
        Ok(entries) => Reading::Ready { entries },
        Err(error) => Reading::Unavailable {
            failure: file::failure(&error),
            message: error.to_string().chars().take(8000).collect(),
        },
    };
    Prepared::new(Directory {
        path: path.into(),
        reading,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context as _;
    #[test]
    fn complete_literal_entries_are_separate_from_status_and_failures_replace_old_data(
    ) -> Result<()> {
        let mut entries: Vec<_> = (0..1200)
            .map(|i| Entry {
                name: format!("File{i:04}.txt"),
                kind: Kind::File,
            })
            .collect();
        entries.push(Entry {
            name: "newline\n日本語.txt".into(),
            kind: Kind::File,
        });
        let mut directories = Directories::default();
        directories.publish(Prepared::new(Directory {
            path: ".".into(),
            reading: Reading::Ready { entries },
        })?);
        let revision = directories
            .index()
            .paths
            .get(".")
            .context("root")?
            .revision
            .clone();
        assert!(serde_json::to_vec(&directories)?.len() < 200);
        let bytes = directories.bytes(&revision).context("complete body")?;
        assert!(Arc::ptr_eq(
            &bytes,
            &directories.bytes(&revision).context("same data")?
        ));
        let data: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(
            data.pointer("/data/reading/entries")
                .and_then(|v| v.as_array())
                .map(Vec::len),
            Some(1201)
        );
        assert_eq!(
            data.pointer("/data/reading/entries/1200/name")
                .and_then(|v| v.as_str()),
            Some("newline\n日本語.txt")
        );
        directories.publish(Prepared::new(Directory {
            path: ".".into(),
            reading: Reading::Unavailable {
                failure: FailureKind::Operation,
                message: "removed".into(),
            },
        })?);
        assert!(directories.bytes(&revision).is_none());
        directories.clear();
        assert!(directories.index().paths.is_empty());
        Ok(())
    }
    #[test]
    fn invalid_components_duplicates_order_and_incomplete_listings_are_refused() {
        for names in [
            vec![".."],
            vec!["a/b"],
            vec!["a\0b"],
            vec![""],
            vec!["b", "a"],
            vec!["a", "a"],
        ] {
            assert!(Prepared::new(Directory {
                path: ".".into(),
                reading: Reading::Ready {
                    entries: names
                        .into_iter()
                        .map(|name| Entry {
                            name: name.into(),
                            kind: Kind::File
                        })
                        .collect()
                }
            })
            .is_err());
        }
        assert!(Prepared::new(Directory {
            path: ".".into(),
            reading: Reading::Ready {
                entries: (0..100_001)
                    .map(|i| Entry {
                        name: format!("F{i:06}"),
                        kind: Kind::File
                    })
                    .collect()
            }
        })
        .is_err());
    }
}
