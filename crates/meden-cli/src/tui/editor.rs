//! Composer editing and menus. Approval screens never call this handler.
use super::{App, KeyCode, KeyModifiers, Ordering, Screen};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Overlay {
    None,
    Commands,
    Help,
    Evidence,
}

impl App {
    pub(super) fn menu_open(&self) -> bool {
        self.overlay == Overlay::Commands
    }
    pub(super) fn help_open(&self) -> bool {
        self.overlay == Overlay::Help
    }
    pub(super) fn evidence_open(&self) -> bool {
        self.overlay == Overlay::Evidence
    }

    pub(super) fn global_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        if matches!(self.screen, Screen::Instructing | Screen::Running) {
            if key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::CONTROL {
                if !self.busy && !self.evidence_open() {
                    self.refresh_project_status();
                }
                self.overlay = if self.evidence_open() {
                    Overlay::None
                } else {
                    Overlay::Evidence
                };
                self.scroll_back = 0;
                return true;
            }
            if self.evidence_open() && key.code == KeyCode::Esc {
                self.overlay = Overlay::None;
                self.scroll_back = 0;
                return true;
            }
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.busy {
                if self.processes.flag().load(Ordering::SeqCst) {
                    self.project.cancel(&self.processes);
                    self.quit_requested = true;
                    return true;
                }
                self.cancel();
                return true;
            }
            if self.screen == Screen::Instructing && self.clear_editor() {
                return true;
            }
            self.quit_requested = true;
            return true;
        }
        if key.code == KeyCode::Char('t')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(self.screen, Screen::Instructing | Screen::Running)
        {
            self.show_tool_details = !self.show_tool_details;
            return true;
        }
        if key.code == KeyCode::Char('o')
            && key.modifiers == KeyModifiers::CONTROL
            && matches!(self.screen, Screen::Instructing | Screen::Running)
        {
            self.copy_latest();
            return true;
        }
        if key.code == KeyCode::Char('d')
            && key.modifiers == KeyModifiers::CONTROL
            && self.instruction.text().is_empty()
        {
            self.project.cancel(&self.processes);
            self.quit_requested = true;
            return true;
        }
        if key.code == KeyCode::Char('l')
            && key.modifiers == KeyModifiers::CONTROL
            && self.screen == Screen::Instructing
        {
            self.view_start = self.lines.len();
            self.scroll_back = 0;
            self.overlay = Overlay::None;
            return true;
        }
        false
    }

    pub(super) fn scroll_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::PageUp => {
                self.scroll_back = if self.help_open()
                    || self.evidence_open()
                    || matches!(
                        self.screen,
                        Screen::ReviewingProcessor
                            | Screen::ReviewingExport
                            | Screen::ReviewingContract
                    ) {
                    self.scroll_back
                        .saturating_sub(if self.help_open() || self.evidence_open() {
                            10
                        } else {
                            self.review_page.max(1)
                        })
                } else {
                    self.scroll_back.saturating_add(10)
                };
                return true;
            }
            KeyCode::PageDown => {
                self.scroll_back = if self.help_open()
                    || self.evidence_open()
                    || matches!(
                        self.screen,
                        Screen::ReviewingProcessor
                            | Screen::ReviewingExport
                            | Screen::ReviewingContract
                    ) {
                    self.scroll_back
                        .saturating_add(if self.help_open() || self.evidence_open() {
                            10
                        } else {
                            self.review_page.max(1)
                        })
                } else {
                    self.scroll_back.saturating_sub(10)
                };
                return true;
            }
            KeyCode::End if self.screen != Screen::Instructing => {
                self.scroll_back = 0;
                return true;
            }
            _ => {}
        }
        false
    }

    pub(super) fn clear_editor(&mut self) -> bool {
        if self.help_open() || self.evidence_open() {
            self.overlay = Overlay::None;
            return true;
        }
        if self.instruction.search_query().is_some() {
            self.instruction.finish_search(true);
            return true;
        }
        if !self.instruction.text().is_empty() {
            self.instruction.clear();
            self.overlay = Overlay::None;
            return true;
        }
        false
    }

    fn search_key(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => self.instruction.finish_search(true),
            KeyCode::Enter => self.instruction.finish_search(false),
            KeyCode::Backspace => self.instruction.search_backspace(),
            KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => {
                self.instruction.search_history();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.instruction.search_insert(&c.to_string());
            }
            _ => {}
        }
    }

    pub(super) fn instruction_key(&mut self, key: crossterm::event::KeyEvent) {
        if self.instruction.search_query().is_some() {
            self.search_key(key);
            return;
        }
        if self.help_open() || self.evidence_open() {
            if key.code == KeyCode::Esc {
                self.overlay = Overlay::None;
            }
            return;
        }
        if key.code == KeyCode::Esc {
            self.overlay = Overlay::None;
            return;
        }
        if self.menu_open() {
            let count = self.matching_commands().len();
            match key.code {
                KeyCode::Up => {
                    self.menu_index = self.menu_index.saturating_sub(1);
                    return;
                }
                KeyCode::Down => {
                    self.menu_index = self
                        .menu_index
                        .saturating_add(1)
                        .min(count.saturating_sub(1));
                    return;
                }
                KeyCode::Tab => {
                    self.complete_command();
                    return;
                }
                KeyCode::Enter if key.modifiers.is_empty() => {
                    self.complete_command();
                    self.start();
                    return;
                }
                _ => {}
            }
        }
        if key.modifiers == KeyModifiers::CONTROL {
            self.control_editor(key.code);
        } else if key.modifiers == KeyModifiers::ALT {
            match key.code {
                KeyCode::Char('b') | KeyCode::Left => self.instruction.word_left(),
                KeyCode::Char('f') | KeyCode::Right => self.instruction.word_right(),
                KeyCode::Backspace => self.instruction.kill_word(),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Enter if key.modifiers == KeyModifiers::SHIFT => {
                    self.instruction.insert("\n");
                }
                KeyCode::Enter if key.modifiers.is_empty() => self.start(),
                KeyCode::Backspace => self.instruction.backspace(),
                KeyCode::Delete => self.instruction.delete(),
                KeyCode::Left => self.instruction.left(),
                KeyCode::Right => self.instruction.right(),
                KeyCode::Home => self.instruction.home(),
                KeyCode::End => self.instruction.end(),
                KeyCode::Up => self.instruction.up(),
                KeyCode::Down => self.instruction.down(),
                KeyCode::Tab => self.refresh_commands(),
                KeyCode::Char('?') if self.instruction.text().is_empty() => {
                    self.overlay = Overlay::Help;
                    self.scroll_back = 0;
                }
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) =>
                {
                    self.instruction.insert(&c.to_string());
                }
                _ => {}
            }
        }
        self.refresh_commands();
    }

    fn control_editor(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') => self.instruction.insert("\n"),
            KeyCode::Char('a') => self.instruction.home(),
            KeyCode::Char('e') => self.instruction.end(),
            KeyCode::Char('b') => self.instruction.left(),
            KeyCode::Char('f') => self.instruction.right(),
            KeyCode::Char('p') => self.instruction.up(),
            KeyCode::Char('n') => self.instruction.down(),
            KeyCode::Char('u') => self.instruction.kill_before(),
            KeyCode::Char('k') => self.instruction.kill_after(),
            KeyCode::Char('w') | KeyCode::Backspace => self.instruction.kill_word(),
            KeyCode::Char('y') => self.instruction.yank(),
            KeyCode::Char('r') => {
                self.overlay = Overlay::None;
                self.instruction.search_history();
            }
            KeyCode::Char('d') => self.instruction.delete(),
            KeyCode::Left => self.instruction.word_left(),
            KeyCode::Right => self.instruction.word_right(),
            _ => {}
        }
    }
}
