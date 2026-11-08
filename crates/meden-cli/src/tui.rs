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

mod input;
use input::Input;

enum Update {
    Asking(usize),
    Text(String),
    Tool(String, bool),
    Done(String),
}
#[derive(PartialEq, Eq)]
enum Screen {
    Picking,
    ReviewingProcessor,
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
    instruction: Input,
    screen: Screen,
    lines: Vec<TranscriptLine>,
    busy: bool,
    rx: Option<Receiver<Update>>,
    worker: Option<Sender<String>>,
    worker_gateway: Option<Arc<Gateway>>,
    processes: Arc<Control>,
    scroll_back: u16,
}

impl App {
    fn new(providers: Vec<Provider>, root: PathBuf) -> Self {
        let mut picker = ListState::default();
        if !providers.is_empty() {
            picker.select(Some(0));
        }
        Self {
            providers,
            picker,
            root,
            workspace: None,
            gateway: None,
            instruction: Input::default(),
            screen: Screen::Picking,
            lines: Vec::new(),
            busy: false,
            rx: None,
            worker: None,
            worker_gateway: None,
            processes: Arc::new(Control::new(Arc::new(AtomicBool::new(false)))),
            scroll_back: 0,
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
            KeyCode::Char('y') if key.modifiers.is_empty() => self.approve_processor(),
            KeyCode::Char('n') | KeyCode::Esc => self.screen = Screen::Picking,
            _ => {}
        }
    }
    fn start(&mut self) {
        if self.busy || self.instruction.text().trim().is_empty() {
            return;
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
            self.worker = Some(commands);
            self.rx = Some(results);
            self.worker_gateway = Some(Arc::clone(&gateway));
            self.lines.clear();
            thread::spawn(move || {
                worker(&gateway, &workspace, &tasks, &updates, &processes);
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
            self.processes.cancel();
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
                Update::Asking(round) => self.say(
                    format!("round {round}: thinking…"),
                    Style::new().fg(Color::DarkGray),
                ),
                Update::Text(text) => self.say(text, Style::new()),
                Update::Tool(text, error) => self.say(
                    text,
                    Style::new().fg(if error { Color::Red } else { Color::Green }),
                ),
                Update::Done(text) => {
                    self.busy = false;
                    self.screen = Screen::Instructing;
                    self.say(text, Style::new().fg(Color::DarkGray));
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
        self.processes.cancel();
        if let Some(gateway) = &self.gateway {
            let _ = gateway.revoke_from_host();
        }
    }
}

fn worker(
    gateway: &Arc<Gateway>,
    workspace: &Arc<Workspace>,
    tasks: &Receiver<String>,
    tx: &Sender<Update>,
    processes: &Arc<Control>,
) {
    let mut session = match crate::session::Session::new(
        Arc::clone(workspace),
        Arc::clone(gateway),
        Arc::clone(processes),
    ) {
        Ok(session) => session,
        Err(error) => {
            let _ = tx.send(Update::Done(error.to_string()));
            return;
        }
    };
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
        let _ = tx.send(update);
    };
    while let Ok(instruction) = tasks.recv() {
        let status = session.run(&instruction, &observer);
        let summary = match status {
            Status::Done(_) => {
                "Finished. Conversation retained for the next instruction.".to_owned()
            }
            Status::Failed(error) => format!("Failed: {error}"),
            Status::Incomplete => "Unfinished: tool execution limit reached.".to_owned(),
            Status::Cancelled => "Cancelled. Conversation retained.".to_owned(),
        };
        if tx.send(Update::Done(summary)).is_err() {
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
pub fn run(providers: Vec<Provider>, root: &Path, approved: Option<&str>) -> Result<()> {
    anyhow::ensure!(
        !providers.is_empty(),
        "TUI requires an OpenAI-compatible HTTP provider; see `meden providers`"
    );
    let workspace = Arc::new(Workspace::import(root, &std::env::current_exe()?)?);
    let mut app = App::new(providers, workspace.path().to_owned());
    app.workspace = Some(workspace);
    if let Some(endpoint) = approved {
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
                    app.processes.cancel();
                    return Ok(());
                }
                app.cancel();
                continue;
            }
            return Ok(());
        }
        match key.code {
            KeyCode::PageUp => {
                app.scroll_back = if app.screen == Screen::ReviewingProcessor {
                    app.scroll_back.saturating_sub(10)
                } else {
                    app.scroll_back.saturating_add(10)
                };
                continue;
            }
            KeyCode::PageDown => {
                app.scroll_back = if app.screen == Screen::ReviewingProcessor {
                    app.scroll_back.saturating_add(10)
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
            Screen::Instructing => match key.code {
                KeyCode::Esc | KeyCode::Tab => app.screen = Screen::Picking,
                KeyCode::Enter if key.modifiers.contains(KeyModifiers::ALT) => {
                    app.instruction.insert("\n");
                }
                KeyCode::Enter => app.start(),
                KeyCode::Backspace => {
                    app.instruction.backspace();
                }
                KeyCode::Delete => app.instruction.delete(),
                KeyCode::Left => app.instruction.left(),
                KeyCode::Right => app.instruction.right(),
                KeyCode::Home => app.instruction.home(),
                KeyCode::End => app.instruction.end(),
                KeyCode::Up => app.instruction.up(),
                KeyCode::Down => app.instruction.down(),
                KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.instruction.home();
                }
                KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.instruction.end();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.instruction.clear();
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.instruction.insert(&c.to_string());
                }
                _ => {}
            },
            Screen::Running => {
                if key.code == KeyCode::Esc {
                    app.cancel();
                }
            }
        }
    }
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    frame.render_widget(Block::default().bg(Color::Rgb(12, 18, 30)), frame.area());
    let areas = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(3),
        Constraint::Length(5),
    ])
    .split(frame.area());
    let top = areas.first().copied().unwrap_or_default();
    let middle = areas.get(1).copied().unwrap_or_default();
    let bottom = areas.get(2).copied().unwrap_or_default();
    let model = app.selected().map_or("no model", |p| p.model.as_str());
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(visible(&format!(
                "MEDEN · private workspace · offline tools · {model}"
            ))),
            Line::from(visible(&format!("files: {}", app.root.display()))),
        ])
        .style(Style::new().fg(Color::Rgb(104, 218, 219)))
        .block(Block::bordered().border_style(Style::new().fg(Color::Rgb(53, 75, 96)))),
        top,
    );
    let welcome =
        app.screen == Screen::Picking || app.screen == Screen::Instructing && app.lines.is_empty();
    let middle = if welcome && middle.width >= 72 && middle.height >= 15 {
        draw_welcome(frame, middle)
    } else {
        middle
    };
    if app.screen == Screen::ReviewingProcessor {
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
    let (title, text) = match app.screen {
        Screen::Picking => (
            " controls ",
            "↑↓ choose · enter: continue · q: quit".to_owned(),
        ),
        Screen::ReviewingProcessor => (
            " processor approval ",
            "y: authorize this processor · n / esc: back · ctrl-c: quit".to_owned(),
        ),
        Screen::Instructing => return,
        Screen::Running => (
            " working ",
            "esc / ctrl-c: cancel after current request".to_owned(),
        ),
    };
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(title)),
        bottom,
    );
}

fn draw_processor_review(frame: &mut Frame<'_>, app: &App, middle: Rect) {
    if let Some(gateway) = &app.gateway {
        let provider = gateway.provider();
        let text = format!(
            "Destination: {}\nConnection: {}\nModel: {}\nCredential: {}\nConfiguration: {}\n\nAllow this processor to receive this workspace's conversation, file contents and tool results for this session, up to eight hours.\n\nLoopback alone does not guarantee local inference; the server may forward data.\n\nOnly y grants approval. Enter and pasted text do not. PgUp/PgDn: review.",
            gateway.endpoint(),
            visible(&provider.id).replace('\n', "⏎"),
            visible(&provider.model).replace('\n', "⏎"),
            if provider.api_key.is_some() { "host-configured account" } else { "none" },
            visible(&provider.note).replace('\n', "⏎"),
        );
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .scroll((app.scroll_back, 0))
                .block(Block::bordered().title(" processor approval · host control ")),
            middle,
        );
    }
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
        Screen::Instructing,
        Screen::Running,
    ] {
        app.screen = screen;
        app.say("a line of transcript", Style::new());
        terminal.draw(|frame| draw(frame, &app))?;
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
        app.review_key(crossterm::event::KeyEvent::from(KeyCode::Char('y')));
        assert!(first.is_authorized(app.session_id()));
        app.move_selection(1);
        app.select_processor();
        assert!(app.screen == Screen::ReviewingProcessor);
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
        terminal.draw(|frame| draw(frame, &app)).expect("draw");
        let cursor = terminal.get_cursor_position().expect("cursor");
        assert!(cursor.x > 0 && cursor.x < 39 && cursor.y >= 12 && cursor.y < 15);
        app.instruction.home();
        terminal.draw(|frame| draw(frame, &app)).expect("draw");
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
