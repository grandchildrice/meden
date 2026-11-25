//! Host observation/progress adapters for the application's syntax cache.
use super::{source_files, File, View};
use crate::gui::lock;
use anyhow::Result;
use meden_app::navigation::cache::{self, Scope};
use meden_runtime::{process::Control, workspace::Workspace};
use serde_json::Value;
use std::{sync::Mutex, time::Instant};
type Store = cache::Store<crate::adapters::source::Symbol, Value>;
fn store(workspace: &Workspace) -> Result<Store> {
    Store::host(
        workspace.path(),
        &crate::adapters::source::identity(),
        crate::adapters::navigation::additional_identity,
    )
}
pub(super) fn generation(workspace: &Workspace) -> Result<u64> {
    store(workspace)?.generation()
}
pub(super) fn load(workspace: &Workspace, path: &str) -> Result<Option<File>> {
    store(workspace)?.load(path)
}
pub(super) fn prepare(
    view: &Mutex<View>,
    workspace: &Workspace,
    control: &Control,
    paths: &[String],
    started: Instant,
    fresh: bool,
) -> Result<()> {
    let preparing = lock(view).overview.state == super::State::Loading;
    if preparing {
        lock(view).activity = "Preparing cached project map".into();
    }
    store(workspace)?.prepare(
        &Reader {
            view,
            workspace,
            control,
            started,
        },
        paths,
        fresh,
        if preparing {
            Scope::Inventory
        } else {
            Scope::Selection
        },
        |count| {
            if preparing {
                lock(view).overview.scanned = count;
            }
        },
    )
}
struct Reader<'a> {
    view: &'a Mutex<View>,
    workspace: &'a Workspace,
    control: &'a Control,
    started: Instant,
}
impl cache::Reader for Reader<'_> {
    type Symbol = crate::adapters::source::Symbol;
    type Stamp = Value;
    fn stamps(&self, paths: &[String]) -> Result<Vec<Value>> {
        crate::adapters::navigation_io::Reader::new(self.workspace, self.control, self.started)
            .stamps(paths)
    }
    fn sources(&self, paths: &[String]) -> Result<Vec<File>> {
        source_files(self.view, self.workspace, self.control, paths, self.started)
    }
    fn active(&self) -> Result<()> {
        Ok(self.control.with_active(|| Ok(()))?)
    }
}
