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
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

mod approval;
mod input;
use input::Input;

enum Update {
    Project(Box<crate::project::Summary>),
    ProjectDirty,
    Asking(usize),
    Text(String),
    Tool(String, bool),
    Done(String, bool),
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
struct TranscriptLine {
    text: String,
    style: Style,
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
        self.lines.push(TranscriptLine {
            text: visible(&text.into()),
            style,
        });
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
                                self.start();
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
                        let _ = updates.send(Update::Done(error.to_string(), false));
                    }
                }
            });
        }
        self.processes.flag().store(false, Ordering::SeqCst);
        let instruction = self.instruction.take();
        self.say(format!("> {instruction}"), Style::new().fg(Color::Cyan));
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
                Update::Asking(round) => self.say(
                    format!("round {round}: thinking…"),
                    Style::new().fg(Color::DarkGray),
                ),
                Update::Text(text) => self.say(text, Style::new()),
                Update::Tool(text, error) => self.say(
                    text,
                    Style::new().fg(if error { Color::Red } else { Color::Green }),
                ),
                Update::Done(text, needs_review) => {
                    self.busy = false;
                    self.screen = Screen::Instructing;
                    self.say(text, Style::new().fg(Color::DarkGray));
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
            Event::Decision { call, refusal } => Update::Tool(
                match refusal {
                    Some((_, reason)) => format!("{} REFUSED: {reason}", call.function.name),
                    None => format!("{} {}", call.function.name, call.function.arguments),
                },
                refusal.is_some(),
            ),
            Event::ToolResult { call, text, error } => {
                let excerpt: String = text.chars().take(1200).collect();
                Update::Tool(
                    format!(
                        "{}{}: {excerpt}{}",
                        call.function.name,
                        if error { " error" } else { " result" },
                        if excerpt.len() < text.len() {
                            "…"
                        } else {
                            ""
                        }
                    ),
                    error,
                )
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
        if tx.send(Update::Done(summary, needs_review)).is_err() {
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

fn draw(frame: &mut Frame<'_>, app: &mut App) {
    app.review_area = Rect::default();
    app.review_frame = frame.area();
    frame.render_widget(Block::default().bg(Color::Rgb(12, 18, 30)), frame.area());
    let areas = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(3),
        Constraint::Length(5),
    ])
    .split(frame.area());
    let top = areas.first().copied().unwrap_or_default();
    let middle = areas.get(1).copied().unwrap_or_default();
    let bottom = areas.get(2).copied().unwrap_or_default();
    draw_header(frame, app, top);
    let welcome =
        app.screen == Screen::Picking || app.screen == Screen::Instructing && app.lines.is_empty();
    let middle = if welcome && middle.width >= 72 && middle.height >= 15 {
        draw_welcome(frame, middle)
    } else {
        middle
    };
    if app.screen == Screen::ReviewingContract {
        draw_contract_review(frame, app, middle);
    } else if app.screen == Screen::ReviewingExport {
        draw_export_review(frame, app, middle);
    } else if app.screen == Screen::ReviewingProcessor {
        draw_processor_review(frame, app, middle);
    } else if app.screen == Screen::Picking {
        let items: Vec<_> = app
            .providers
            .iter()
            .map(|p| ListItem::new(visible(&format!("{} · {} · {}", p.label, p.model, p.note))))
            .collect();
        let mut state = app.picker;
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(" model "))
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> "),
            middle,
            &mut state,
        );
    } else {
        let text: Vec<_> = app
            .lines
            .iter()
            .flat_map(|line| {
                line.text
                    .split('\n')
                    .map(|part| ratatui::text::Line::styled(part.to_owned(), line.style))
            })
            .collect();
        let body = Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(Block::bordered().title(" conversation · PgUp/PgDn to scroll "));
        let rows =
            u16::try_from(body.line_count(middle.width.saturating_sub(2))).unwrap_or(u16::MAX);
        let scroll = rows
            .saturating_sub(middle.height.saturating_sub(2))
            .saturating_sub(app.scroll_back);
        frame.render_widget(body.scroll((scroll, 0)), middle);
    }
    if app.screen == Screen::Instructing {
        draw_input(frame, &app.instruction, bottom);
        return;
    }
    draw_controls(frame, app, bottom);
}

fn draw_controls(frame: &mut Frame<'_>, app: &App, bottom: Rect) {
    let (title, text) = match app.screen {
        Screen::Picking => (
            " controls ",
            "↑↓ choose · enter: continue · q: quit".to_owned(),
        ),
        Screen::ReviewingProcessor => (
            " processor approval ",
            "y: authorize this processor · n / esc: back · PgUp/PgDn: details".to_owned(),
        ),
        Screen::ReviewingExport => (
            " exact export · host control ",
            "y: send once · n: reject · esc: back · ←/→: bytes · PgUp/PgDn: scroll".to_owned(),
        ),
        Screen::ReviewingContract => (
            " contract review · host control ",
            "y: accept and continue · n / esc: reject proposal · PgUp/PgDn: scroll".to_owned(),
        ),
        Screen::Instructing => return,
        Screen::Running => (
            " working ",
            "esc / ctrl-c: cancel after current request".to_owned(),
        ),
    };
    let text = if matches!(
        app.screen,
        Screen::ReviewingProcessor | Screen::ReviewingExport | Screen::ReviewingContract
    ) && (app.review_area.width <= 3
        || app.review_rows == 0
        || app.review_rows > app.review_area.height.saturating_sub(2))
    {
        "Approval unavailable: enlarge the terminal to show the complete heading. · n / esc: back"
            .to_owned()
    } else {
        text
    };
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(title)),
        bottom,
    );
}

fn draw_header(frame: &mut Frame<'_>, app: &App, top: Rect) {
    let model = app.selected().map_or("no model", |p| p.model.as_str());
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(visible(&format!(
                "MEDEN · private workspace · offline tools · {model}"
            ))),
            Line::from(visible(&format!("files: {}", app.root.display()))),
            Line::from(match &app.project_status {
                Some(s) if s.verified => format!(
                    "VERIFIED project kernel · source {}",
                    s.source_sha256
                        .as_deref()
                        .unwrap_or_default()
                        .chars()
                        .take(12)
                        .collect::<String>()
                ),
                Some(s) if s.accepted => "Project kernel: current source is not verified".into(),
                Some(s) if s.proposed => "Project contract awaits host acceptance".into(),
                _ => "Verification: no accepted project contract".into(),
            }),
        ])
        .style(Style::new().fg(Color::Rgb(104, 218, 219)))
        .block(Block::bordered().border_style(Style::new().fg(Color::Rgb(53, 75, 96)))),
        top,
    );
}

/// Keep the authority heading fixed while scrolling the explanatory text.
/// The same Paragraph supplies its row count and the actual painted heading.
fn draw_review(
    frame: &mut Frame<'_>,
    app: &mut App,
    middle: Rect,
    title: &str,
    heading: &str,
    details: &str,
) {
    let heading = Paragraph::new(visible(heading)).wrap(Wrap { trim: false });
    app.review_rows =
        u16::try_from(heading.line_count(middle.width.saturating_sub(2))).unwrap_or(u16::MAX);
    let areas = Layout::vertical([
        Constraint::Length(app.review_rows.saturating_add(2)),
        Constraint::Min(3),
    ])
    .split(middle);
    let header = areas.first().copied().unwrap_or_default();
    let body = areas.get(1).copied().unwrap_or_default();
    app.review_area = header;
    app.review_page = body.height.saturating_sub(2);
    frame.render_widget(
        heading.block(Block::bordered().title(title.to_owned())),
        header,
    );
    frame.render_widget(
        Paragraph::new(visible(details))
            .wrap(Wrap { trim: false })
            .scroll((app.scroll_back, 0))
            .block(Block::bordered().title(" details · PgUp/PgDn ")),
        body,
    );
}

fn draw_contract_review(frame: &mut Frame<'_>, app: &mut App, middle: Rect) {
    let Some(review) = &app.contract_review else {
        return;
    };
    let heading = format!(
        "Contract revision: {}\nContract SHA-256: {}",
        review.reference.revision, review.reference.contract
    );
    let details = format!(
        "{}\n\nAccepting continues development under this exact contract.",
        review.review
    );
    draw_review(
        frame,
        app,
        middle,
        " Rust transition contract · host control ",
        &heading,
        &details,
    );
}

fn draw_processor_review(frame: &mut Frame<'_>, app: &mut App, middle: Rect) {
    let Some(gateway) = &app.gateway else {
        return;
    };
    let provider = gateway.provider();
    let heading = format!(
        "Destination: {}\nConnection: {}\nModel: {}\nCredential: {}\nScope: workspace conversation, files and tool results; one session, up to eight hours.",
        gateway.endpoint(),
        visible(&provider.id).replace('\n', "⏎"),
        visible(&provider.model).replace('\n', "⏎"),
        if provider.api_key.is_some() {
            "host-configured account"
        } else {
            "none"
        }
    );
    let details = format!("Configuration: {}\n\nAllow this processor to receive this workspace's conversation, file contents and tool results for this session, up to eight hours.\n\nLoopback alone does not guarantee local inference; the server may forward data.\n\nOnly y grants approval. Enter and pasted text do not.",
        visible(&provider.note).replace('\n', "⏎"));
    draw_review(
        frame,
        app,
        middle,
        " processor approval · host control ",
        &heading,
        &details,
    );
}

fn draw_export_review(frame: &mut Frame<'_>, app: &mut App, middle: Rect) {
    let Some(entry) = app
        .reviewing_export
        .as_ref()
        .and_then(|reference| app.exports.get(reference).ok())
    else {
        return;
    };
    let Ok(summary) = entry.summary() else {
        return;
    };
    let Ok(preview) = entry.preview(app.export_offset) else {
        return;
    };
    let heading = format!("Export #{}\nDestination: {}\nMethod: POST · Content-Type: application/octet-stream\nFrozen body: {} bytes\nAccepted contract: {}\nAttempt: one POST; no redirects or automatic retries",
        summary.reference.id, summary.target, summary.bytes,
        summary.contract.as_deref().unwrap_or("ordinary file; no project contract"));
    let details = format!("State: {}\nSource name (untrusted): {}\nByte offset: {}\n\n{}\nThese exact payload bytes are retained outside the worker. Current project source and evidence are checked again before dispatch. No redirects or automatic retries. Approval is for one attempt, including uncertain delivery.",
        summary.state, visible(&summary.source).replace('\n', "⏎"), app.export_offset, preview);
    draw_review(
        frame,
        app,
        middle,
        " immutable export review · host control ",
        &heading,
        &details,
    );
}

fn draw_welcome(frame: &mut Frame<'_>, middle: Rect) -> Rect {
    let welcome_areas =
        Layout::vertical([Constraint::Length(10), Constraint::Min(3)]).split(middle);
    let logo = vec![
        Line::from("███╗   ███╗███████╗██████╗ ███████╗███╗   ██╗"),
        Line::from("████╗ ████║██╔════╝██╔══██╗██╔════╝████╗  ██║"),
        Line::from("██╔████╔██║█████╗  ██║  ██║█████╗  ██╔██╗ ██║"),
        Line::from("██║╚██╔╝██║██╔══╝  ██║  ██║██╔══╝  ██║╚██╗██║"),
        Line::from("██║ ╚═╝ ██║███████╗██████╔╝███████╗██║ ╚████║"),
        Line::from("╚═╝     ╚═╝╚══════╝╚═════╝ ╚══════╝╚═╝  ╚═══╝"),
        Line::from(""),
        Line::from("Read. Edit. Build. Test."),
        Line::from("Edits stay in this copy; the original project is unchanged."),
    ];
    frame.render_widget(
        Paragraph::new(logo)
            .centered()
            .style(Style::new().fg(Color::Rgb(104, 218, 219))),
        welcome_areas.first().copied().unwrap_or_default(),
    );
    welcome_areas.get(1).copied().unwrap_or(middle)
}

fn draw_input(frame: &mut Frame<'_>, input: &Input, bottom: Rect) {
    let block = Block::bordered().title(
        " instruction · enter: send · alt-enter: newline · ↑↓ history · /clear · tab: model ",
    );
    let inner = block.inner(bottom);
    frame.render_widget(block, bottom);
    if inner.width >= 4 && inner.height > 0 {
        let view = input.view(usize::from(inner.width));
        let scroll = view
            .row
            .saturating_sub(usize::from(inner.height).saturating_sub(1));
        let lines: Vec<_> = view
            .lines
            .into_iter()
            .skip(scroll)
            .take(usize::from(inner.height))
            .map(Line::raw)
            .collect();
        frame.render_widget(Paragraph::new(lines), inner);
        frame.set_cursor_position((
            inner
                .x
                .saturating_add(u16::try_from(view.column).unwrap_or(0)),
            inner
                .y
                .saturating_add(u16::try_from(view.row.saturating_sub(scroll)).unwrap_or(0)),
        ));
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
            .all(|line| !line.text.chars().any(|c| c.is_control() && c != '\n')));
        assert!(app
            .lines
            .first()
            .is_some_and(|line| line.text.starts_with("日本語🙂\n")));
    }
    fn providers() -> Vec<Provider> {
        vec![Provider {
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
        assert!(cursor.x > 0 && cursor.x < 39 && cursor.y >= 12 && cursor.y < 15);
        app.instruction.home();
        terminal.draw(|frame| draw(frame, &mut app)).expect("draw");
        assert_eq!(
            terminal.get_cursor_position().expect("cursor"),
            Position::new(3, 12)
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
            .any(|line| line.text.contains("disconnected")));
    }
}
