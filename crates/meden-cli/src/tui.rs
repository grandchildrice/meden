//! Interactive native tool-calling agent, backed by real workspace tools.

#[cfg(test)]
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::exports::{Exports, Reference};
use crate::text::visible;
use anyhow::Result;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use meden_agent::conversation::{Event, Status};
use meden_llm::{Gateway, Provider};
use meden_runtime::process::Control;
use meden_runtime::workspace::Workspace;
use ratatui::prelude::*;
use ratatui::widgets::ListState;

mod approval;
mod input;
mod view;
use input::Input;
use view::draw;

enum Update {
    Project(Box<crate::project::Summary>),
    ProjectDirty,
    Asking(usize),
    Text(String),
    ToolStarted {
        id: String,
        name: String,
        arguments: String,
        refused: Option<String>,
    },
    ToolFinished {
        id: String,
        name: String,
        arguments: String,
        text: String,
        error: bool,
    },
    Done {
        text: String,
        needs_review: bool,
        failed: bool,
    },
}
#[derive(PartialEq, Eq)]
enum Screen {
    Picking,
    ReviewingProcessor,
    ReviewingExport,
    ReviewingContract,
    Instructing,
    Running,
}
#[derive(Clone, Copy)]
enum Speaker {
    User,
    Assistant,
}

enum TranscriptLine {
    Message { speaker: Speaker, text: String },
    Tool(ToolLine),
    Notice { text: String, style: Style },
}

struct ToolLine {
    id: String,
    name: String,
    arguments: String,
    result: Option<String>,
    error: bool,
}

struct App {
    providers: Vec<Provider>,
    picker: ListState,
    root: PathBuf,
    workspace: Option<Arc<Workspace>>,
    gateway: Option<Arc<Gateway>>,
    exports: Arc<Exports>,
    project: Arc<crate::project::Project>,
    project_status: Option<crate::project::Summary>,
    contract_review: Option<crate::project::Summary>,
    pages: Arc<meden_runtime::pages::Pages>,
    reviewing_export: Option<Reference>,
    export_offset: usize,
    instruction: Input,
    screen: Screen,
    lines: Vec<TranscriptLine>,
    busy: bool,
    activity: Option<String>,
    show_tool_details: bool,
    rx: Option<Receiver<Update>>,
    worker: Option<Sender<String>>,
    worker_gateway: Option<Arc<Gateway>>,
    processes: Arc<Control>,
    scroll_back: u16,
    // Populated by drawing; changing a review target discards this readiness.
    review_area: Rect,
    review_frame: Rect,
    review_rows: u16,
    review_page: u16,
}

impl App {
    fn new(providers: Vec<Provider>, root: PathBuf) -> Self {
        let mut picker = ListState::default();
        if !providers.is_empty() {
            picker.select(Some(0));
        }
        let project = Arc::new(crate::project::Project::new(&root));
        let exports = Arc::new(Exports::new(&root, Arc::clone(&project)));
        Self {
            providers,
            picker,
            root,
            workspace: None,
            gateway: None,
            exports,
            project,
            project_status: None,
            contract_review: None,
            pages: Arc::default(),
            reviewing_export: None,
            export_offset: 0,
            instruction: Input::default(),
            screen: Screen::Picking,
            lines: Vec::new(),
            busy: false,
            activity: None,
            show_tool_details: false,
            rx: None,
            worker: None,
            worker_gateway: None,
            processes: Arc::new(Control::new(Arc::new(AtomicBool::new(false)))),
            scroll_back: 0,
            review_area: Rect::default(),
            review_frame: Rect::default(),
            review_rows: 0,
            review_page: 0,
        }
    }
    fn selected(&self) -> Option<&Provider> {
        self.providers.get(self.picker.selected().unwrap_or(0))
    }
    fn say(&mut self, text: impl Into<String>, style: Style) {
        self.lines.push(TranscriptLine::Notice {
            text: visible(&text.into()),
            style,
        });
    }
    fn speak(&mut self, speaker: Speaker, text: &str) {
        self.lines.push(TranscriptLine::Message {
            speaker,
            text: visible(text),
        });
    }
    fn tool_started(&mut self, id: String, name: &str, arguments: &str, refused: Option<String>) {
        self.activity = Some(format!("Running {}", visible(name)));
        self.lines.push(TranscriptLine::Tool(ToolLine {
            id,
            name: visible(name),
            arguments: visible(arguments),
            error: refused.is_some(),
            result: refused.map(|reason| visible(&reason)),
        }));
    }
    fn tool_finished(&mut self, id: &str, name: &str, arguments: &str, text: &str, error: bool) {
        if let Some(TranscriptLine::Tool(tool)) = self
            .lines
            .iter_mut()
            .rev()
            .find(|line| matches!(line, TranscriptLine::Tool(tool) if tool.id == id))
        {
            tool.result = Some(visible(text));
            tool.error = error;
        } else {
            // A stopped conversation can report skipped calls without a
            // preparation event. Preserve those original results as well.
            self.lines.push(TranscriptLine::Tool(ToolLine {
                id: id.to_owned(),
                name: visible(name),
                arguments: visible(arguments),
                result: Some(visible(text)),
                error,
            }));
        }
    }
    fn move_selection(&mut self, delta: isize) {
        if self.providers.is_empty() {
            return;
        }
        let current = self.picker.selected().unwrap_or(0);
        self.picker.select(Some(if delta < 0 {
            current.saturating_sub(1)
        } else {
            current
                .saturating_add(1)
                .min(self.providers.len().saturating_sub(1))
        }));
    }
    fn session_id(&self) -> &[u8] {
        self.root.as_os_str().as_encoded_bytes()
    }
    fn select_processor(&mut self) {
        self.review_area = Rect::default();
        let Some(provider) = self.selected().cloned() else {
            return;
        };
        if self.gateway.as_ref().is_some_and(|gateway| {
            gateway.provider() == &provider && gateway.is_authorized(self.session_id())
        }) {
            self.screen = Screen::Instructing;
            return;
        }
        if let Some(previous) = self.gateway.take() {
            let _ = previous.revoke_from_host();
        }
        self.worker = None;
        self.worker_gateway = None;
        self.rx = None;
        match Gateway::prepare(self.session_id(), provider) {
            Ok(gateway) => {
                self.gateway = Some(Arc::new(gateway));
                self.scroll_back = 0;
                self.screen = Screen::ReviewingProcessor;
            }
            Err(error) => self.say(error.to_string(), Style::new().fg(Color::Red)),
        }
    }
    fn approve_processor(&mut self) {
        let Some(gateway) = &self.gateway else {
            return;
        };
        match gateway.approve_from_host() {
            Ok(()) => self.screen = Screen::Instructing,
            Err(error) => self.say(error.to_string(), Style::new().fg(Color::Red)),
        }
    }
    fn review_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            _ if matches!(
                approval::route(
                    &self.screen,
                    &event::Event::Key(key),
                    self.review_area.width,
                    self.review_area.height,
                    self.review_rows
                ),
                Some(approval::Choice::Processor)
            ) =>
            {
                self.approve_processor();
            }
            KeyCode::Char('n') | KeyCode::Esc => self.screen = Screen::Picking,
            _ => {}
        }
    }
    fn open_export(&mut self, id: Option<usize>) {
        self.review_area = Rect::default();
        let result = self.exports.list().and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| {
                    id.map_or(matches!(entry.state, "pending" | "reviewed"), |id| {
                        entry.reference.id == id
                    })
                })
                .map(|entry| entry.reference)
                .ok_or_else(|| anyhow::anyhow!("No matching export proposal."))
        });
        match result {
            Ok(reference) => {
                if let Err(error) = self.exports.review(&reference) {
                    self.say(error.to_string(), Style::new().fg(Color::Red));
                    return;
                }
                self.reviewing_export = Some(reference);
                self.export_offset = 0;
                self.scroll_back = 0;
                self.screen = Screen::ReviewingExport;
            }
            Err(error) => self.say(error.to_string(), Style::new().fg(Color::Yellow)),
        }
    }
    fn export_key(&mut self, key: crossterm::event::KeyEvent) {
        if key.code == KeyCode::Esc {
            self.screen = Screen::Instructing;
            return;
        }
        if key.code == KeyCode::Left {
            self.export_offset = self.export_offset.saturating_sub(128);
            return;
        }
        if key.code == KeyCode::Right {
            if let Some(entry) = self
                .reviewing_export
                .as_ref()
                .and_then(|reference| self.exports.get(reference).ok())
            {
                if let Ok(summary) = entry.summary() {
                    let next = self.export_offset.saturating_add(128);
                    if next < summary.bytes {
                        self.export_offset = next;
                    }
                }
            }
            return;
        }
        let Some(reference) = self.reviewing_export.clone() else {
            return;
        };
        let result = match key.code {
            _ if matches!(
                approval::route(
                    &self.screen,
                    &event::Event::Key(key),
                    self.review_area.width,
                    self.review_area.height,
                    self.review_rows
                ),
                Some(approval::Choice::Export)
            ) =>
            {
                self.workspace
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("workspace unavailable"))
                    .and_then(|workspace| self.exports.send(&reference, workspace, &self.processes))
                    .map(|(summary, code)| {
                        format!(
                            "Export #{}: {} · HTTP {:?} · {}",
                            summary.reference.id, summary.state, code, summary.target
                        )
                    })
            }
            KeyCode::Char('n') if key.modifiers.is_empty() => self
                .exports
                .reject(&reference)
                .map(|summary| format!("Export #{} rejected.", summary.reference.id)),
            _ => return,
        };
        match result {
            Ok(message) => self.say(message, Style::new().fg(Color::Yellow)),
            Err(error) => self.say(error.to_string(), Style::new().fg(Color::Red)),
        }
        self.screen = Screen::Instructing;
        self.reviewing_export = None;
    }
    fn instruction_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Tab => self.screen = Screen::Picking,
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
                self.instruction.insert("\n");
            }
            KeyCode::Enter => self.start(),
            KeyCode::Backspace => {
                self.instruction.backspace();
            }
            KeyCode::Delete => self.instruction.delete(),
            KeyCode::Left => self.instruction.left(),
            KeyCode::Right => self.instruction.right(),
            KeyCode::Home => self.instruction.home(),
            KeyCode::End => self.instruction.end(),
            KeyCode::Up => self.instruction.up(),
            KeyCode::Down => self.instruction.down(),
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.instruction.home();
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.instruction.end();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.instruction.clear();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.instruction.insert(&c.to_string());
            }
            _ => {}
        }
    }
    fn open_contract(&mut self) {
        self.review_area = Rect::default();
        let result = self
            .project
            .current_reference()
            .and_then(|r| self.project.review(&r));
        match result {
            Ok(summary) => {
                self.contract_review = Some(summary.clone());
                self.project_status = Some(summary);
                self.scroll_back = 0;
                self.screen = Screen::ReviewingContract;
            }
            Err(error) => self.say(error.to_string(), Style::new().fg(Color::Red)),
        }
    }
    fn contract_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.screen != Screen::ReviewingContract {
            return;
        }
        let Some(review) = self.contract_review.clone() else {
            return;
        };
        match key.code {
            _ if matches!(
                approval::route(
                    &self.screen,
                    &event::Event::Key(key),
                    self.review_area.width,
                    self.review_area.height,
                    self.review_rows
                ),
                Some(approval::Choice::Contract)
            ) =>
            {
                if review.accepted {
                    self.screen = Screen::Instructing;
                    return;
                }
                match self.project.accept(&review.reference) {
                    Ok(summary) => {
                        self.project_status = Some(summary);
                        self.screen = Screen::Instructing;
                        match self.project.continuation() {
                            Ok(instruction) => {
                                self.instruction.insert(&instruction);
                                self.start_turn(false);
                            }
                            Err(error) => self.say(error.to_string(), Style::new().fg(Color::Red)),
                        }
                    }
                    Err(error) => self.say(error.to_string(), Style::new().fg(Color::Red)),
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                if !review.accepted {
                    match self.project.reject(&review.reference) {
                        Ok(summary) => self.project_status = Some(summary),
                        Err(error) => self.say(error.to_string(), Style::new().fg(Color::Red)),
                    }
                }
                self.contract_review = None;
                self.screen = Screen::Instructing;
            }
            _ => {}
        }
    }
    fn host_command(&mut self) -> bool {
        match self.instruction.text().trim() {
            "/models" => {
                self.instruction.clear();
                self.screen = Screen::Picking;
                return true;
            }
            "/workspace" => {
                self.instruction.clear();
                self.say(format!("Files: {}\nThis is the retained working copy. The original project is unchanged.", self.root.display()), Style::new());
                return true;
            }
            "/help" => {
                self.instruction.clear();
                self.say("/models     Choose a model connection\n/workspace  Show the generated files location\n/contract   Review the project specification\n/exports    Review proposed deliveries\n/clear      Start a fresh conversation\nCtrl+O      Expand or collapse tool details", Style::new());
                return true;
            }
            _ => {}
        }
        if self.instruction.text().trim() == "/contract" {
            self.instruction.clear();
            self.open_contract();
            return true;
        }
        if self.instruction.text().trim() == "/clear" {
            self.worker = None;
            self.rx = None;
            self.worker_gateway = None;
            self.instruction.clear();
            self.lines.clear();
            self.scroll_back = 0;
            self.say(
                "Conversation cleared. Workspace files retained.",
                Style::new().fg(Color::DarkGray),
            );
            return true;
        }
        let mut parts = self.instruction.text().split_whitespace();
        if parts.next() != Some("/exports") {
            return false;
        }
        let id = parts.next().map(str::parse::<usize>).transpose();
        let extra = parts.next().is_some();
        self.instruction.clear();
        match id {
            Ok(id) if !extra => self.open_export(id),
            _ => self.say("Use /exports or /exports ID.", Style::new().fg(Color::Red)),
        }
        true
    }
    fn start(&mut self) {
        self.start_turn(true);
    }
    fn start_turn(&mut self, show_instruction: bool) {
        if self.busy || self.instruction.text().trim().is_empty() {
            return;
        }
        if self.host_command() {
            return;
        }
        let Some(gateway) = self
            .gateway
            .as_ref()
            .filter(|gateway| {
                self.selected()
                    .is_some_and(|provider| gateway.provider() == provider)
                    && gateway.is_authorized(self.session_id())
            })
            .map(Arc::clone)
        else {
            self.select_processor();
            return;
        };
        if !self
            .worker_gateway
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &gateway))
            || self.worker.is_none()
        {
            let (commands, tasks) = mpsc::channel();
            let (updates, results) = mpsc::channel();
            let Some(workspace) = self.workspace.as_ref().map(Arc::clone) else {
                self.say(
                    "No isolated workspace available.",
                    Style::new().fg(Color::Red),
                );
                return;
            };
            let processes = Arc::clone(&self.processes);
            let exports = Arc::clone(&self.exports);
            let pages = Arc::clone(&self.pages);
            let project = Arc::clone(&self.project);
            self.worker = Some(commands);
            self.rx = Some(results);
            self.worker_gateway = Some(Arc::clone(&gateway));
            self.lines.clear();
            thread::spawn(move || {
                match crate::session::Session::new(
                    workspace, gateway, processes, &exports, &pages, &project,
                ) {
                    Ok(session) => worker(session, &tasks, &updates),
                    Err(error) => {
                        let _ = updates.send(Update::Done {
                            text: format!("Failed: {error}"),
                            needs_review: false,
                            failed: true,
                        });
                    }
                }
            });
        }
        self.processes.flag().store(false, Ordering::SeqCst);
        let instruction = self.instruction.take();
        if show_instruction {
            self.speak(Speaker::User, &instruction);
        } else {
            self.say(
                "Specification accepted. Continuing implementation.",
                Style::new().fg(Color::Gray),
            );
        }
        self.activity = Some("Thinking".into());
        self.scroll_back = 0;
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.send(instruction).is_ok())
        {
            self.busy = true;
            self.screen = Screen::Running;
        } else {
            self.say(
                "The worker stopped; enter your instruction again.",
                Style::new().fg(Color::Red),
            );
            self.worker = None;
        }
    }
    fn cancel(&mut self) {
        if self.busy && !self.processes.flag().load(Ordering::SeqCst) {
            self.project.cancel(&self.processes);
            self.say(
                "Stopping tools. The current model request may finish; Ctrl-C again quits.",
                Style::new().fg(Color::Yellow),
            );
        }
    }
    fn poll(&mut self) {
        let mut updates = Vec::new();
        let mut disconnected = false;
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(update) => updates.push(update),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        for update in updates {
            match update {
                Update::Project(summary) => {
                    if summary.verified
                        && !self.project_status.as_ref().is_some_and(|old| {
                            old.verified && old.artifact_sha256 == summary.artifact_sha256
                        })
                    {
                        if let Some(path) = &summary.artifact {
                            self.say(
                                format!(
                                    "Host: verified project kernel. Artifact: {}",
                                    path.display()
                                ),
                                Style::new().fg(Color::Green),
                            );
                        }
                    }
                    self.project_status = Some(*summary);
                }
                Update::ProjectDirty => {
                    if let Some(status) = &mut self.project_status {
                        status.verified = false;
                    }
                }
                Update::Asking(round) => {
                    self.activity = Some(format!("Thinking · request {round}"));
                }
                Update::Text(text) => self.speak(Speaker::Assistant, &text),
                Update::ToolStarted {
                    id,
                    name,
                    arguments,
                    refused,
                } => self.tool_started(id, &name, &arguments, refused),
                Update::ToolFinished {
                    id,
                    name,
                    arguments,
                    text,
                    error,
                } => self.tool_finished(&id, &name, &arguments, &text, error),
                Update::Done {
                    text,
                    needs_review,
                    failed,
                } => {
                    self.busy = false;
                    self.activity = None;
                    self.screen = Screen::Instructing;
                    self.say(
                        text,
                        Style::new().fg(if failed { Color::Red } else { Color::Gray }),
                    );
                    if needs_review {
                        self.open_contract();
                    }
                    if let Ok(entries) = self.exports.list() {
                        let pending = entries
                            .iter()
                            .filter(|entry| matches!(entry.state, "pending" | "reviewed"))
                            .count();
                        if pending > 0 {
                            self.say(format!("Host: {pending} export proposal(s) await review. Use /exports."), Style::new().fg(Color::Yellow));
                        }
                    }
                }
            }
        }
        if disconnected {
            if self.busy {
                self.say(
                    "The worker disconnected before finishing.",
                    Style::new().fg(Color::Red),
                );
            }
            self.busy = false;
            self.screen = Screen::Instructing;
            self.worker = None;
            self.rx = None;
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.project.cancel(&self.processes);
        if let Some(gateway) = &self.gateway {
            let _ = gateway.revoke_from_host();
        }
    }
}

fn worker(mut session: crate::session::Session, tasks: &Receiver<String>, tx: &Sender<Update>) {
    let observer = |event: Event<'_>| {
        let update = match event {
            Event::Asking(round) => Update::Asking(round),
            Event::Assistant(reply) => {
                if let Some(text) = reply.content.as_deref().filter(|s| !s.is_empty()) {
                    let _ = tx.send(Update::Text(text.to_owned()));
                }
                return;
            }
            Event::Decision { call, refusal } => Update::ToolStarted {
                id: call.id.clone(),
                name: call.function.name.clone(),
                arguments: call.function.arguments.clone(),
                refused: refusal.map(|(_, reason)| reason.to_owned()),
            },
            Event::ToolResult { call, text, error } => {
                let excerpt: String = text.chars().take(1200).collect();
                Update::ToolFinished {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    arguments: call.function.arguments.clone(),
                    text: format!(
                        "{excerpt}{}",
                        if excerpt.len() < text.len() {
                            "…"
                        } else {
                            ""
                        }
                    ),
                    error,
                }
            }
        };
        if let Event::Decision {
            call,
            refusal: None,
        } = event
        {
            if matches!(
                call.function.name.as_str(),
                "write_file" | "edit_file" | "bash"
            ) {
                let _ = tx.send(Update::ProjectDirty);
            }
        }
        let _ = tx.send(update);
    };
    while let Ok(instruction) = tasks.recv() {
        let status = session.run(&instruction, &observer);
        let needs_review = status == Status::NeedsReview;
        let failed = matches!(status, Status::Failed(_) | Status::Incomplete);
        let summary = match status {
            Status::Done(_) => {
                "Finished. Conversation retained for the next instruction.".to_owned()
            }
            Status::Failed(error) => format!("Failed: {error}"),
            Status::Incomplete => {
                "Unfinished: verification or tool execution limit reached.".to_owned()
            }
            Status::Cancelled => "Cancelled. Conversation retained.".to_owned(),
            Status::NeedsReview => "Review the proposed contract to continue.".to_owned(),
        };
        if let Ok(status) = session.project_status() {
            let _ = tx.send(Update::Project(Box::new(status)));
        }
        if tx
            .send(Update::Done {
                text: summary,
                needs_review,
                failed,
            })
            .is_err()
        {
            return;
        }
    }
}

/// Restore the terminal even when startup or rendering returns an error.
struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let guard = Self;
        io::stdout()
            .execute(EnterAlternateScreen)?
            .execute(EnableBracketedPaste)?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = io::stdout()
            .execute(DisableBracketedPaste)
            .and_then(|out| out.execute(LeaveAlternateScreen))
            .and_then(|out| out.execute(crossterm::cursor::Show));
    }
}

/// Open a native-tool-calling interface for the selected workspace.
pub fn run(
    providers: Vec<Provider>,
    root: &Path,
    approved: Option<&str>,
    pages: Arc<meden_runtime::pages::Pages>,
    local: Option<meden_llm::LocalConfig>,
) -> Result<()> {
    anyhow::ensure!(
        local.is_some() || !providers.is_empty(),
        "TUI requires an OpenAI-compatible HTTP provider; see `meden providers`"
    );
    let workspace = Arc::new(Workspace::import(root, &std::env::current_exe()?)?);
    let mut app = App::new(providers, workspace.path().to_owned());
    app.pages = pages;
    app.workspace = Some(workspace);
    if let Some(config) = local {
        let gateway = Gateway::prepare_local(
            app.session_id(),
            &config,
            &std::env::current_exe()?,
            Arc::clone(&app.processes),
        )?;
        app.providers = vec![gateway.provider().clone()];
        app.picker.select(Some(0));
        app.gateway = Some(Arc::new(gateway));
        app.screen = Screen::Instructing;
    } else if let Some(endpoint) = approved {
        let provider = app
            .selected()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no selected processor"))?;
        let gateway = Gateway::prepare(app.session_id(), provider)?;
        crate::approve_processor(&gateway, endpoint)?;
        app.gateway = Some(Arc::new(gateway));
        app.screen = Screen::Instructing;
    } else if app.providers.len() == 1 {
        app.select_processor();
    }
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    event_loop(&mut terminal, &mut app)
}

fn event_loop<B>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B: Backend,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    loop {
        app.poll();
        terminal.draw(|frame| draw(frame, app))?;
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let event = event::read()?;
        if let event::Event::Paste(text) = event {
            if app.screen == Screen::Instructing {
                app.instruction.insert(&text);
            }
            continue;
        }
        let event::Event::Key(key) = event else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if app.busy {
                if app.processes.flag().load(Ordering::SeqCst) {
                    app.project.cancel(&app.processes);
                    return Ok(());
                }
                app.cancel();
                continue;
            }
            return Ok(());
        }
        if key.code == KeyCode::Char('o')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(app.screen, Screen::Instructing | Screen::Running)
        {
            app.show_tool_details = !app.show_tool_details;
            continue;
        }
        if matches!(
            app.screen,
            Screen::ReviewingProcessor | Screen::ReviewingExport | Screen::ReviewingContract
        ) {
            let size = terminal.size()?;
            if size.width != app.review_frame.width || size.height != app.review_frame.height {
                app.review_area = Rect::default();
                continue;
            }
        }
        match key.code {
            KeyCode::PageUp => {
                app.scroll_back = if matches!(
                    app.screen,
                    Screen::ReviewingProcessor
                        | Screen::ReviewingExport
                        | Screen::ReviewingContract
                ) {
                    app.scroll_back.saturating_sub(app.review_page.max(1))
                } else {
                    app.scroll_back.saturating_add(10)
                };
                continue;
            }
            KeyCode::PageDown => {
                app.scroll_back = if matches!(
                    app.screen,
                    Screen::ReviewingProcessor
                        | Screen::ReviewingExport
                        | Screen::ReviewingContract
                ) {
                    app.scroll_back.saturating_add(app.review_page.max(1))
                } else {
                    app.scroll_back.saturating_sub(10)
                };
                continue;
            }
            KeyCode::End if app.screen != Screen::Instructing => {
                app.scroll_back = 0;
                continue;
            }
            _ => {}
        }
        match app.screen {
            Screen::Picking => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
                KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
                KeyCode::Enter => app.select_processor(),
                _ => {}
            },
            Screen::ReviewingProcessor => app.review_key(key),
            Screen::ReviewingExport => app.export_key(key),
            Screen::ReviewingContract => app.contract_key(key),
            Screen::Instructing => app.instruction_key(key),
            Screen::Running => {
                if key.code == KeyCode::Esc {
                    app.cancel();
                }
            }
        }
    }
}

/// Exercise every screen using an in-memory terminal.
#[cfg(test)]
fn selftest(providers: Vec<Provider>) -> Result<String> {
    let mut app = App::new(providers, PathBuf::from("/workspace"));
    app.select_processor();
    let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 24))?;
    let mut rendered = String::new();
    for screen in [
        Screen::Picking,
        Screen::ReviewingProcessor,
        Screen::ReviewingExport,
        Screen::ReviewingContract,
        Screen::Instructing,
        Screen::Running,
    ] {
        app.screen = screen;
        app.say("a line of transcript", Style::new());
        terminal.draw(|frame| draw(frame, &mut app))?;
        let _ = writeln!(rendered, "{:?}", terminal.backend().buffer().area);
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_and_file_text_cannot_emit_terminal_control_sequences() {
        let mut app = App::new(Vec::new(), PathBuf::new());
        app.say("日本語🙂\n\u{1b}]52;c;payload\u{7}", Style::new());
        assert!(app
            .lines
            .iter()
            .all(|line| matches!(line, TranscriptLine::Notice { text, .. }
                if !text.chars().any(|c| c.is_control() && c != '\n'))));
        assert!(app
            .lines
            .first()
            .is_some_and(|line| matches!(line, TranscriptLine::Notice { text, .. }
                if text.starts_with("日本語🙂\n"))));
    }
    fn providers() -> Vec<Provider> {
        vec![Provider {
            protocol: meden_llm::Protocol::ChatCompletions,
            id: "test".into(),
            label: "Test".into(),
            model: "m".into(),
            base_url: "http://127.0.0.1:1/v1".into(),
            api_key: None,
            note: String::new(),
        }]
    }
    #[test]
    fn every_screen_draws() {
        assert!(selftest(providers()).is_ok());
    }
    #[test]
    fn conversation_labels_and_tool_details_are_separate() -> Result<()> {
        let mut app = App::new(providers(), PathBuf::from("/workspace"));
        app.screen = Screen::Instructing;
        app.speak(Speaker::User, "Read note.txt.");
        app.tool_started("call-a".into(), "read_file", r#"{"path":"note.txt"}"#, None);
        app.tool_finished(
            "call-a",
            "read_file",
            r#"{"path":"note.txt"}"#,
            "TOOL-DETAILS-CANARY",
            false,
        );
        app.speak(Speaker::Assistant, "The file has been read.");
        assert_eq!(app.lines.len(), 3, "tool result duplicated its request row");
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 24))?;
        terminal.draw(|frame| draw(frame, &mut app))?;
        let painted: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(painted.contains("YOU") && painted.contains("MEDEN"));
        assert!(painted.contains("read_file") && painted.contains("note.txt"));
        assert!(!painted.contains("TOOL-DETAILS-CANARY"));
        app.show_tool_details = true;
        terminal.draw(|frame| draw(frame, &mut app))?;
        let expanded: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(expanded.contains("TOOL-DETAILS-CANARY"));
        Ok(())
    }
    #[test]
    fn skipped_tool_failures_remain_visible_without_expanding_details() -> Result<()> {
        let mut app = App::new(providers(), PathBuf::from("/workspace"));
        app.screen = Screen::Instructing;
        app.tool_finished(
            "skipped",
            "write_file",
            "{}",
            "Cancelled before preparation\u{1b}[31m",
            true,
        );
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 24))?;
        terminal.draw(|frame| draw(frame, &mut app))?;
        let painted: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(painted.contains("Cancelled before preparation"));
        assert!(!painted.contains('\u{1b}'));
        Ok(())
    }
    #[test]
    fn processor_cannot_be_approved_without_a_visible_review_area() -> Result<()> {
        use anyhow::Context as _;
        let mut app = App::new(providers(), PathBuf::from("/workspace/hidden-review"));
        app.select_processor();
        let gateway = Arc::clone(app.gateway.as_ref().context("pending gateway")?);
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(2, 20))?;
        for (width, height) in [(2, 20), (100, 2), (0, 0)] {
            terminal.backend_mut().resize(width, height);
            terminal.draw(|frame| draw(frame, &mut app))?;
            app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
            assert!(
                !gateway.is_authorized(app.session_id()),
                "an invisible processor was authorized"
            );
        }
        terminal.backend_mut().resize(100, 24);
        terminal.draw(|frame| draw(frame, &mut app))?;
        let painted: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(painted.contains(gateway.endpoint()));
        app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
        assert!(gateway.is_authorized(app.session_id()));
        Ok(())
    }
    #[test]
    fn processor_cannot_be_approved_with_a_clipped_destination() -> Result<()> {
        use anyhow::Context as _;
        let mut connections = providers();
        connections.first_mut().context("provider")?.base_url =
            format!("https://example.com/{}/destination-end", "a".repeat(240));
        let mut app = App::new(connections, PathBuf::from("/workspace/clipped-review"));
        app.select_processor();
        let gateway = Arc::clone(app.gateway.as_ref().context("pending gateway")?);
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(20, 24))?;
        terminal.draw(|frame| draw(frame, &mut app))?;
        let painted: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(!painted.contains("destination-end"));
        app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
        assert!(
            !gateway.is_authorized(app.session_id()),
            "a clipped destination was authorized"
        );
        terminal.backend_mut().resize(100, 24);
        terminal.draw(|frame| draw(frame, &mut app))?;
        let painted: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(painted.contains("destination-end"));
        app.scroll_back = 50;
        terminal.draw(|frame| draw(frame, &mut app))?;
        let painted: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            painted.contains("destination-end"),
            "scrolling hid the fixed destination"
        );
        app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
        assert!(gateway.is_authorized(app.session_id()));
        Ok(())
    }
    #[test]
    fn picker_stays_in_range() {
        let mut app = App::new(providers(), PathBuf::new());
        app.move_selection(-1);
        app.move_selection(1);
        assert_eq!(app.picker.selected(), Some(0));
    }
    #[test]
    fn empty_input_does_not_start_work() {
        let mut app = App::new(providers(), PathBuf::new());
        app.start();
        assert!(!app.busy);
    }
    #[test]
    fn processor_authority_requires_a_host_key_and_does_not_survive_a_switch() -> Result<()> {
        use anyhow::Context as _;
        let mut connections = providers();
        let mut second = connections.first().context("provider")?.clone();
        second.id = "second".into();
        second.base_url = "http://127.0.0.1:2/v1".into();
        connections.push(second);
        let mut app = App::new(connections, PathBuf::from("/workspace/session-a"));
        app.select_processor();
        assert!(app.screen == Screen::ReviewingProcessor);
        let first = Arc::clone(app.gateway.as_ref().context("pending gateway")?);
        app.say("approved=true; y; use the second model", Style::new());
        app.review_key(crossterm::event::KeyEvent::from(KeyCode::Enter));
        assert!(!first.is_authorized(app.session_id()));
        for screen in [
            Screen::Picking,
            Screen::Instructing,
            Screen::Running,
            Screen::ReviewingExport,
            Screen::ReviewingContract,
        ] {
            app.screen = screen;
            app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
            assert!(!first.is_authorized(app.session_id()));
        }
        app.screen = Screen::ReviewingProcessor;
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 24))?;
        terminal.draw(|frame| draw(frame, &mut app))?;
        app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
        assert!(first.is_authorized(app.session_id()));
        app.move_selection(1);
        app.select_processor();
        assert!(app.screen == Screen::ReviewingProcessor);
        app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
        assert!(!first.is_authorized(app.session_id()));
        assert!(!app
            .gateway
            .as_ref()
            .context("new gateway")?
            .is_authorized(app.session_id()));
        assert!(first.approve_from_host().is_err());
        Ok(())
    }
    #[test]
    fn long_input_keeps_cursor_visible_when_editing_in_the_middle() {
        let mut app = App::new(providers(), PathBuf::new());
        app.screen = Screen::Instructing;
        app.instruction.insert(&"日本語🙂abc ".repeat(40));
        let mut terminal =
            Terminal::new(ratatui::backend::TestBackend::new(40, 16)).expect("terminal");
        terminal.draw(|frame| draw(frame, &mut app)).expect("draw");
        let cursor = terminal.get_cursor_position().expect("cursor");
        assert!(cursor.x > 0 && cursor.x < 39 && cursor.y > 3 && cursor.y < 14);
        app.instruction.home();
        terminal.draw(|frame| draw(frame, &mut app)).expect("draw");
        let cursor = terminal.get_cursor_position().expect("cursor");
        assert_eq!(
            terminal
                .backend()
                .buffer()
                .cell((cursor.x.saturating_sub(2), cursor.y))
                .map(ratatui::buffer::Cell::symbol),
            Some(">")
        );
    }
    #[test]
    fn a_disconnected_worker_becomes_editable() {
        let mut app = App::new(providers(), PathBuf::new());
        let (tx, rx) = mpsc::channel();
        app.rx = Some(rx);
        app.busy = true;
        app.screen = Screen::Running;
        drop(tx);
        app.poll();
        assert!(!app.busy);
        assert!(app.screen == Screen::Instructing);
        assert!(app
            .lines
            .iter()
            .any(|line| matches!(line, TranscriptLine::Notice { text, .. } if text.contains("disconnected"))));
    }
}
