//! Host connection preferences and a separate, masked settings editor.
//! Settings never restore a processor grant or enter model conversation history.

use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use meden_llm::{LocalConfig, Provider};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation as _;

use super::input::Input;

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Profile {
    Http {
        base_url: String,
        model: String,
        api_key: Option<String>,
        ask_key: bool,
    },
    Local {
        engine: PathBuf,
        model: PathBuf,
    },
    Existing {
        id: String,
        model: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u32,
    profile: Profile,
}

impl Profile {
    fn http(provider: &Provider, remember_key: bool) -> Self {
        Self::Http {
            base_url: provider.base_url.clone(),
            model: provider.model.clone(),
            api_key: provider.api_key.clone().filter(|_| remember_key),
            ask_key: provider.api_key.is_some() && !remember_key,
        }
    }
    pub(super) fn load(root: &Path) -> Result<Option<Self>> {
        let path = crate::storage::directory(root)?.join("connection.json");
        let Some(saved) = crate::storage::read::<Saved>(&path)? else {
            return Ok(None);
        };
        anyhow::ensure!(
            saved.version == 1,
            "unsupported connection settings version"
        );
        Ok(Some(saved.profile))
    }

    pub(super) fn save(&self, root: &Path) -> Result<()> {
        crate::storage::write(
            &crate::storage::directory(root)?.join("connection.json"),
            &Saved {
                version: 1,
                profile: self.clone(),
            },
        )
    }

    pub(super) fn provider(&self) -> Option<Provider> {
        match self {
            Self::Http {
                base_url,
                model,
                api_key,
                ..
            } => Some(http(base_url, model, api_key.clone())),
            _ => None,
        }
    }
}

fn http(base_url: &str, model: &str, api_key: Option<String>) -> Provider {
    Provider {
        id: "custom".into(),
        label: "Your HTTP connection".into(),
        base_url: base_url.to_owned(),
        model: model.to_owned(),
        api_key,
        protocol: meden_llm::Protocol::ChatCompletions,
        note: "configured in the TUI".into(),
    }
}

pub(super) enum Candidate {
    Http(Provider),
    Local(LocalConfig),
}

pub(super) enum Action {
    None,
    Cancel,
    Submit,
}

pub(super) struct Form {
    local: bool,
    fields: Vec<Input>,
    focus: usize,
    remember_key: bool,
    pub error: Option<String>,
}

impl Form {
    pub(super) fn new(profile: Option<&Profile>, local: bool) -> Self {
        let (values, remember_key) = match profile {
            Some(Profile::Http {
                base_url,
                model,
                api_key,
                ..
            }) if !local => (
                vec![
                    base_url.clone(),
                    model.clone(),
                    api_key.clone().unwrap_or_default(),
                ],
                api_key.is_some(),
            ),
            Some(Profile::Local { engine, model }) if local => (
                vec![engine.display().to_string(), model.display().to_string()],
                false,
            ),
            _ if local => (vec![String::new(), String::new()], false),
            _ => (
                vec!["https://ollama.com/v1".into(), String::new(), String::new()],
                false,
            ),
        };
        Self {
            local,
            fields: values
                .into_iter()
                .map(|value| {
                    let mut input = Input::default();
                    input.insert(&value);
                    input
                })
                .collect(),
            focus: 0,
            remember_key,
            error: None,
        }
    }

    fn submit_index(&self) -> usize {
        self.fields.len().saturating_add(usize::from(!self.local))
    }

    pub(super) fn paste(&mut self, text: &str) {
        let before = self.fields.first().map(|input| input.text().to_owned());
        if let Some(input) = self.fields.get_mut(self.focus) {
            if input.text().len().saturating_add(text.len()) <= 8192 {
                input.insert(&text.replace(['\r', '\n'], ""));
            }
        }
        self.changed_url(before.as_deref());
        self.error = None;
    }

    fn changed_url(&mut self, before: Option<&str>) {
        if !self.local && before != self.fields.first().map(Input::text) {
            if let Some(key) = self.fields.get_mut(2) {
                key.clear();
            }
            self.remember_key = false;
        }
    }

    pub(super) fn key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Action::Cancel,
            KeyCode::Tab | KeyCode::Down => {
                self.focus = self
                    .focus
                    .saturating_add(1)
                    .checked_rem(self.submit_index().saturating_add(1))
                    .unwrap_or(0);
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.focus = self.focus.checked_sub(1).unwrap_or(self.submit_index());
            }
            KeyCode::Enter if key.modifiers.is_empty() => {
                if self.focus >= self.fields.len().saturating_sub(1) {
                    return Action::Submit;
                }
                self.focus = self.focus.saturating_add(1);
            }
            KeyCode::Char(' ') if self.focus == self.fields.len() && !self.local => {
                self.remember_key = !self.remember_key;
            }
            _ => self.edit(key),
        }
        Action::None
    }

    fn edit(&mut self, key: KeyEvent) {
        let before = self.fields.first().map(|input| input.text().to_owned());
        if let Some(input) = self.fields.get_mut(self.focus) {
            match key.code {
                KeyCode::Backspace => input.backspace(),
                KeyCode::Delete => input.delete(),
                KeyCode::Left => input.left(),
                KeyCode::Right => input.right(),
                KeyCode::Home => input.home(),
                KeyCode::End => input.end(),
                KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => input.home(),
                KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => input.end(),
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => input.delete(),
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => input.clear(),
                KeyCode::Char(c)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) && input.text().len() < 8192 =>
                {
                    input.insert(&c.to_string());
                }
                _ => {}
            }
        }
        self.changed_url(before.as_deref());
        self.error = None;
    }

    pub(super) fn candidate(&self, root: &Path) -> Result<(Candidate, Profile)> {
        let value = |index| self.fields.get(index).map(Input::text).unwrap_or_default();
        anyhow::ensure!(
            !value(0).trim().is_empty() && !value(1).trim().is_empty(),
            "Complete both connection fields before continuing."
        );
        if self.local {
            let engine = host_path(root, value(0))?;
            let model = host_path(root, value(1))?;
            Ok((
                Candidate::Local(LocalConfig {
                    engine: engine.clone(),
                    model: model.clone(),
                }),
                Profile::Local { engine, model },
            ))
        } else {
            let api_key = (!value(2).is_empty()).then(|| value(2).to_owned());
            let profile = Profile::Http {
                base_url: value(0).to_owned(),
                model: value(1).to_owned(),
                api_key: api_key.clone().filter(|_| self.remember_key),
                ask_key: api_key.is_some() && !self.remember_key,
            };
            Ok((Candidate::Http(http(value(0), value(1), api_key)), profile))
        }
    }
}

fn host_path(root: &Path, value: &str) -> Result<PathBuf> {
    let path = if value == "~" || value.starts_with("~/") {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| anyhow::anyhow!("HOME is unavailable; enter an absolute path"))?;
        PathBuf::from(home).join(value.strip_prefix("~/").unwrap_or_default())
    } else {
        PathBuf::from(value)
    };
    // Keep the original filename/symlink alias for split-GGUF preparation.
    // Making paths absolute must not canonicalize a first shard into a cache blob.
    Ok(if path.is_absolute() {
        path
    } else {
        root.join(path)
    })
}

pub(super) fn draw(frame: &mut Frame<'_>, form: &Form, body: Rect, footer: Rect) {
    let parts = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(
            u16::try_from(form.fields.len())
                .unwrap_or(0)
                .saturating_mul(3),
        ),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .split(body);
    let title = if form.local {
        "Use a local model"
    } else {
        "Add a model connection"
    };
    let hint = if form.local {
        "Choose prepared llama-server and GGUF files. Execution stays offline."
    } else {
        "Ollama Cloud is prefilled. Edit the URL for another OpenAI-compatible server."
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(title, Style::new().bold()),
            Line::raw(hint),
        ])
        .wrap(Wrap { trim: false }),
        parts.first().copied().unwrap_or_default(),
    );
    draw_fields(frame, form, parts.get(1).copied().unwrap_or_default());
    if !form.local {
        let mark = if form.remember_key { "[x]" } else { "[ ]" };
        let style = if form.focus == form.fields.len() {
            Style::new().fg(Color::Cyan)
        } else {
            Style::default()
        };
        frame.render_widget(Paragraph::new(vec![Line::styled(format!("{mark} Remember API key on this device"), style),
            Line::raw("Leave off to keep the key only for this session. Settings stay outside your repository.")]).wrap(Wrap { trim: false }), parts.get(2).copied().unwrap_or_default());
    }
    if let Some(error) = &form.error {
        frame.render_widget(
            Paragraph::new(super::visible(error))
                .style(Style::new().fg(Color::Red))
                .wrap(Wrap { trim: false }),
            parts.get(3).copied().unwrap_or_default(),
        );
    }
    let action = if form.local {
        "Use offline model"
    } else {
        "Review connection"
    };
    let style = if form.focus == form.submit_index() {
        Style::new().fg(Color::Black).bg(Color::Cyan).bold()
    } else {
        Style::new().fg(Color::Cyan)
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(format!(" {action} "), style),
            Line::raw("Tab / ↑↓ move · Enter next / continue · Space toggle · Esc cancel"),
        ])
        .wrap(Wrap { trim: false }),
        footer,
    );
}

fn draw_fields(frame: &mut Frame<'_>, form: &Form, area: Rect) {
    let rows = Layout::vertical(vec![Constraint::Length(3); form.fields.len()]).split(area);
    for (index, input) in form.fields.iter().enumerate() {
        let label = if form.local {
            if index == 0 {
                "llama-server executable"
            } else {
                "GGUF model file / first shard"
            }
        } else {
            match index {
                0 => "API base URL",
                1 => "Model",
                _ => "API key (optional)",
            }
        };
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(label)
            .border_style(Style::new().fg(if index == form.focus {
                Color::Cyan
            } else {
                Color::DarkGray
            }));
        let inner = block.inner(rows.get(index).copied().unwrap_or_default());
        let view = input.view(usize::from(inner.width).max(1));
        let text = view.lines.get(view.row).cloned().unwrap_or_default();
        let text = if !form.local && index == 2 {
            format!(
                "{}{}",
                text.get(..2).unwrap_or_default(),
                "•".repeat(text.get(2..).unwrap_or_default().graphemes(true).count())
            )
        } else {
            text
        };
        frame.render_widget(
            Paragraph::new(text).block(block),
            rows.get(index).copied().unwrap_or_default(),
        );
        if index == form.focus && inner.width > 0 && inner.height > 0 {
            frame.set_cursor_position((
                inner.x.saturating_add(
                    u16::try_from(view.column)
                        .unwrap_or(0)
                        .min(inner.width.saturating_sub(1)),
                ),
                inner.y,
            ));
        }
    }
}

impl super::App {
    pub(super) fn open_setup(&mut self, local: bool) {
        self.connection_error = None;
        self.review_area = Rect::default();
        self.screen = super::Screen::Picking;
        self.overlay = super::editor::Overlay::None;
        let profile = if local {
            self.local_config
                .as_ref()
                .map(|config| Profile::Local {
                    engine: config.engine.clone(),
                    model: config.model.clone(),
                })
                .or_else(|| self.connection_profile.clone())
        } else {
            self.custom_profile
                .clone()
                .or_else(|| self.connection_profile.clone())
        };
        let mut form = Form::new(profile.as_ref(), local);
        if !local {
            if let Some(provider) = self.selected().filter(|provider| provider.id == "custom") {
                if let Some(key) = form.fields.get_mut(2) {
                    key.clear();
                    key.insert(provider.api_key.as_deref().unwrap_or_default());
                }
            }
        }
        self.connection_form = Some(form);
    }

    pub(super) fn setup_key(&mut self, key: KeyEvent) {
        let Some(form) = &mut self.connection_form else {
            return;
        };
        match form.key(key) {
            Action::None => {}
            Action::Cancel => {
                self.connection_form = None;
            }
            Action::Submit => {
                let candidate = form.candidate(&self.root);
                let result = candidate
                    .and_then(|(candidate, profile)| self.apply_connection(candidate, profile));
                if let Err(error) = result {
                    if let Some(form) = &mut self.connection_form {
                        form.error = Some(format!(
                            "Could not prepare this connection: {error}. Check the fields above."
                        ));
                    }
                }
            }
        }
    }

    pub(super) fn apply_connection(
        &mut self,
        candidate: Candidate,
        profile: Profile,
    ) -> Result<()> {
        let (gateway, local) = match candidate {
            Candidate::Http(provider) => (
                meden_llm::Gateway::prepare(self.session_id(), provider)?,
                None,
            ),
            Candidate::Local(config) => {
                let gateway = meden_llm::Gateway::prepare_local(
                    self.session_id(),
                    &config,
                    &std::env::current_exe()?,
                    std::sync::Arc::clone(&self.processes),
                )?;
                (gateway, Some(config))
            }
        };
        if let Some(previous) = self.gateway.take() {
            let _ = previous.revoke_from_host();
        }
        let provider = gateway.provider().clone();
        let index = self
            .providers
            .iter()
            .position(|p| p.id == provider.id)
            .unwrap_or(self.providers.len());
        if let Some(current) = self.providers.get_mut(index) {
            *current = provider;
        } else {
            self.providers.push(provider);
        }
        if let Some(config) = local {
            self.local_config = Some(config);
        }
        self.picker.select(Some(index));
        self.worker = None;
        self.worker_gateway = None;
        self.rx = None;
        self.connection_form = None;
        if matches!(profile, Profile::Http { .. }) {
            self.custom_profile = Some(profile.clone());
        }
        self.connection_profile = Some(profile);
        self.review_area = Rect::default();
        self.scroll_back = 0;
        let local = gateway.is_local();
        self.gateway = Some(std::sync::Arc::new(gateway));
        self.screen = if local {
            super::Screen::Instructing
        } else {
            super::Screen::ReviewingProcessor
        };
        if local {
            self.remember_connection();
            if self.project.pending() {
                self.open_contract();
            }
        }
        Ok(())
    }

    pub(super) fn choose_profile(&mut self, provider: &Provider) {
        if provider.id == "custom" {
            let remember = matches!(&self.custom_profile, Some(Profile::Http { base_url, api_key: Some(key), .. })
                if base_url == &provider.base_url && provider.api_key.as_ref() == Some(key));
            let profile = Profile::http(provider, remember);
            self.custom_profile = Some(profile.clone());
            self.connection_profile = Some(profile);
            return;
        }
        self.connection_profile = Some(Profile::Existing {
            id: provider.id.clone(),
            model: provider.model.clone(),
        });
    }

    pub(super) fn remember_connection(&mut self) {
        if self.workspace.is_none() {
            return;
        }
        if let Some(profile) = &self.connection_profile {
            if let Err(error) = profile.save(&self.root) {
                self.say(
                    format!(
                        "Connected for this session, but settings could not be remembered: {error}"
                    ),
                    Style::new().fg(Color::Yellow),
                );
            }
        }
    }

    pub(super) fn restore_connection(&mut self) -> bool {
        match self.restore_profile() {
            Ok(restored) => restored,
            Err(error) => {
                self.connection_error = Some(format!(
                    "Could not restore the saved connection: {error}. Choose or add a connection."
                ));
                false
            }
        }
    }

    fn restore_profile(&mut self) -> Result<bool> {
        let Some(profile) = Profile::load(&self.root)? else {
            return Ok(false);
        };
        self.connection_profile = Some(profile.clone());
        if matches!(profile, Profile::Http { .. }) {
            self.custom_profile = Some(profile.clone());
        }
        match &profile {
            Profile::Http { ask_key: true, .. } => {
                self.connection_profile = Some(profile);
                self.open_setup(false);
                if let Some(form) = &mut self.connection_form {
                    form.focus = 2;
                }
            }
            Profile::Http { .. } => {
                let provider = profile
                    .provider()
                    .ok_or_else(|| anyhow::anyhow!("invalid saved connection"))?;
                self.apply_connection(Candidate::Http(provider), profile)?;
            }
            Profile::Local { engine, model } => {
                let config = LocalConfig {
                    engine: engine.clone(),
                    model: model.clone(),
                };
                self.apply_connection(Candidate::Local(config), profile)?;
            }
            Profile::Existing { id, model } => {
                let index = self
                    .providers
                    .iter()
                    .position(|p| &p.id == id)
                    .ok_or_else(|| anyhow::anyhow!("saved login is not available"))?;
                if let Some(provider) = self.providers.get_mut(index) {
                    provider.model.clone_from(model);
                }
                self.picker.select(Some(index));
                self.select_processor();
            }
        }
        Ok(true)
    }
}
