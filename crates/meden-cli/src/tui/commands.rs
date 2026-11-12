//! One catalog drives the slash popup and the host-only command dispatcher.
use super::{editor::Overlay, App, Screen, Speaker, TranscriptLine};
use ratatui::prelude::*;

pub(super) const COMMANDS: &[(&str, &str)] = &[
    (
        "/retry",
        "Continue the stopped task from current files and conversation",
    ),
    ("/model", "Choose a connection or set /model <name>"),
    (
        "/new",
        "Start a new chat; keep files and the visible transcript",
    ),
    ("/clear", "Clear the view and start a new chat; keep files"),
    (
        "/status",
        "Show the connection, workspace and last verification state",
    ),
    ("/copy", "Copy the latest completed response"),
    ("/help", "Show keyboard shortcuts and commands"),
    ("/keymap", "Show the current keyboard shortcuts"),
    ("/workspace", "Show the retained working directory"),
    ("/contract", "Review the project specification"),
    (
        "/proofs",
        "Inspect requirements, proof scope and retained evidence",
    ),
    ("/exports", "Review a pending delivery, or /exports <id>"),
    ("/quit", "Exit Meden"),
    ("/exit", "Exit Meden"),
];

pub(super) const SHORTCUTS: &str = "Keyboard shortcuts\n\nEnter                 Send message / run selected command\nShift+Enter / Ctrl+J   Insert newline\nTab                   Complete selected slash command\nCtrl+T                Expand / collapse transcript details\nCtrl+G                Inspect specification and evidence / return\nCtrl+O                Copy latest completed response\nCtrl+R                Search prompt history; repeat for older match\nCtrl+L                Clear the view; keep conversation context\nCtrl+C                Cancel work, clear draft, or exit when empty\nCtrl+D                Exit when empty; otherwise delete character\nEsc                   Close popup / cancel history search / stop work\n↑ / ↓, Ctrl+P / Ctrl+N Recall prompt history\nCtrl+A / Ctrl+E       Start / end of line\nCtrl+B / Ctrl+F       Move left / right\nAlt+B / Alt+F         Move one word\nCtrl+W / Ctrl+U / Ctrl+K Delete previous word / before cursor / after cursor\nCtrl+Y                Paste last deleted text\nWheel / trackpad       Scroll conversation or review details\nPgUp / PgDn           Scroll one viewport page\nShift+Up / Shift+Down Scroll one line\nCtrl+Home / Ctrl+End  Jump to beginning / end\n\nShift+Enter requires a terminal that reports modified Enter. Ctrl+J works in legacy terminals.\n\nEsc closes this help. Type / to browse commands.";

impl App {
    pub(super) fn matching_commands(&self) -> Vec<(&'static str, &'static str)> {
        let prefix = self.instruction.text();
        if !prefix.starts_with('/') || prefix.chars().any(char::is_whitespace) {
            return Vec::new();
        }
        COMMANDS
            .iter()
            .copied()
            .filter(|(name, _)| name.starts_with(prefix))
            .collect()
    }

    pub(super) fn refresh_commands(&mut self) {
        self.menu_index = 0;
        self.overlay = if self.matching_commands().is_empty() {
            Overlay::None
        } else {
            Overlay::Commands
        };
    }

    pub(super) fn complete_command(&mut self) {
        if let Some((name, _)) = self.matching_commands().get(self.menu_index).copied() {
            self.instruction.clear();
            self.instruction.insert(name);
        }
        self.overlay = Overlay::None;
    }

    pub(super) fn copy_latest(&mut self) {
        self.copy_request = self.last_stop.clone().or_else(|| {
            self.lines
                .iter()
                .skip(self.response_start)
                .rev()
                .find_map(|line| {
                    if let TranscriptLine::Message {
                        speaker: Speaker::Assistant,
                        text,
                    } = line
                    {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
        });
        let message = if self.copy_request.is_some() {
            "Latest response sent to the terminal clipboard."
        } else {
            "No completed response to copy."
        };
        self.say(message, Style::new().fg(Color::DarkGray));
    }

    fn new_chat(&mut self, clear_view: bool) {
        if self.workspace.is_some() {
            if let Err(error) = crate::session::history::clear(&self.root, clear_view) {
                self.say(
                    format!("Could not reset the saved conversation: {error}"),
                    Style::new().fg(Color::Red),
                );
                return;
            }
        }
        self.worker = None;
        self.rx = None;
        self.worker_gateway = None;
        self.instruction.clear();
        self.draft = None;
        self.reasoning = None;
        self.last_stop = None;
        self.reveals.clear();
        self.overlay = Overlay::None;
        self.scroll_back = 0;
        if clear_view {
            self.lines.clear();
            self.reveals.clear();
            self.view_start = 0;
        }
        self.response_start = self.lines.len();
        self.say(
            if clear_view {
                "Conversation cleared. New chat started; workspace files retained."
            } else {
                "New chat started. Workspace files retained."
            },
            Style::new().fg(Color::DarkGray),
        );
    }

    fn status(&mut self) {
        let connection = self.selected().map_or_else(
            || "No connection selected".into(),
            |p| format!("{} · {}", p.id, p.model),
        );
        let verification =
            self.project_status
                .as_ref()
                .map_or("No accepted project specification", |s| {
                    if s.verified {
                        "Last observed check: VERIFIED project kernel"
                    } else if s.accepted {
                        "Accepted specification; verification required"
                    } else {
                        "Specification has not been accepted"
                    }
                });
        self.say(format!("Connection: {connection}\nWorkspace: {}\nExecution: selected repository; offline tools\n{verification}", self.root.display()), Style::new());
    }

    fn retry_turn(&mut self) {
        if self.last_stop.is_none() {
            self.say(
                "No stopped task to retry. Enter your next instruction.",
                Style::new().fg(Color::Gray),
            );
            return;
        }
        if self.project.pending() {
            self.open_contract();
            return;
        }
        self.instruction.insert("Continue the previous interrupted request from the current repository and conversation. Inspect completed operations and current files first; do not blindly repeat commands or duplicate already applied changes. Diagnose and repair recoverable errors, rerun relevant checks, and preserve all host-accepted requirements. If a host service or resource is still unavailable, explain the remaining blocker and next step.");
        if let Some(TranscriptLine::Message { text, .. }) = self.lines.iter().rev().find(|line| {
            matches!(
                line,
                TranscriptLine::Message {
                    speaker: Speaker::User,
                    ..
                }
            )
        }) {
            self.instruction
                .insert(&format!("\nOriginal user request:\n{text}"));
        }
        self.say(
            "Continuing the stopped task. Inspecting existing results before further changes.",
            Style::new().fg(Color::Gray),
        );
        self.start_turn(false);
    }

    pub(super) fn host_command(&mut self) -> bool {
        let text = self.instruction.text().trim().to_owned();
        if !text.starts_with('/') {
            return false;
        }
        let mut parts = text.split_whitespace();
        let command = parts.next().unwrap_or_default();
        let arguments: Vec<_> = parts.collect();
        self.instruction.clear();
        self.overlay = Overlay::None;
        match (command, arguments.as_slice()) {
            ("/model", []) => self.screen = Screen::Picking,
            ("/model", [model]) => {
                if self.gateway.as_ref().is_some_and(|gateway| gateway.is_local()) {
                    self.say("Local weights are fixed for this session. Restart with --local-model to change them.", Style::new().fg(Color::Yellow));
                    return true;
                }
                if let Some(provider) = self.picker.selected().and_then(|i| self.providers.get_mut(i)) {
                    (*model).clone_into(&mut provider.model); self.select_processor();
                }
            }
            ("/new", []) => self.new_chat(false),
            ("/clear", []) => self.new_chat(true),
            ("/status", []) => self.status(),
            ("/retry", []) => self.retry_turn(),
            ("/copy", []) => self.copy_latest(),
            ("/help" | "/keymap", []) => { self.overlay = Overlay::Help; self.scroll_back = 0; }
            ("/quit" | "/exit", []) => self.quit_requested = true,
            ("/workspace", []) => self.say(format!("Project: {}\nFile and Bash changes are saved directly here. Reopening this directory resumes the conversation.", self.root.display()), Style::new()),
            ("/contract", []) => self.open_contract(),
            ("/proofs", []) => { self.refresh_project_status(); self.overlay = Overlay::Evidence; self.scroll_back = 0; }
            ("/exports", []) => self.open_export(None),
            ("/exports", [id]) => match id.parse() {
                Ok(id) => self.open_export(Some(id)),
                Err(_) => self.say("Use /exports or /exports ID.", Style::new().fg(Color::Red)),
            },
            _ => self.say("Unknown command or arguments. Type / to see available commands.", Style::new().fg(Color::Yellow)),
        }
        true
    }
}
