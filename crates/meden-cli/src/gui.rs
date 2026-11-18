//! Loopback HTTP adapter. Native sessions, policies and checkpoints remain authoritative.
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Component, Path};
use std::sync::{atomic::AtomicBool, mpsc, Arc, Mutex};

use anyhow::{Context, Result};
use meden_agent::conversation::{Delta, Event, Status};
use meden_llm::Gateway;
use meden_runtime::{process::Control, workspace::Workspace};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::{session::history, session::Session, WorkspaceArgs};
mod analysis;
mod assets;
mod delivery;
mod preview;
mod sources;

struct Task {
    command: Command,
    processes: Arc<Control>,
}

#[derive(Serialize)]
struct View {
    workspace: String,
    endpoint: String,
    model: String,
    authorized: bool,
    approval_available: bool,
    running: bool,
    activity: String,
    partial: String,
    reasoning: String,
    messages: Vec<history::Message>,
    project: crate::project::Summary,
    stopped: Option<String>,
    logs: Vec<Value>,
    inspection: Option<Value>,
    source_snapshot: Option<sources::Snapshot>,
    analysis: Option<analysis::Record>,
    preview: Option<preview::Summary>,
    exports: Vec<crate::exports::Summary>,
    export_review: Option<delivery::Review>,
    export_outcome: Option<delivery::Outcome>,
    atlas: crate::atlas::Status,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    ApproveProcessor {
        endpoint: String,
    },
    RevokeProcessor,
    Instruction {
        text: String,
    },
    Cancel,
    RefreshStatus,
    PrepareAtlas,
    AtlasSnapshot {
        paths: Vec<String>,
    },
    ReviewExport {
        reference: crate::exports::Reference,
        offset: usize,
    },
    SendExport {
        reference: crate::exports::Reference,
        serial: u64,
    },
    RejectExport {
        reference: crate::exports::Reference,
        serial: u64,
    },
    CloseExport {
        reference: crate::exports::Reference,
        serial: u64,
    },
    CapturePreview {
        root: String,
        entry: String,
    },
    SourceSnapshot {
        paths: Vec<String>,
    },
    AnalyzeMeaning {
        input: crate::semantic::Input,
    },
    ReviewContract {
        reference: crate::project::Reference,
    },
    AcceptContract {
        reference: crate::project::Reference,
    },
    RejectContract {
        reference: crate::project::Reference,
    },
    Inspect {
        path: String,
        directory: bool,
    },
}

impl Command {
    fn check_submission(&self, gateway: &Gateway, session: &[u8]) -> Result<()> {
        if let Self::Instruction { text } = self {
            anyhow::ensure!(!text.trim().is_empty(), "Instruction is empty");
        }
        if matches!(self, Self::Instruction { .. } | Self::AnalyzeMeaning { .. }) {
            anyhow::ensure!(
                gateway.is_authorized(session),
                "Review and approve the processor before sending"
            );
        }
        Ok(())
    }
}

fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    // Only presentation and process handles live here; authority stays in native kernels.
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn run(args: &WorkspaceArgs, assets: &Path, port: u16) -> Result<()> {
    anyhow::ensure!(
        args.model.approve_processor.is_none(),
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
    anyhow::ensure!(args.model.tui_local_config()?.is_none(), "GUI currently requires --provider or --base-url/--model; local inference remains available in the TUI");
    let gateway = Arc::new(Gateway::prepare(
        workspace.session_id(),
        args.model.connection()?,
    )?);
    let project = Arc::new(crate::project::Project::open(
        workspace.path(),
        workspace.session_id(),
    )?);
    let view = initial_view(&workspace, &gateway, &project)?;
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
    let preview = Arc::new(preview::Preview::new(&origin)?);
    let (sender, receiver) = mpsc::sync_channel::<Task>(1);
    let worker_view = Arc::clone(&view);
    let worker_workspace = Arc::clone(&workspace);
    let worker_gateway = Arc::clone(&gateway);
    let worker_project = Arc::clone(&project);
    let worker_preview = Arc::clone(&preview);
    let worker_thread = std::thread::spawn(move || {
        worker(
            receiver,
            &worker_view,
            &worker_workspace,
            &worker_gateway,
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
        gateway: &gateway,
        workspace: &workspace,
        project: &project,
        sender: &sender,
        preview: &preview,
    };
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
    gateway: &Gateway,
    project: &crate::project::Project,
) -> Result<Arc<Mutex<View>>> {
    let saved = history::load(workspace.path())?;
    let (analysis, analysis_error) = match analysis::load(workspace.path()) {
        Ok(record) => (record, None),
        Err(error) => (None, Some(format!("Meaning-analysis checkpoint unavailable: {error:#}. The file was retained; reread sources and rerun analysis in the GUI."))),
    };
    let view = Arc::new(Mutex::new(View {
        workspace: workspace.path().display().to_string(),
        endpoint: gateway.endpoint().into(),
        model: gateway.provider().model.clone(),
        authorized: false,
        approval_available: true,
        running: false,
        activity: "Ready".into(),
        partial: String::new(),
        reasoning: String::new(),
        messages: saved.transcript,
        project: project.stored_status()?,
        stopped: analysis_error.or(saved.stopped),
        logs: Vec::new(),
        inspection: None,
        source_snapshot: None,
        analysis,
        preview: None,
        exports: Vec::new(),
        export_review: None,
        export_outcome: None,
        atlas: crate::atlas::Configuration::host().status(),
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
    gateway: &'a Gateway,
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

    fn handle(&self, mut request: Request) {
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
        if request.method() == &Method::Get && request.url() == "/api/state" {
            let snapshot = {
                let mut view = lock(self.view);
                view.authorized = self.gateway.is_authorized(self.workspace.session_id());
                view.preview = self.preview.summary();
                json!(&*view)
            };
            respond(request, 200, &snapshot);
            return;
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
                Command::ApproveProcessor { endpoint } => {
                    anyhow::ensure!(
                        endpoint == self.gateway.endpoint(),
                        "processor review target changed"
                    );
                    self.gateway.approve_from_host()?;
                    lock(self.view).approval_available = false;
                }
                Command::RevokeProcessor => {
                    self.gateway.revoke_from_host()?;
                    lock(self.view).approval_available = false;
                    self.project.cancel(&lock(self.control));
                }
                Command::Cancel => self.project.cancel(&lock(self.control)),
                command => {
                    let mut view = lock(self.view);
                    anyhow::ensure!(!view.running, "Wait for the current operation or cancel it");
                    command.check_submission(self.gateway, self.workspace.session_id())?;
                    // Reserve before enqueueing: concurrent tabs cannot queue a second turn.
                    let processes = Arc::new(Control::new(Arc::new(AtomicBool::new(false))));
                    *lock(self.control) = Arc::clone(&processes);
                    self.sender
                        .try_send(Task { command, processes })
                        .context("Native session unavailable")?;
                    view.running = true;
                    view.activity = "Preparing".into();
                    view.partial.clear();
                    view.reasoning.clear();
                    view.stopped = None;
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

fn worker(
    receiver: mpsc::Receiver<Task>,
    view: &Mutex<View>,
    workspace: &Arc<Workspace>,
    gateway: &Arc<Gateway>,
    project: &Arc<crate::project::Project>,
    pages: &Arc<meden_runtime::pages::Pages>,
    preview: &preview::Preview,
) {
    let mut delivery = delivery::Delivery::new(project);
    for Task { command, processes } in receiver {
        delivery.begin(&command);
        let result = (|| -> Result<()> {
            let mut session = Session::new(
                Arc::clone(workspace),
                Arc::clone(gateway),
                Arc::clone(&processes),
                &delivery.queue,
                pages,
                project,
            )?;
            let instruction = match command {
                Command::RefreshStatus => None,
                Command::PrepareAtlas => {
                    lock(view).activity = "Lean Atlasライブラリをオフラインで準備".into();
                    let atlas = crate::atlas::Configuration::host();
                    let result = atlas.prepare(&processes);
                    lock(view).atlas = atlas.status();
                    result?;
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
                    delivery.perform(command, workspace, &processes)?;
                    None
                }
                Command::CapturePreview { root, entry } => {
                    preview.capture(workspace, &processes, &root, &entry)?;
                    None
                }
                Command::AnalyzeMeaning { input } => {
                    analysis::run(view, &session, input)?;
                    None
                }
                Command::SourceSnapshot { paths } => {
                    sources::refresh(view, workspace, &processes, &paths)?;
                    None
                }
                Command::Instruction { text } => {
                    lock(view).source_snapshot = None;
                    analysis::invalidate(&mut lock(view));
                    lock(view).messages.push(history::Message {
                        speaker: history::Speaker::User,
                        text: text.clone(),
                    });
                    Some(text)
                }
                Command::ReviewContract { reference } => {
                    lock(view).project = project.review(&reference)?;
                    None
                }
                Command::AcceptContract { reference } => {
                    lock(view).project = project.accept(&reference)?;
                    Some(project.continuation()?)
                }
                Command::RejectContract { reference } => {
                    lock(view).project = project.reject(&reference)?;
                    None
                }
                Command::Inspect { path, directory } => {
                    let observation = inspect(workspace, &processes, &path, directory)?;
                    lock(view).inspection = Some(observation);
                    None
                }
                _ => anyhow::bail!("Unexpected worker command"),
            };
            if let Some(instruction) = instruction {
                preview.clear();
                run_instruction(&mut session, &instruction, view, workspace)?;
            }
            // This observes real source freshness, not only the last saved report.
            let summary = session.project_status()?;
            lock(view).project = summary;
            Ok(())
        })();
        let mut view = lock(view);
        if let Err(error) = delivery.observe(&mut view) {
            view.stopped = Some(format!("{error:#}"));
        }
        if let Err(error) = result {
            view.stopped = Some(format!("{error:#}"));
        }
        view.running = false;
    }
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

fn inspect(
    workspace: &Arc<Workspace>,
    processes: &Arc<Control>,
    path: &str,
    directory: bool,
) -> Result<Value> {
    let (registry, policy) = crate::workspace::environment(workspace, processes)?;
    let call = meden_agent::conversation::ToolCall {
        id: "gui-inspect".into(),
        function: meden_agent::conversation::FunctionCall {
            name: if directory { "list_files" } else { "read_file" }.into(),
            arguments: serde_json::to_string(&json!({"path":path}))?,
        },
    };
    let result = meden_agent::conversation::execute(
        &call,
        &registry,
        &policy,
        &|_| {},
        processes.flag(),
        &|| None,
    );
    Ok(match result {
        Ok(text) => json!({"path":path,"directory":directory,"text":text}),
        Err(error) => json!({"path":path,"error":format!("{error:?}")}),
    })
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
            if let Some(text) = reply.content.as_ref().filter(|t| !t.is_empty()) {
                view.messages.push(history::Message {
                    speaker: history::Speaker::Assistant,
                    text: text.clone(),
                });
            }
            for call in &reply.tool_calls {
                view.logs.push(json!({"type":"call","id":call.id,"tool":call.function.name,"text":call.function.arguments}));
            }
            view.partial.clear();
        }
        Event::Decision { call, refusal } => {
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
            view.logs.push(json!({"type":"result","id":call.id,"tool":call.function.name,"text":text,"error":error}));
        }
        Event::ToolOutput { progress, .. } => {
            view.partial.clear();
            bounded_append(&mut view.partial, progress.stdout);
            bounded_append(&mut view.partial, progress.stderr);
        }
        Event::Notice(text) => view.activity = (*text).into(),
        Event::ModelFailed(error) => view.activity = error.to_string(),
    }
    let excess = view.logs.len().saturating_sub(128);
    view.logs.drain(..excess);
}

fn bounded_append(target: &mut String, text: &str) {
    let remaining = 131_072usize.saturating_sub(target.len());
    target.extend(text.chars().scan(remaining, |remaining, c| {
        *remaining = remaining.checked_sub(c.len_utf8())?;
        Some(c)
    }));
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
fn respond(request: Request, status: u16, value: &Value) {
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
