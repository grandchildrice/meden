//! Loopback HTTP adapter. Native sessions, policies and checkpoints remain authoritative.
use meden_runtime::workspace::SessionId;
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Component, Path};
use std::sync::{atomic::AtomicBool, mpsc, Arc, Mutex};

use anyhow::{Context, Result};
use meden_agent::conversation::{Delta, Event, Status};
use meden_app::command::{Command, Operation};
use meden_runtime::{process::Control, workspace::Workspace};
use serde::Serialize;
use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::{session::history, session::Session, WorkspaceArgs};
mod analysis;
mod connections;
mod delivery;
mod overview;
mod preview;
mod repair;
mod sources;

struct Task {
    command: Command,
    processes: Arc<Control>,
    timing: meden_runtime::metrics::Context,
    queued: std::time::Instant,
}

#[derive(Clone, Copy)]
enum PublicationRole {
    Preview,
    History,
    Source,
    Reader,
    Inventory,
    Directory,
    PreparationLog,
    Analysis,
    AnalysisHistory,
    Evidence,
    VerificationJob,
}

impl Task {
    fn new(command: Command, processes: Arc<Control>) -> Self {
        let span = meden_runtime::metrics::Span::start("gui.enqueue");
        Self {
            command,
            processes,
            timing: span.context(),
            queued: std::time::Instant::now(),
        }
    }
}

struct View {
    history: meden_app::view::history::History,
    workspace: String,
    endpoint: String,
    model: String,
    provider: Option<String>,
    connections: Vec<meden_app::view::session::Choice>,
    connection_revision: Option<meden_app::view::sequence::Sequence>,
    models: Vec<String>,
    models_error: Option<String>,
    models_provider: Option<String>,
    overview: overview::Status,
    overview_snapshot: Option<Arc<overview::Snapshot>>,
    judge: Option<crate::judge::Binding>,
    authorized: bool,
    approval_available: bool,
    running: bool,
    operation: Operation,
    chat_stopped: Option<String>,
    activity: String,
    partial: String,
    reasoning: String,
    project: meden_app::view::evidence::Reader,
    verification_job: meden_app::view::verification_jobs::Reader,
    work_job: meden_app::view::verification_jobs::Reader,
    stopped: Option<String>,
    directories: meden_app::view::directory::Directories,
    readers: meden_app::view::readers::Readers,
    sources: meden_app::view::sources::Sources,
    source_preparation: meden_app::view::jobs::Jobs,
    analyses: meden_app::view::analyses::Analyses,
    repair_review: Option<meden_app::view::repairs::Review>,
    preview: meden_app::view::preview::Reader,
    exports: Vec<meden_app::view::delivery::Summary>,
    export_review: Option<meden_app::view::delivery::Review>,
    export_outcome: Option<meden_app::view::delivery::Outcome>,
    atlas: crate::atlas::Status,
}

impl Serialize for View {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        meden_app::view::session::State {
            history: &self.history,
            workspace: &self.workspace,
            endpoint: &self.endpoint,
            model: &self.model,
            provider: self.provider.as_deref(),
            connections: &self.connections,
            connection_revision: self.connection_revision,
            models: &self.models,
            models_error: self.models_error.as_deref(),
            models_provider: self.models_provider.as_deref(),
            overview: &self.overview,
            judge: self.judge.as_ref(),
            authorized: self.authorized,
            approval_available: self.approval_available,
            running: self.running,
            operation: self.operation,
            chat_stopped: self.chat_stopped.as_deref(),
            activity: &self.activity,
            partial: &self.partial,
            reasoning: &self.reasoning,
            project: &self.project,
            verification_job: &self.verification_job,
            work_job: &self.work_job,
            stopped: self.stopped.as_deref(),
            directories: &self.directories,
            readers: &self.readers,
            sources: &self.sources,
            source_preparation: &self.source_preparation,
            analyses: &self.analyses,
            repair_review: self.repair_review.as_ref(),
            preview: &self.preview,
            exports: &self.exports,
            export_review: self.export_review.as_ref(),
            export_outcome: self.export_outcome.as_ref(),
            atlas: (&self.atlas).into(),
        }
        .serialize(serializer)
    }
}

fn check_submission(
    command: &Command,
    connections: &connections::Connections,
    session: &SessionId,
) -> Result<()> {
    if let Command::Instruction { text } = command {
        anyhow::ensure!(!text.trim().is_empty(), "Instruction is empty");
    }
    if command.uses_model() {
        anyhow::ensure!(
            connections
                .gateway
                .as_ref()
                .is_some_and(|gateway| gateway.is_authorized(session))
                && crate::judge::authorized(connections.judge.as_deref(), session),
            "Review and approve the processor before sending"
        );
    }
    Ok(())
}

fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // Only presentation and process handles live here; authority stays in native kernels.
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn run(args: &WorkspaceArgs, assets: &Path, port: u16) -> Result<()> {
    anyhow::ensure!(
        args.model.approve_processor.is_none() && args.model.approve_judge.is_none(),
        "GUI processor approval happens in the browser; omit --approve-processor"
    );
    let assets = assets
        .canonicalize()
        .context("run npm run build, then select its directory with --assets")?;
    anyhow::ensure!(
        assets.join("index.html").is_file(),
        "GUI index.html is missing; run npm run build"
    );
    let pages = crate::load_pages(args.pages.as_deref())?;
    let workspace = Arc::new(Workspace::open(&args.workspace, &std::env::current_exe()?)?);
    // The cloud-first development flow uses host-selected connections. Never
    // deserialize model endpoints, credentials or policies from browser/model data.
    anyhow::ensure!(args.model.interactive() || args.model.tui_local_config()?.is_none(), "Prepared GGUF inference remains available in the TUI; the GUI selects existing HTTP connections");
    let connections = Arc::new(Mutex::new(connections::Connections::new(
        &args.model,
        workspace.session_id(),
    )?));
    let project = Arc::new(crate::project::Project::open(
        workspace.path(),
        workspace.session_id().as_bytes(),
        args.verifier.as_deref(),
    )?);
    let preview_views = meden_app::view::preview::Previews::default();
    let view = initial_view(&workspace, &project, preview_views.reader())?;
    lock(&connections).display(&mut lock(&view));
    let control = Arc::new(Mutex::new(Arc::new(Control::new(Arc::new(
        AtomicBool::new(false),
    )))));
    let server = Server::http((std::net::Ipv4Addr::LOCALHOST, port))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let server = Arc::new(server);
    let address = server
        .server_addr()
        .to_ip()
        .context("loopback listener missing")?;
    let origin = format!("http://{address}");
    let authority = address.to_string();
    let preview = Arc::new(preview::Preview::new(&origin, preview_views)?);
    let (sender, receiver) = mpsc::sync_channel::<Task>(1);
    let worker_view = Arc::clone(&view);
    let worker_workspace = Arc::clone(&workspace);
    let worker_connections = Arc::clone(&connections);
    let worker_project = Arc::clone(&project);
    let worker_preview = Arc::clone(&preview);
    let worker_thread = std::thread::spawn(move || {
        worker(
            receiver,
            &worker_view,
            &worker_workspace,
            &worker_connections,
            &worker_project,
            &pages,
            &worker_preview,
        );
    });
    let stopping = watch_shutdown(&server, &control)?;
    let preview_server = Arc::clone(&preview);
    let preview_stopping = Arc::clone(&stopping);
    let preview_thread = std::thread::spawn(move || preview_server.listen(&preview_stopping));
    let mut random = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
    let token = random.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    });
    println!("Meden GUI: {origin}/#session={token}");
    let http = Http {
        assets: &assets,
        authority: &authority,
        origin: &origin,
        token: &token,
        view: &view,
        control: &control,
        stopping: &stopping,
        connections: &connections,
        workspace: &workspace,
        project: &project,
        sender: &sender,
        preview: &preview,
    };
    let startup = Arc::clone(&lock(&control));
    {
        let mut view = lock(&view);
        view.running = true;
        view.activity = "Reading workspace".into();
    }
    sender.try_send(Task::new(Command::RefreshWorkspace, startup))?;
    http.listen(&server);
    project.cancel(&lock(&control));
    drop(sender);
    preview.clear();
    preview_thread
        .join()
        .map_err(|_| anyhow::anyhow!("Preview server stopped unexpectedly"))?;
    worker_thread
        .join()
        .map_err(|_| anyhow::anyhow!("Native session worker stopped unexpectedly"))?;
    Ok(())
}

fn initial_view(
    workspace: &Workspace,
    project: &crate::project::Project,
    preview: meden_app::view::preview::Reader,
) -> Result<Arc<Mutex<View>>> {
    project.stored_status()?;
    let saved = history::load(workspace.path())?;
    let analyses = match crate::analysis::journal::list(workspace.path()) {
        Ok(history) => meden_app::view::analyses::Analyses::new(meden_app::view::analyses::Records {
            current: history.last().cloned(), history,
        }),
        Err(error) => meden_app::view::analyses::Analyses::unavailable(&format!("Meaning-analysis checkpoint unavailable: {error:#}. The file was retained; reread sources and rerun analysis in the GUI.")),
    };
    let view = Arc::new(Mutex::new(View {
        history: meden_app::view::history::History::new(meden_app::view::history::Data {
            portable: crate::handoff::view(
                workspace,
                &Control::new(Arc::new(AtomicBool::new(false))),
            ),
            messages: saved.transcript,
            logs: Vec::new(),
        }),
        workspace: workspace.path().display().to_string(),
        endpoint: String::new(),
        model: String::new(),
        provider: None,
        connections: Vec::new(),
        connection_revision: None,
        models: Vec::new(),
        models_error: None,
        models_provider: None,
        overview: overview::Status::default(),
        overview_snapshot: None,
        judge: None,
        authorized: false,
        approval_available: false,
        running: false,
        operation: Operation::Workspace,
        chat_stopped: saved.stopped.clone(),
        activity: "Ready".into(),
        partial: String::new(),
        reasoning: String::new(),
        project: project.evidence_view()?,
        verification_job: project.verification_view(),
        work_job: project.work_view(),
        stopped: saved.stopped,
        directories: meden_app::view::directory::Directories::default(),
        readers: meden_app::view::readers::Readers::default(),
        sources: meden_app::view::sources::Sources::default(),
        source_preparation: meden_app::view::jobs::Jobs::new(workspace.session_id().as_bytes()),
        analyses,
        repair_review: None,
        preview,
        exports: Vec::new(),
        export_review: None,
        export_outcome: None,
        atlas: crate::atlas::Configuration::host(workspace.path()).status(),
    }));
    Ok(view)
}

fn watch_shutdown(
    server: &Arc<Server>,
    control: &Arc<Mutex<Arc<Control>>>,
) -> Result<Arc<AtomicBool>> {
    let mut signals = signal_hook::iterator::Signals::new([
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
    ])?;
    let stopping = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stopping);
    let stop_server = Arc::clone(server);
    let stop_control = Arc::clone(control);
    std::thread::spawn(move || {
        if signals.forever().next().is_some() {
            stopped.store(true, std::sync::atomic::Ordering::SeqCst);
            lock(&stop_control).cancel();
            stop_server.unblock();
        }
    });
    Ok(stopping)
}

struct Http<'a> {
    assets: &'a Path,
    authority: &'a str,
    origin: &'a str,
    token: &'a str,
    view: &'a Mutex<View>,
    control: &'a Mutex<Arc<Control>>,
    stopping: &'a AtomicBool,
    connections: &'a Mutex<connections::Connections>,
    workspace: &'a Workspace,
    project: &'a crate::project::Project,
    sender: &'a mpsc::SyncSender<Task>,
    preview: &'a preview::Preview,
}
impl Http<'_> {
    fn listen(&self, server: &Server) {
        // Browsers fetch module-preload chunks concurrently. A serial response
        // loop can block on one large asset while its dependencies wait behind it.
        // The native operation queue remains independently serial.
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    while !self.stopping.load(std::sync::atomic::Ordering::SeqCst) {
                        match server.recv_timeout(std::time::Duration::from_millis(200)) {
                            Ok(Some(request)) => self.handle(request),
                            Ok(None) => {}
                            Err(_) => break,
                        }
                    }
                });
            }
        });
    }

    fn handle(&self, request: Request) {
        if header(&request, "Host") != Some(self.authority)
            || header(&request, "Origin").is_some_and(|value| value != self.origin)
            || header(&request, "Sec-Fetch-Site")
                .is_some_and(|value| !matches!(value, "same-origin" | "none"))
        {
            respond(
                request,
                403,
                &json!({"error":"Open the exact loopback URL printed by meden gui"}),
            );
            return;
        }
        if !request.url().starts_with("/api/") {
            serve_asset(request, self.assets, &self.preview.frame_source());
            return;
        }
        if header(&request, "Authorization") != Some(format!("Bearer {}", self.token).as_str()) {
            respond(
                request,
                401,
                &json!({"error":"Session unavailable. Open the URL printed by meden gui."}),
            );
            return;
        }
        if request.method() == &Method::Get && request.url() == "/api/metrics" {
            respond(request, 200, &json!(meden_runtime::metrics::snapshot()));
            return;
        }
        if request.method() == &Method::Get {
            if let Some((span, bytes)) = self.publication(request.url()) {
                respond_publication(request, bytes, span);
                return;
            }
        }
        if request.method() == &Method::Get && request.url() == "/api/state" {
            let _timing = meden_runtime::metrics::Span::start("gui.state");
            let snapshot = {
                let mut view = lock(self.view);
                let connections = lock(self.connections);
                view.authorized = connections
                    .gateway
                    .as_ref()
                    .is_some_and(|gateway| gateway.is_authorized(self.workspace.session_id()))
                    && crate::judge::authorized(
                        connections.judge.as_deref(),
                        self.workspace.session_id(),
                    );
                json!(&*view)
            };
            respond(request, 200, &snapshot);
            return;
        }
        if request.method() == &Method::Get {
            if let Some(encoded) = request.url().strip_prefix("/api/map?inventory=") {
                let (revision, encoded) = encoded.split_once("&path=").unwrap_or((encoded, ""));
                let revision = percent_encoding::percent_decode_str(revision)
                    .decode_utf8()
                    .map(std::borrow::Cow::into_owned);
                let path = percent_encoding::percent_decode_str(encoded)
                    .decode_utf8()
                    .map(std::borrow::Cow::into_owned);
                match path.map_err(anyhow::Error::new).and_then(|path| {
                    let revision = meden_app::view::publication::ArtifactId::parse(&revision?)?;
                    overview::cached(self.view, self.workspace, &revision, &path)
                }) {
                    Ok(bytes) => {
                        respond_publication(request, Some(bytes), "gui.inventory_chunk_respond");
                    }
                    Err(error) => respond(request, 409, &json!({"error":format!("{error:#}")})),
                }
                return;
            }
        }
        if request.method() != &Method::Post || request.url() != "/api/command" {
            respond(request, 404, &json!({"error":"Unknown route"}));
            return;
        }
        if header(&request, "Origin") != Some(self.origin)
            || header(&request, "Content-Type") != Some("application/json")
        {
            respond(request, 403, &json!({"error":"Same-origin JSON required"}));
            return;
        }
        self.submit(request);
    }

    fn publication(&self, url: &str) -> Option<(&'static str, Option<Arc<[u8]>>)> {
        let (role, encoded) = if let Some(revision) = url.strip_prefix("/api/preview?revision=") {
            (PublicationRole::Preview, revision)
        } else if let Some(revision) = url.strip_prefix("/api/history?revision=") {
            (PublicationRole::History, revision)
        } else if let Some(revision) = url.strip_prefix("/api/preparation-log?revision=") {
            (PublicationRole::PreparationLog, revision)
        } else if let Some(revision) = url.strip_prefix("/api/analysis?revision=") {
            (PublicationRole::Analysis, revision)
        } else if let Some(revision) = url.strip_prefix("/api/analysis-history?revision=") {
            (PublicationRole::AnalysisHistory, revision)
        } else if let Some(revision) = url.strip_prefix("/api/evidence?revision=") {
            (PublicationRole::Evidence, revision)
        } else if let Some(revision) = url.strip_prefix("/api/verification-job?revision=") {
            (PublicationRole::VerificationJob, revision)
        } else if let Some(revision) = url.strip_prefix("/api/directory?revision=") {
            (PublicationRole::Directory, revision)
        } else if let Some(revision) = url.strip_prefix("/api/inventory?revision=") {
            (PublicationRole::Inventory, revision)
        } else if let Some(revision) = url.strip_prefix("/api/reader?revision=") {
            (PublicationRole::Reader, revision)
        } else {
            (
                PublicationRole::Source,
                url.strip_prefix("/api/source?revision=")?,
            )
        };
        let bytes = percent_encoding::percent_decode_str(encoded)
            .decode_utf8()
            .ok()
            .and_then(|revision| {
                let view = lock(self.view);
                match role {
                    PublicationRole::Preview => query(&revision, |id| view.preview.bytes(id)),
                    PublicationRole::History => query(&revision, |id| view.history.bytes(id)),
                    PublicationRole::Source => query(&revision, |id| view.sources.bytes(id)),
                    PublicationRole::Reader => query(&revision, |id| view.readers.bytes(id)),
                    PublicationRole::Directory => query(&revision, |id| view.directories.bytes(id)),
                    PublicationRole::PreparationLog => {
                        query(&revision, |id| view.source_preparation.bytes(id))
                    }
                    PublicationRole::Analysis => {
                        query(&revision, |id| view.analyses.current_bytes(id))
                    }
                    PublicationRole::AnalysisHistory => {
                        query(&revision, |id| view.analyses.history_bytes(id))
                    }
                    PublicationRole::Evidence => query(&revision, |id| view.project.bytes(id)),
                    PublicationRole::VerificationJob => query(&revision, |id| {
                        view.verification_job
                            .bytes(id)
                            .or_else(|| view.work_job.bytes(id))
                    }),
                    PublicationRole::Inventory => query(&revision, |id| {
                        view.overview_snapshot.as_ref().and_then(|s| s.bytes(id))
                    }),
                }
            });
        Some((
            match role {
                PublicationRole::Preview => "gui.preview_respond",
                PublicationRole::History => "gui.history_respond",
                PublicationRole::Source => "gui.source_respond",
                PublicationRole::Reader => "gui.reader_respond",
                PublicationRole::Directory => "gui.directory_respond",
                PublicationRole::PreparationLog => "gui.preparation_log_respond",
                PublicationRole::Analysis => "gui.analysis_respond",
                PublicationRole::AnalysisHistory => "gui.analysis_history_respond",
                PublicationRole::Evidence => "gui.evidence_respond",
                PublicationRole::VerificationJob => "gui.verification_job_respond",
                PublicationRole::Inventory => "gui.inventory_respond",
            },
            bytes,
        ))
    }

    fn submit(&self, mut request: Request) {
        let _timing = meden_runtime::metrics::Span::start("gui.command");
        let result = (|| -> Result<()> {
            anyhow::ensure!(
                !self.stopping.load(std::sync::atomic::Ordering::SeqCst),
                "GUI is stopping"
            );
            let mut bytes = Vec::new();
            request
                .as_reader()
                .take(1_048_577)
                .read_to_end(&mut bytes)?;
            anyhow::ensure!(bytes.len() <= 1_048_576, "command exceeds 1 MiB");
            let command: Command = serde_json::from_slice(&bytes)?;
            match command {
                Command::ApproveProcessor {
                    connection_revision,
                    endpoint,
                    model,
                    judge,
                } => {
                    let mut view = lock(self.view);
                    connections::approve(
                        self.connections,
                        self.workspace.session_id(),
                        connection_revision.value(),
                        &endpoint,
                        &model,
                        judge.as_ref(),
                    )?;
                    view.approval_available = false;
                }
                Command::RevokeProcessor => {
                    let mut view = lock(self.view);
                    let connections = lock(self.connections);
                    if let Some(gateway) = &connections.gateway {
                        gateway.revoke_from_host()?;
                    }
                    if let Some(judge) = &connections.judge {
                        judge.revoke_from_host()?;
                    }
                    view.approval_available = false;
                    self.project.cancel(&lock(self.control));
                }
                Command::SelectConnection { provider, model } => {
                    let mut view = lock(self.view);
                    anyhow::ensure!(!view.running, "Wait for the current operation or cancel it");
                    let mut connections = lock(self.connections);
                    connections.select(self.workspace.session_id(), &provider, &model)?;
                    connections.display(&mut view);
                    view.models.clear();
                    view.models_error = None;
                    view.models_provider = None;
                    view.repair_review = None;
                }
                Command::Cancel => self.project.cancel(&lock(self.control)),
                command => {
                    let mut view = lock(self.view);
                    anyhow::ensure!(!view.running, "Wait for the current operation or cancel it");
                    check_submission(
                        &command,
                        &lock(self.connections),
                        self.workspace.session_id(),
                    )?;
                    let preserve_stop = command.operation() == Operation::Inspection;
                    view.operation = command.operation();
                    if view.operation == Operation::Conversation {
                        view.chat_stopped = None;
                    }
                    // Reserve before enqueueing: concurrent tabs cannot queue a second turn.
                    let processes = Arc::new(Control::new(Arc::new(AtomicBool::new(false))));
                    *lock(self.control) = Arc::clone(&processes);
                    self.sender
                        .try_send(Task::new(command, processes))
                        .context("Native session unavailable")?;
                    view.running = true;
                    view.activity = "Preparing".into();
                    view.partial.clear();
                    view.reasoning.clear();
                    if !preserve_stop {
                        view.stopped = None;
                    }
                }
            }
            Ok(())
        })();
        match result {
            Ok(()) => respond(request, 202, &json!({"accepted":true})),
            Err(error) => respond(request, 409, &json!({"error":format!("{error:#}")})),
        }
    }
}

#[allow(clippy::too_many_lines)] // One flat command dispatcher; effects stay in their own modules.
fn worker(
    receiver: mpsc::Receiver<Task>,
    view: &Arc<Mutex<View>>,
    workspace: &Arc<Workspace>,
    connections: &Mutex<connections::Connections>,
    project: &Arc<crate::project::Project>,
    pages: &Arc<meden_runtime::pages::Pages>,
    preview: &preview::Preview,
) {
    let mut delivery = meden_app::delivery::Delivery::new(project);
    let mut repairs = crate::analysis::repair::Queue::default();
    for Task {
        command,
        processes,
        timing,
        queued,
    } in receiver
    {
        meden_runtime::metrics::queue(timing, queued);
        let _timing = meden_runtime::metrics::Span::linked("gui.job", Some(timing));
        let checkpoint_after = command.operation() != Operation::Inspection;
        delivery::begin(&mut delivery, &command);
        let result = (|| -> Result<()> {
            repair::begin(view, workspace, &mut repairs, &command)?;
            let exports = Arc::clone(&delivery.queue);
            let session = || {
                let connections = lock(connections);
                Session::new(
                    Arc::clone(workspace),
                    connections
                        .gateway
                        .clone()
                        .context("Choose a provider and model first")?,
                    Arc::clone(&processes),
                    &exports,
                    pages,
                    project,
                )
                .map(|session| session.with_judge(connections.judge.clone()))
            };
            let instruction = match command {
                Command::RefreshConnections => {
                    let choices = {
                        let mut connections = lock(connections);
                        connections.refresh();
                        connections.choices()
                    };
                    lock(view).connections = choices;
                    None
                }
                Command::RefreshWorkspace => {
                    overview::refresh(view, workspace, &processes)?;
                    None
                }
                command @ (Command::InitializeWork { .. }
                | Command::PutWork { .. }
                | Command::NoteWork { .. }
                | Command::CheckWork { .. }
                | Command::ResumeWork { .. }
                | Command::SaveWork) => {
                    perform_work(command, workspace, &processes, project, view)?
                }
                Command::RefreshStatus => {
                    lock(view).atlas = crate::atlas::Configuration::host(workspace.path()).status();
                    None
                }
                Command::PrepareAtlas => {
                    lock(view).activity = "Preparing Lean Atlas offline".into();
                    let result =
                        crate::atlas::Configuration::host(workspace.path()).prepare(&processes);
                    lock(view).atlas = crate::atlas::Configuration::host(workspace.path()).status();
                    result?;
                    None
                }
                Command::ListModels { provider } => {
                    let result = lock(connections).models(&provider);
                    let mut view = lock(view);
                    view.models_error = result.as_ref().err().map(ToString::to_string);
                    view.models = result.unwrap_or_default();
                    view.models_provider = Some(provider);
                    None
                }
                Command::AtlasSnapshot { paths } => {
                    sources::extract_atlas(view, workspace, &processes, &paths)?;
                    None
                }
                command @ (Command::ReviewExport { .. }
                | Command::SendExport { .. }
                | Command::RejectExport { .. }
                | Command::CloseExport { .. }) => {
                    delivery::perform(&mut delivery, command, workspace, &processes)?;
                    None
                }
                Command::CapturePreview { root, entry } => {
                    preview.capture(workspace, &processes, &root, &entry)?;
                    None
                }
                command @ (Command::PrepareRepair { .. }
                | Command::ApplyRepair { .. }
                | Command::RejectRepair { .. }) => {
                    repair::perform(view, &session()?, &mut repairs, command)?;
                    None
                }
                Command::SelectAnalysis { id } => {
                    let record = crate::analysis::journal::select(workspace.path(), &id)?;
                    lock(view).analyses.select(Some(record));
                    None
                }
                Command::AnalyzeMeaning { input } => {
                    analysis::run(view, &session()?, input)?;
                    None
                }
                Command::SourceSnapshot { paths } => {
                    sources::refresh(view, workspace, &processes, &paths)?;
                    None
                }
                Command::Instruction { text } => Some(instruction(view, text)),
                Command::ReviewContract { reference } => {
                    project.review(&reference.original())?;
                    None
                }
                Command::AcceptContract { reference } => {
                    project.accept(&reference.original())?;
                    Some(project.continuation()?)
                }
                Command::RejectContract { reference } => {
                    project.reject(&reference.original())?;
                    None
                }
                Command::Inspect { path, directory } => {
                    if directory {
                        let directory =
                            meden_app::view::directory::observe(workspace, &processes, &path)?;
                        lock(view).directories.publish(directory);
                    } else {
                        read_source(view, workspace, &processes, &path)?;
                    }
                    None
                }
                _ => anyhow::bail!("Unexpected worker command"),
            };
            if let Some(instruction) = instruction {
                preview.clear();
                run_instruction(&mut session()?, &instruction, view, workspace)?;
                if lock(view).overview.state == overview::State::Stale
                    && !processes.flag().load(std::sync::atomic::Ordering::SeqCst)
                {
                    // Read-only navigation refresh; it cannot admit native evidence.
                    let activity = lock(view).activity.clone();
                    overview::refresh(view, workspace, &processes)?;
                    lock(view).activity = activity;
                }
            }
            // This observes real source freshness, not only the last saved report.
            project
                .status(workspace, &processes)
                .or_else(|_| project.stored_status())?;
            Ok(())
        })();
        if checkpoint_after {
            checkpoint(view, workspace);
        }
        finish_operation(view, &delivery, result);
    }
}

fn checkpoint(view: &Mutex<View>, workspace: &Workspace) {
    let logging = Control::new(Arc::new(AtomicBool::new(false)));
    if let Err(error) = crate::handoff::checkpoint(workspace, &logging) {
        lock(view).history.edit(|data| {
            data.logs.push(meden_app::view::history::Log::Notice {
                text: format!("Portable history was retained but could not be saved: {error:#}"),
            });
        });
    }
    let portable = crate::handoff::view(workspace, &logging);
    lock(view).history.edit(|data| data.portable = portable);
}

fn finish_operation(
    view: &Mutex<View>,
    delivery: &meden_app::delivery::Delivery,
    result: Result<()>,
) {
    let mut view = lock(view);
    if let Err(error) = delivery::observe(delivery, &mut view) {
        view.stopped = Some(format!("{error:#}"));
    }
    if let Err(error) = result {
        view.stopped = Some(format!("{error:#}"));
    }
    if view.operation == Operation::Conversation {
        let stopped = view.stopped.clone();
        view.chat_stopped = stopped;
    }
    view.running = false;
}

fn run_instruction(
    session: &mut Session,
    instruction: &str,
    view: &Mutex<View>,
    workspace: &Workspace,
) -> Result<()> {
    let status = session.run(instruction, &|event| observe(view, &event));
    let mut current = lock(view);
    current.partial.clear();
    current.stopped = session.stop_report().map(str::to_owned);
    current.activity = match status {
        Status::Done(_) => "Complete",
        Status::Failed(_) => "Failed",
        Status::Incomplete => "Incomplete",
        Status::Cancelled => "Cancelled",
        Status::NeedsReview => "Contract review required",
    }
    .into();
    if let Status::Failed(error) = status {
        current.stopped.get_or_insert(error);
    }
    let transcript = current
        .history
        .data()
        .messages
        .iter()
        .map(|message| history::Message {
            speaker: message.speaker,
            text: message.text.clone(),
        })
        .collect();
    history::save_transcript(workspace.path(), transcript, current.stopped.clone())?;
    Ok(())
}

fn read_source(
    view: &Arc<Mutex<View>>,
    workspace: &Workspace,
    processes: &Control,
    path: &str,
) -> Result<()> {
    let timing = meden_runtime::metrics::Span::start("gui.reader_prepare");
    let observation = meden_app::view::file::observe(workspace, processes, path)?;
    sources::observe(view, &observation);
    let readable = observation.is_text();
    let available = matches!(
        &observation.reading,
        meden_app::view::file::Reading::Ready { .. }
    );
    let reader = meden_app::view::readers::Prepared::new(observation)?;
    lock(view).readers.publish(reader);
    drop(timing);
    if readable {
        sources::prepare(view, workspace, processes, path, true)?;
    }
    if available {
        overview::observe_file(view, workspace, processes, path);
    }
    Ok(())
}

fn observe(view: &Mutex<View>, event: &Event<'_>) {
    let mut view = lock(view);
    match event {
        Event::Asking(attempt) => {
            view.activity = format!("Waiting for model · request {attempt}");
            view.partial.clear();
            view.reasoning.clear();
        }
        Event::Delta(Delta::Text(text)) => bounded_append(&mut view.partial, text),
        Event::Delta(Delta::Reasoning(text)) => bounded_append(&mut view.reasoning, text),
        Event::Assistant(reply) => {
            if reply.content.as_ref().is_some_and(|text| !text.is_empty())
                || !reply.tool_calls.is_empty()
            {
                view.history.edit(|data| {
                    if let Some(text) = reply.content.as_ref().filter(|text| !text.is_empty()) {
                        data.messages.push(history::Message {
                            speaker: history::Speaker::Assistant,
                            text: text.clone(),
                        });
                    }
                    data.logs.extend(reply.tool_calls.iter().map(|call| {
                        meden_app::view::history::Log::Call {
                            id: call.id.clone(),
                            tool: call.function.name.clone(),
                            text: call.function.arguments.clone(),
                        }
                    }));
                });
            }
            view.partial.clear();
        }
        Event::Decision { call, refusal } => {
            if refusal.is_none()
                && matches!(
                    call.function.name.as_str(),
                    "write_file" | "edit_file" | "bash" | "check_lean_work"
                )
            {
                view.overview.state = overview::State::Stale;
            }
            if refusal.is_none() && call.function.name == "check_lean_work" {
                view.project.invalidate();
                view.sources.clear();
                view.analyses.invalidate();
            }
            view.activity = format!(
                "{} · {}",
                if refusal.is_some() {
                    "Refused"
                } else {
                    "Running"
                },
                call.function.name
            );
        }
        Event::ToolResult { call, text, error } => {
            view.history.edit(|data| {
                data.logs.push(meden_app::view::history::Log::Result {
                    id: call.id.clone(),
                    tool: call.function.name.clone(),
                    text: (*text).into(),
                    error: *error,
                });
            });
        }
        Event::ToolOutput { progress, .. } => {
            view.partial.clear();
            bounded_append(&mut view.partial, progress.stdout);
            bounded_append(&mut view.partial, progress.stderr);
        }
        Event::Notice(text) => view.activity = (*text).into(),
        Event::ModelFailed(error) => view.activity = error.to_string(),
    }
}

fn bounded_append(target: &mut String, text: &str) {
    let remaining = 131_072usize.saturating_sub(target.len());
    target.extend(text.chars().scan(remaining, |remaining, c| {
        *remaining = remaining.checked_sub(c.len_utf8())?;
        Some(c)
    }));
}
fn query<D>(
    revision: &str,
    read: impl FnOnce(&meden_app::view::publication::ArtifactId<D>) -> Option<Arc<[u8]>>,
) -> Option<Arc<[u8]>> {
    let original = meden_app::view::publication::ArtifactId::parse(revision).ok()?;
    read(&original)
}

fn header<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.to_string().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}
fn response_headers(content_type: &str, frame_source: &str) -> Vec<Header> {
    let policy = format!("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-src {frame_source}; frame-ancestors 'none'; base-uri 'none'; form-action 'none'");
    [
        ("Content-Type", content_type),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
        ("Cross-Origin-Resource-Policy", "same-origin"),
        ("Content-Security-Policy", policy.as_str()),
    ]
    .into_iter()
    .filter_map(|(name, value)| Header::from_bytes(name, value).ok())
    .collect()
}
fn respond_publication(request: Request, bytes: Option<Arc<[u8]>>, span: &'static str) {
    match bytes {
        Some(bytes) => {
            let _timing = meden_runtime::metrics::Span::start(span);
            let length = bytes.len();
            let response = Response::new(
                StatusCode(200),
                response_headers("application/json", "'none'"),
                std::io::Cursor::new(bytes),
                Some(length),
                None,
            );
            let _ = request.respond(response);
        }
        None => respond(
            request,
            409,
            &json!({"error":"View revision is no longer retained; refresh its reference"}),
        ),
    }
}

fn respond(request: Request, status: u16, value: &Value) {
    let _timing = meden_runtime::metrics::Span::start("gui.serialize_respond");
    let mut response =
        Response::from_string(value.to_string()).with_status_code(StatusCode(status));
    for header in response_headers("application/json", "'none'") {
        response.add_header(header);
    }
    let _ = request.respond(response);
}
fn serve_asset(request: Request, assets: &Path, frame_source: &str) {
    if request.method() != &Method::Get {
        respond(request, 405, &json!({"error":"GET required"}));
        return;
    }
    let relative = request.url().trim_start_matches('/');
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    if Path::new(relative)
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        respond(request, 404, &json!({"error":"Not found"}));
        return;
    }
    let file = assets
        .join(relative)
        .canonicalize()
        .ok()
        .filter(|p| p.starts_with(assets) && p.is_file());
    let Some(file) = file else {
        respond(request, 404, &json!({"error":"Not found"}));
        return;
    };
    let mime = match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("woff2") => "font/woff2",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    };
    match std::fs::File::open(&file) {
        Ok(file) => {
            let mut response = Response::from_file(file);
            for header in response_headers(mime, frame_source) {
                response.add_header(header);
            }
            let _ = request.respond(response);
        }
        Err(_) => respond(request, 404, &json!({"error":"Not found"})),
    }
}

fn perform_work(
    command: Command,
    workspace: &Workspace,
    processes: &Control,
    project: &crate::project::Project,
    view: &Mutex<View>,
) -> Result<Option<String>> {
    Ok(match command {
        Command::InitializeWork { name } => {
            crate::handoff::initialize(workspace, processes, name)?;
            None
        }
        Command::PutWork { work } => {
            crate::handoff::put(workspace, processes, work)?;
            None
        }
        Command::NoteWork { id, text, next } => {
            crate::handoff::note(workspace, processes, &id, text, next)?;
            None
        }
        Command::CheckWork { id } => {
            lock(view).project.invalidate();
            lock(view).sources.clear();
            lock(view).analyses.invalidate();
            lock(view).activity = format!("Checking Lean work {id}");
            // The shared check publishes every result in work history. Keep its
            // failures local instead of adding a second workspace-wide banner.
            let _result = crate::handoff::check(workspace, processes, &id, Some(project));
            None
        }
        Command::ResumeWork { id } => Some(instruction(
            view,
            crate::handoff::continuation(workspace, processes, &id)?,
        )),
        Command::SaveWork => None,
        _ => anyhow::bail!("Unexpected portable work command"),
    })
}

fn instruction(view: &Mutex<View>, text: String) -> String {
    let mut view = lock(view);
    view.sources.clear();
    view.analyses.invalidate();
    view.history.edit(|data| {
        data.messages.push(history::Message {
            speaker: history::Speaker::User,
            text: text.clone(),
        });
    });
    text
}
