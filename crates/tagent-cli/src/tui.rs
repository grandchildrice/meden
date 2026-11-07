//! `tagent tui`: pick a model, give it an instruction, watch the gate decide.
//!
//! The screen exists to make one thing visible: which calls ran, which were
//! refused, and why. A refusal is the product, so it is what the layout gives
//! room to.
//!
//! The tools are the demonstration ones from [`crate::demo`] — a `fetch` that
//! returns an injected page and a `send_email` that goes nowhere. The screen
//! says so, because an interface that looks like it is sending real mail
//! while sending none would be worse than no interface.

use std::fmt::Write as _;
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use tagent_agent::planner::{Outcome, Quarantined, Session, SessionEvent};
use tagent_agent::{GateConfig, Refusal};
use tagent_llm::Provider;

/// What the worker thread reports back.
enum Update {
    Asking(usize),
    Call {
        attempt: usize,
        tool: String,
        refusal: Option<Refusal>,
    },
    Done(String),
}

/// Which screen is showing.
#[derive(PartialEq, Eq)]
enum Screen {
    /// Choosing a model.
    Picking,
    /// Typing an instruction.
    Instructing,
    /// Watching it run.
    Running,
}

/// One line in the transcript.
struct Line {
    text: String,
    style: Style,
}

struct App {
    providers: Vec<Provider>,
    picker: ListState,
    instruction: String,
    screen: Screen,
    lines: Vec<Line>,
    busy: bool,
    rx: Option<Receiver<Update>>,
}

impl App {
    fn new(providers: Vec<Provider>) -> Self {
        let mut picker = ListState::default();
        if !providers.is_empty() {
            picker.select(Some(0));
        }
        App {
            providers,
            picker,
            instruction: crate::demo::INSTRUCTION.to_owned(),
            screen: Screen::Picking,
            lines: Vec::new(),
            busy: false,
            rx: None,
        }
    }

    fn selected(&self) -> Option<&Provider> {
        self.providers.get(self.picker.selected().unwrap_or(0))
    }

    fn say(&mut self, text: impl Into<String>, style: Style) {
        self.lines.push(Line {
            text: text.into(),
            style,
        });
    }

    fn move_selection(&mut self, delta: isize) {
        if self.providers.is_empty() {
            return;
        }
        let current = self.picker.selected().unwrap_or(0);
        let last = self.providers.len().saturating_sub(1);
        let next = if delta < 0 {
            current.saturating_sub(1)
        } else {
            current.saturating_add(1).min(last)
        };
        self.picker.select(Some(next));
    }

    /// Start a run on a worker thread so the screen stays responsive.
    fn start(&mut self) {
        let Some(provider) = self.selected().cloned() else {
            return;
        };
        let instruction = self.instruction.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.busy = true;
        self.lines.clear();
        self.screen = Screen::Running;
        self.say(
            format!("planning with {} ({})", provider.label, provider.model),
            Style::new().fg(Color::Cyan),
        );
        self.say(
            "the tools are demonstration ones: fetch returns an injected page, \
             send_email sends nothing",
            Style::new().fg(Color::DarkGray),
        );
        thread::spawn(move || run_session(&provider, &instruction, &tx));
    }

    /// Drain anything the worker has reported.
    fn poll(&mut self) {
        let mut updates = Vec::new();
        if let Some(rx) = &self.rx {
            while let Ok(u) = rx.try_recv() {
                updates.push(u);
            }
        }
        for u in updates {
            match u {
                Update::Asking(n) => self.say(
                    format!("attempt {n}: asking the planner…"),
                    Style::new().fg(Color::Yellow),
                ),
                Update::Call {
                    attempt,
                    tool,
                    refusal: None,
                } => self.say(
                    format!("  attempt {attempt}: {tool} allowed"),
                    Style::new().fg(Color::Green),
                ),
                Update::Call {
                    attempt,
                    tool,
                    refusal: Some(r),
                } => {
                    self.say(
                        format!("  attempt {attempt}: {tool} REFUSED"),
                        Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
                    );
                    self.say(format!("    {r}"), Style::new().fg(Color::Red));
                    self.say(
                        format!("    the planner is told: {}", r.safe_summary()),
                        Style::new().fg(Color::DarkGray),
                    );
                }
                Update::Done(summary) => {
                    self.busy = false;
                    self.say("", Style::new());
                    self.say(summary, Style::new().add_modifier(Modifier::BOLD));
                    self.say(
                        "enter to run again, tab to change model, q to quit",
                        Style::new().fg(Color::DarkGray),
                    );
                }
            }
        }
    }
}

/// Run one session, reporting as it goes.
fn run_session(provider: &Provider, instruction: &str, tx: &Sender<Update>) {
    let registry = crate::demo::registry();
    let config = GateConfig::new(crate::demo::policy());
    let reader: &dyn Quarantined = provider;

    let observer = |event: SessionEvent<'_>| match event {
        SessionEvent::Asking { attempt } => {
            let _ = tx.send(Update::Asking(attempt));
        }
        SessionEvent::Attempted { attempt, trace, .. } => {
            for call in &trace.calls {
                let _ = tx.send(Update::Call {
                    attempt,
                    tool: call.tool.clone(),
                    refusal: call.outcome.as_ref().err().cloned(),
                });
            }
        }
    };

    let report = Session::new(&registry, &config, provider)
        .with_quarantined(reader)
        .with_observer(&observer)
        .run(instruction);

    let refused_for_provenance = report
        .refusals()
        .iter()
        .any(|r| matches!(r, Refusal::UntrustedControl { .. }));

    let summary = match &report.outcome {
        Outcome::Done(_) if refused_for_provenance => {
            "the planner took the page's address, the runtime refused it, and the corrected \
             plan ran."
                .to_owned()
        }
        Outcome::Done(_) => {
            "this planner was not taken in, so the runtime had nothing to refuse.".to_owned()
        }
        Outcome::Refused(r) => format!("every attempt was refused: {r}"),
        Outcome::Failed(e) => format!("the plan failed: {e}"),
        Outcome::NoPlan(e) => format!("no plan: {e}"),
    };
    let _ = tx.send(Update::Done(summary));
}

/// Show the interface.
pub fn run(providers: Vec<Provider>) -> Result<()> {
    if providers.is_empty() {
        anyhow::bail!("no model found; try `tagent providers`");
    }
    let mut app = App::new(providers);

    enable_raw_mode()?;
    let mut out = io::stdout();
    out.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop<B>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B: Backend,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    loop {
        app.poll();
        terminal.draw(|f| draw(f, app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }
        let event::Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(());
        }

        match app.screen {
            Screen::Picking => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
                KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
                KeyCode::Enter => app.screen = Screen::Instructing,
                _ => {}
            },
            Screen::Instructing => match key.code {
                KeyCode::Esc => app.screen = Screen::Picking,
                KeyCode::Enter => app.start(),
                KeyCode::Backspace => {
                    app.instruction.pop();
                }
                KeyCode::Char(c) => app.instruction.push(c),
                _ => {}
            },
            Screen::Running => match key.code {
                KeyCode::Char('q') | KeyCode::Esc if !app.busy => return Ok(()),
                KeyCode::Tab if !app.busy => app.screen = Screen::Picking,
                KeyCode::Enter if !app.busy => app.start(),
                _ => {}
            },
        }
    }
}

fn draw(frame: &mut Frame<'_>, app: &App) {
    let areas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(3),
    ])
    .split(frame.area());
    let (top, middle, bottom) = (
        areas.first().copied().unwrap_or_default(),
        areas.get(1).copied().unwrap_or_default(),
        areas.get(2).copied().unwrap_or_default(),
    );

    let title = Paragraph::new("T Agent — proof-driven development for agent actions")
        .style(Style::new().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, top);

    match app.screen {
        Screen::Picking => {
            let items: Vec<ListItem<'_>> = app
                .providers
                .iter()
                .map(|p| ListItem::new(format!("{:<24} {}  — {}", p.label, p.model, p.note)))
                .collect();
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" models found on this machine "),
                )
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");
            let mut state = app.picker;
            frame.render_stateful_widget(list, middle, &mut state);
        }
        Screen::Instructing | Screen::Running => {
            let text: Vec<ratatui::text::Line<'_>> = app
                .lines
                .iter()
                .map(|l| ratatui::text::Line::styled(l.text.clone(), l.style))
                .collect();
            let body = Paragraph::new(text).wrap(Wrap { trim: false }).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" what the gate decided "),
            );
            frame.render_widget(body, middle);
        }
    }

    let footer = match app.screen {
        Screen::Picking => Paragraph::new("↑↓ choose   enter: continue   q: quit")
            .block(Block::default().borders(Borders::ALL)),
        Screen::Instructing => Paragraph::new(format!("> {}", app.instruction)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" instruction (enter to run, esc to go back) "),
        ),
        Screen::Running if app.busy => {
            Paragraph::new("working…").block(Block::default().borders(Borders::ALL))
        }
        Screen::Running => Paragraph::new("enter: run again   tab: change model   q: quit")
            .block(Block::default().borders(Borders::ALL)),
    };
    frame.render_widget(footer, bottom);
}

/// The environment variable that makes the interface run once and exit,
/// so it can be smoke-tested without a terminal.
pub const ENV_SELFTEST: &str = "TAGENT_TUI_SELFTEST";

/// Draw every screen once into a fixed-size buffer, proving the layout code
/// runs without a terminal attached.
pub fn selftest(providers: Vec<Provider>) -> Result<String> {
    use ratatui::backend::TestBackend;
    let mut app = App::new(providers);
    let mut terminal = Terminal::new(TestBackend::new(100, 24))?;
    let mut rendered = String::new();
    for screen in [Screen::Picking, Screen::Instructing, Screen::Running] {
        app.screen = screen;
        app.say("a line of transcript", Style::new());
        terminal.draw(|f| draw(f, &app))?;
        let _ = writeln!(rendered, "{:?}", terminal.backend().buffer().area);
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_providers() -> Vec<Provider> {
        vec![Provider {
            id: "test".to_owned(),
            label: "Test".to_owned(),
            model: "m".to_owned(),
            transport: tagent_llm::Transport::OpenAi {
                base_url: "http://127.0.0.1:1/v1".to_owned(),
                api_key: None,
            },
            note: "for tests".to_owned(),
        }]
    }

    #[test]
    fn every_screen_draws() {
        assert!(selftest(fake_providers()).is_ok());
    }

    #[test]
    fn the_picker_stays_in_range() {
        let mut app = App::new(fake_providers());
        app.move_selection(-1);
        assert_eq!(app.picker.selected(), Some(0));
        app.move_selection(1);
        assert_eq!(app.picker.selected(), Some(0), "only one provider");
        assert!(app.selected().is_some());
    }

    #[test]
    fn an_empty_picker_does_not_panic() {
        let mut app = App::new(Vec::new());
        app.move_selection(1);
        assert_eq!(app.selected().map(|p| p.id.clone()), None);
    }
}
