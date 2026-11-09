//! Pin canonical workspace targets before content I/O. Kernel path resolution,
//! procfs descriptor reopening and the private mount namespace are trusted.

use super::{failed, Operation, ToolError};
use rustix::fs::{open, openat2, Mode, OFlags, ResolveFlags};
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

pub(super) enum Access {
    File(File),
    Directory(File),
    Replacement {
        parent: File,
        name: OsString,
        original: Option<File>,
    },
}

impl Access {
    /// `canonical` is an observation, not authority: resolve it again beneath
    /// the pinned root without following any newly introduced symlink. Existing
    /// internal links have already been resolved by FileOperation::resolve.
    pub(super) fn open(
        root: &Path,
        canonical: &Path,
        operation: Operation,
    ) -> Result<Self, ToolError> {
        let relative = canonical
            .strip_prefix(root)
            .map_err(|_| ToolError::Failed("path leaves the selected workspace".into()))?;
        let root = File::from(
            open(
                root,
                OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::empty(),
            )
            .map_err(failed)?,
        );
        match operation {
            Operation::List => Ok(Self::Directory(pin(&root, relative, true).map_err(failed)?)),
            Operation::Read | Operation::Snapshot => {
                let file = pin(&root, relative, false).map_err(failed)?;
                regular(&file)?;
                Ok(Self::File(
                    File::open(descriptor_path(&file)).map_err(failed)?,
                ))
            }
            Operation::Write | Operation::Edit => {
                let parent = relative
                    .parent()
                    .ok_or_else(|| ToolError::Failed("path has no parent".into()))?;
                let name = relative
                    .file_name()
                    .ok_or_else(|| ToolError::Failed("path has no filename".into()))?;
                let parent = pin(&root, parent, true).map_err(failed)?;
                let original = match pin(&parent, Path::new(name), false) {
                    Ok(file) => {
                        regular(&file)?;
                        Some(if operation == Operation::Edit {
                            File::open(descriptor_path(&file)).map_err(failed)?
                        } else {
                            file
                        })
                    }
                    // Keep errno until this decision; display text is not authority.
                    Err(rustix::io::Errno::NOENT) if operation == Operation::Write => None,
                    Err(error) => return Err(failed(error)),
                };
                Ok(Self::Replacement {
                    parent,
                    name: name.to_owned(),
                    original,
                })
            }
        }
    }
}

fn pin(directory: &File, relative: &Path, is_directory: bool) -> rustix::io::Result<File> {
    let relative = if relative.as_os_str().is_empty() {
        Path::new(".")
    } else {
        relative
    };
    openat2(
        directory,
        relative,
        OFlags::PATH
            | OFlags::CLOEXEC
            | OFlags::NOFOLLOW
            | if is_directory {
                OFlags::DIRECTORY
            } else {
                OFlags::empty()
            },
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
    )
    .map(File::from)
}

fn regular(file: &File) -> Result<(), ToolError> {
    if !file.metadata().map_err(failed)?.is_file() {
        return Err(ToolError::Failed(
            "file content access requires a regular file".into(),
        ));
    }
    Ok(())
}

/// Only a live, host-owned descriptor supplies this procfs path. Never append
/// an untrusted directory path or close the descriptor while using the path.
pub(super) fn descriptor_path(file: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
}

pub(super) fn atomic_write(
    parent: &File,
    name: &OsStr,
    original: Option<&File>,
    content: &str,
) -> Result<(), ToolError> {
    let directory = descriptor_path(parent);
    let mut temporary = tempfile::NamedTempFile::new_in(&directory).map_err(failed)?;
    temporary.write_all(content.as_bytes()).map_err(failed)?;
    if let Some(original) = original {
        temporary
            .as_file()
            .set_permissions(original.metadata().map_err(failed)?.permissions())
            .map_err(failed)?;
    }
    temporary.persist(directory.join(name)).map_err(failed)?;
    Ok(())
}
