//! Native ownership of heterogeneous typed observations. Backend ASTs never
//! become a universal JSON execution interface; dispatch retains their Rust type.
use super::{repair, sources, Record};
use crate::{adapters::Context, model::Model, semantic, view};
use anyhow::{Context as _, Result};
use meden_core::Policy;
use meden_runtime::{process::Control, workspace::Workspace};
use semantic::Observation as _;
use serde::Serialize;
use sha2::Digest as _;
use sources::Source as _;
use std::sync::Arc;
use view::Projection as _;

/// One host-selected preparation implementation and its original decoded output.
pub trait Backend: Send + Sync + 'static {
    type Output: repair::Observation + view::Projection + Send + Sync + 'static;
    fn read(&self, context: &Context<'_>, paths: &[String]) -> Result<(Self::Output, bool)>;
    /// Backend-owned scope attribution, independent of the terminal renderer.
    fn details(&self, _: &Self::Output) -> Vec<String> {
        Vec::new()
    }
    fn saved(&self, _: &Context<'_>, _: &[String]) -> Result<Option<view::sources::SavedSource>> {
        Ok(None)
    }
    fn background(&self) -> bool {
        true
    }
}

/// A host-composed typed loader. Its output type is retained behind dispatch,
/// rather than decoded through a second backend/format switch in the viewer.
pub struct Preparation(Arc<dyn Loader>);
impl Preparation {
    pub fn new<B: Backend>(backend: B) -> Self {
        Self(Arc::new(Load(Arc::new(backend))))
    }
    pub fn read(&self, context: &Context<'_>, paths: &[String]) -> Result<(Snapshot, bool)> {
        self.0.read(context, paths)
    }
    pub fn saved(
        &self,
        context: &Context<'_>,
        paths: &[String],
    ) -> Result<Option<view::sources::SavedSource>> {
        self.0.saved(context, paths)
    }
    pub fn background(&self) -> bool {
        self.0.background()
    }
}
trait Loader: Send + Sync {
    fn read(&self, context: &Context<'_>, paths: &[String]) -> Result<(Snapshot, bool)>;
    fn saved(
        &self,
        context: &Context<'_>,
        paths: &[String],
    ) -> Result<Option<view::sources::SavedSource>>;
    fn background(&self) -> bool;
}
struct Load<B>(Arc<B>);
impl<B: Backend> Loader for Load<B> {
    fn read(&self, context: &Context<'_>, paths: &[String]) -> Result<(Snapshot, bool)> {
        crate::compiler::Request::new(paths.to_vec())?;
        let selected = paths
            .iter()
            .map(|path| {
                let logical = crate::workspace::logical_path(path)
                    .context("Source selection leaves the workspace")?;
                Ok(logical
                    .strip_prefix("/workspace")?
                    .to_str()
                    .context("Source path is not UTF-8")?
                    .to_owned())
            })
            .collect::<Result<Vec<_>>>()?;
        let (output, reused) = self.0.read(context, paths)?;
        anyhow::ensure!(
            selected
                .iter()
                .all(|p| output.sources().files.contains_key(p)),
            "Prepared backend omitted an originally selected source"
        );
        Ok((Snapshot::new(Arc::clone(&self.0), output)?, reused))
    }
    fn saved(
        &self,
        context: &Context<'_>,
        paths: &[String],
    ) -> Result<Option<view::sources::SavedSource>> {
        self.0.saved(context, paths)
    }
    fn background(&self) -> bool {
        self.0.background()
    }
}

/// Borrowed original host invocation. The existing services still check model
/// approval, policy, freshness, cancellation and immutable request bindings.
pub struct Run<'a> {
    pub workspace: &'a Workspace,
    pub gateway: &'a dyn Model,
    pub judge: Option<&'a dyn Model>,
    pub control: &'a Control,
    pub notify: &'a dyn Fn(&Record),
}

/// Cloneable native owner without a prover-specific enum, downcast or mutable
/// backend getter. Serialization exposes the original observation, not authority.
/// A caller cannot pair a later loader with an independently supplied result:
/// ```compile_fail,E0624
/// use meden_app::analysis::prepared::{Backend,Snapshot};
/// fn wrong<B:Backend>(b:std::sync::Arc<B>,o:B::Output) { let _=Snapshot::new(b,o); }
/// ```
/// Stored JSON cannot select or reconstruct a runnable backend:
/// ```compile_fail,E0277
/// fn wrong(v:serde_json::Value) { let _=serde_json::from_value::<meden_app::analysis::prepared::Snapshot>(v); }
/// ```
#[derive(Clone)]
pub struct Snapshot(Arc<dyn Original>);
impl Snapshot {
    fn new<B: Backend>(backend: Arc<B>, output: B::Output) -> Result<Self> {
        let _timing = meden_runtime::metrics::Span::start("analysis.bind_backend");
        output.sources().validate()?;
        let body = fingerprint(&serde_json::to_value(&output)?)?;
        Ok(Self(Arc::new(Bound {
            backend,
            output,
            body,
        })))
    }
    pub fn sources(&self) -> &sources::Snapshot {
        self.0.sources()
    }
    pub fn graph_revision(&self) -> Option<&crate::identity::snapshots::GraphRevision> {
        self.0.graph_revision()
    }
    pub fn details(&self) -> Result<Vec<String>> {
        self.0.details()
    }
    pub fn analyze(&self, run: &Run<'_>, input: semantic::Input) -> Result<Record> {
        self.0.analyze(run, input)
    }
    pub fn prepare_repair(
        &self,
        run: &Run<'_>,
        policy: &Policy,
        record: &Record,
        project: &crate::project::Summary,
    ) -> Result<Pending> {
        self.0.prepare_repair(run, policy, record, project)
    }
}
impl sources::Source for Snapshot {
    fn sources(&self) -> &sources::Snapshot {
        self.sources()
    }
}
impl view::Projection for Snapshot {
    fn page(&self) -> Result<view::SourcePage> {
        self.0.page()
    }
}
impl Serialize for Snapshot {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0
            .body()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}
trait Original: Send + Sync {
    fn sources(&self) -> &sources::Snapshot;
    fn graph_revision(&self) -> Option<&crate::identity::snapshots::GraphRevision>;
    fn body(&self) -> Result<serde_json::Value>;
    fn page(&self) -> Result<view::SourcePage>;
    fn details(&self) -> Result<Vec<String>>;
    fn analyze(&self, run: &Run<'_>, input: semantic::Input) -> Result<Record>;
    fn prepare_repair(
        &self,
        run: &Run<'_>,
        policy: &Policy,
        record: &Record,
        project: &crate::project::Summary,
    ) -> Result<Pending>;
}
struct Bound<B: Backend> {
    backend: Arc<B>,
    output: B::Output,
    body: [u8; 32],
}
fn fingerprint(body: &serde_json::Value) -> Result<[u8; 32]> {
    let mut hash = sha2::Sha256::new();
    hash.update(b"meden-native-source-v1\0");
    hash.update(serde_json::to_vec(body)?);
    Ok(hash.finalize().into())
}
impl<B: Backend> Original for Bound<B> {
    fn sources(&self) -> &sources::Snapshot {
        self.output.sources()
    }
    fn graph_revision(&self) -> Option<&crate::identity::snapshots::GraphRevision> {
        self.output.graph_revision()
    }
    fn body(&self) -> Result<serde_json::Value> {
        let body = serde_json::to_value(&self.output)?;
        anyhow::ensure!(
            fingerprint(&body)? == self.body,
            "Originally prepared backend observation changed"
        );
        Ok(body)
    }
    fn page(&self) -> Result<view::SourcePage> {
        self.body()?;
        let page = self.output.page()?;
        let original = self.output.graph_revision();
        anyhow::ensure!(
            page.compiled == original.is_some()
                && (!page.compiled || page.graph.as_ref().map(|g| &g.revision) == original),
            "Backend projection changed its original semantic graph binding"
        );
        Ok(page)
    }
    fn details(&self) -> Result<Vec<String>> {
        self.body()?;
        Ok(self.backend.details(&self.output))
    }
    fn analyze(&self, run: &Run<'_>, input: semantic::Input) -> Result<Record> {
        self.body()?;
        super::run(
            run.workspace,
            run.gateway,
            run.judge,
            run.control,
            &self.output,
            input,
            run.notify,
        )
    }
    fn prepare_repair(
        &self,
        run: &Run<'_>,
        policy: &Policy,
        record: &Record,
        project: &crate::project::Summary,
    ) -> Result<Pending> {
        self.body()?;
        let pending = repair::prepare(
            run.workspace,
            run.gateway,
            run.control,
            policy,
            &self.output,
            record,
            project,
        )?;
        Ok(Pending(Box::new(TypedPending {
            backend: Arc::clone(&self.backend),
            pending,
        })))
    }
}

/// Original typed repair context and preparation implementation remain paired.
/// There is no decoder or API for replacing either member of this pair.
pub struct Pending(Box<dyn PendingRepair>);
trait PendingRepair: Send {
    fn install(
        self: Box<Self>,
        workspace: &Workspace,
    ) -> Result<(Box<dyn RepairQueue>, repair::Review)>;
}
struct TypedPending<B: Backend> {
    backend: Arc<B>,
    pending: repair::Pending<<B::Output as repair::Observation>::Repair>,
}
impl<B: Backend> PendingRepair for TypedPending<B> {
    fn install(
        self: Box<Self>,
        workspace: &Workspace,
    ) -> Result<(Box<dyn RepairQueue>, repair::Review)> {
        let mut queue = repair::Queue::default();
        let review = queue.install(workspace, self.pending)?;
        Ok((
            Box::new(TypedQueue {
                backend: self.backend,
                queue,
            }),
            review,
        ))
    }
}
trait RepairQueue: Send {
    fn abandon(&mut self, workspace: &Workspace) -> Result<()>;
    fn reject(&mut self, workspace: &Workspace, reference: &repair::Reference) -> Result<()>;
    fn apply(
        &mut self,
        run: &Run<'_>,
        reference: &repair::Reference,
        accepted: Option<&crate::project::Reference>,
        dirty: &dyn Fn(),
    ) -> Result<(Snapshot, Record)>;
}
struct TypedQueue<B: Backend> {
    backend: Arc<B>,
    queue: repair::Queue<<B::Output as repair::Observation>::Repair>,
}
impl<B: Backend> RepairQueue for TypedQueue<B> {
    fn abandon(&mut self, workspace: &Workspace) -> Result<()> {
        self.queue.abandon(workspace)
    }
    fn reject(&mut self, workspace: &Workspace, reference: &repair::Reference) -> Result<()> {
        self.queue.reject(workspace, reference)
    }
    fn apply(
        &mut self,
        run: &Run<'_>,
        reference: &repair::Reference,
        accepted: Option<&crate::project::Reference>,
        dirty: &dyn Fn(),
    ) -> Result<(Snapshot, Record)> {
        let applied = self
            .queue
            .apply(run.workspace, run.control, reference, accepted, dirty)?;
        let (output, record) = applied.reanalyze(
            run.workspace,
            run.gateway,
            run.judge,
            run.control,
            |_, paths, _| {
                self.backend
                    .read(
                        &Context {
                            workspace: run.workspace,
                            control: run.control,
                            observer: None,
                        },
                        paths,
                    )
                    .map(|(output, _)| output)
            },
            run.notify,
        )?;
        Ok((Snapshot::new(Arc::clone(&self.backend), output)?, record))
    }
}

/// Native repair retention delegates every lifecycle/effect to the existing
/// typed queue and consuming kernel; it adds no approval or acceptance state.
#[derive(Default)]
pub struct Repairs(Option<Box<dyn RepairQueue>>);
impl Repairs {
    pub fn install(&mut self, workspace: &Workspace, pending: Pending) -> Result<repair::Review> {
        self.abandon(workspace)?;
        let (queue, review) = pending.0.install(workspace)?;
        self.0 = Some(queue);
        Ok(review)
    }
    pub fn abandon(&mut self, workspace: &Workspace) -> Result<()> {
        if let Some(mut queue) = self.0.take() {
            queue.abandon(workspace)?;
        }
        Ok(())
    }
    pub fn reject(&mut self, workspace: &Workspace, reference: &repair::Reference) -> Result<()> {
        self.0
            .take()
            .context("No repair awaits review")?
            .reject(workspace, reference)
    }
    pub fn apply(
        &mut self,
        run: &Run<'_>,
        reference: &repair::Reference,
        accepted: Option<&crate::project::Reference>,
        dirty: &dyn Fn(),
    ) -> Result<(Snapshot, Record)> {
        self.0
            .take()
            .context("Repair already consumed, rejected or unavailable after restart")?
            .apply(run, reference, accepted, dirty)
    }
}
