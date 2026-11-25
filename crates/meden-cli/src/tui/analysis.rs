//! Public TUI adapter for the shared source/meaning service.
use super::{App, Screen, Task, Update};
use anyhow::{Context as _, Result};
use ratatui::prelude::*;
use std::fmt::Write as _;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Instant;

pub(super) enum WorkTask {
    Show(Option<String>),
    Initialize(String),
    Save,
    Check(String),
    Note { id: String, text: String },
    Resume(String),
}

/// Local source operations never select, approve or invoke a model.
pub(super) enum SourceTask {
    Read { paths: Vec<String>, atlas: bool },
    Prepare,
}

impl App {
    pub(super) fn work_command(&mut self, arguments: &str) {
        let parts: Vec<_> = arguments.split_whitespace().collect();
        let task = match parts.as_slice() {
            [] => WorkTask::Show(None),
            ["init", ..]
                if !arguments
                    .strip_prefix("init")
                    .unwrap_or_default()
                    .trim()
                    .is_empty() =>
            {
                WorkTask::Initialize(
                    arguments
                        .strip_prefix("init")
                        .unwrap_or_default()
                        .trim()
                        .to_owned(),
                )
            }
            ["save"] => WorkTask::Save,
            ["check", id] => WorkTask::Check((*id).to_owned()),
            ["resume", id] => WorkTask::Resume((*id).to_owned()),
            ["note", id, "--", ..] => {
                let text = arguments.split_once(" -- ").map(|(_, text)| text.trim());
                let Some(text) = text.filter(|text| !text.is_empty()) else {
                    self.say(
                        "Use /work note ID -- note text.",
                        Style::new().fg(Color::Yellow),
                    );
                    return;
                };
                WorkTask::Note {
                    id: (*id).to_owned(),
                    text: text.to_owned(),
                }
            }
            [id] => WorkTask::Show(Some((*id).to_owned())),
            _ => {
                self.say("Use /work, /work ID, /work init NAME, /work save, /work check ID, /work resume ID, or /work note ID -- text.", Style::new().fg(Color::Yellow));
                return;
            }
        };
        self.work_task(task);
    }

    fn work_task(&mut self, task: WorkTask) {
        let Some(workspace) = self.workspace.as_ref().map(std::sync::Arc::clone) else {
            self.say(
                "No isolated workspace available.",
                Style::new().fg(Color::Red),
            );
            return;
        };
        if matches!(task, WorkTask::Check(_)) {
            self.source_snapshot = None;
            self.invalidate_analysis();
            self.project_dirty();
        }
        // Host work records and local checking require no inference grant.
        // Drop the idle model worker; the next user turn recreates it from saved history.
        self.worker = None;
        self.worker_gateway = None;
        let (tx, rx) = std::sync::mpsc::channel();
        self.rx = Some(rx);
        self.processes.flag().store(false, Ordering::SeqCst);
        let control = std::sync::Arc::clone(&self.processes);
        let project = std::sync::Arc::clone(&self.project);
        self.busy = true;
        self.turn_started = Some(Instant::now());
        self.activity = Some(
            if matches!(task, WorkTask::Check(_)) {
                "Checking portable Lean work offline"
            } else {
                "Reading / saving portable work records"
            }
            .into(),
        );
        self.screen = Screen::Running;
        self.scroll_back = 0;
        std::thread::spawn(move || {
            let result = (|| -> Result<String> {
                match task {
                    WorkTask::Show(id) => {
                        work_text(crate::handoff::view(&workspace, &control), id.as_deref())
                    }
                    WorkTask::Initialize(name) => {
                        crate::handoff::initialize(&workspace, &control, name)?;
                        Ok("Portable workspace initialized in .meden/workspace.json. Use /work to inspect it.".into())
                    }
                    WorkTask::Save => {
                        crate::handoff::checkpoint(&workspace, &control)?;
                        Ok("Conversation checkpoint saved to .meden/workspace.json. Commit and push the record with your source changes.".into())
                    }
                    WorkTask::Check(id) => {
                        let checked =
                            crate::handoff::check(&workspace, &control, &id, Some(&project));
                        let text = work_text(crate::handoff::view(&workspace, &control), Some(&id));
                        match checked {
                            Ok(()) => Ok(format!("{}\nOffline Lean check completed for {id}. The recorded result is historical.", text?)),
                            Err(error) => anyhow::bail!("{error:#}\n{}\nPortable work stopped: offline Lean check failed / interrupted for {id}. Inspect the retained output above and rerun explicitly.", text.unwrap_or_default()),
                        }
                    }
                    WorkTask::Note { id, text } => {
                        crate::handoff::note(&workspace, &control, &id, text, None)?;
                        Ok(format!(
                            "Work note saved for {id}. Use /work {id} to inspect its history."
                        ))
                    }
                    WorkTask::Resume(id) => {
                        let instruction = crate::handoff::continuation(&workspace, &control, &id)?;
                        let _ = tx.send(Update::WorkResume(instruction));
                        Ok(String::new())
                    }
                }
            })();
            let failed = result.is_err();
            let text = result.unwrap_or_else(|error| format!("Portable work stopped: {error:#}"));
            let _ = tx.send(Update::AnalysisDone { text, failed });
        });
    }
    pub(super) fn source_task(&mut self, task: SourceTask) {
        let Some(workspace) = self.workspace.as_ref().map(std::sync::Arc::clone) else {
            self.say(
                "No isolated workspace available.",
                Style::new().fg(Color::Red),
            );
            return;
        };
        if matches!(task, SourceTask::Read { .. }) {
            self.source_snapshot = None;
            self.invalidate_analysis();
        }
        self.worker = None;
        self.worker_gateway = None;
        let (tx, rx) = std::sync::mpsc::channel();
        self.rx = Some(rx);
        self.processes.flag().store(false, Ordering::SeqCst);
        let control = std::sync::Arc::clone(&self.processes);
        let project = std::sync::Arc::clone(&self.project);
        self.busy = true;
        self.last_stop = None;
        self.turn_started = Some(Instant::now());
        self.activity = Some("Reading sources / preparing local compiler".into());
        self.screen = Screen::Running;
        self.scroll_back = 0;
        std::thread::spawn(move || {
            let result = (|| -> Result<String> {
                match task {
                    SourceTask::Read { paths, atlas } => {
                        let source = crate::adapters::preparation::read(
                            &crate::adapters::Context {
                                workspace: &workspace,
                                control: &control,
                                observer: None,
                            },
                            &paths,
                            atlas,
                        )?;
                        control.with_active(|| {
                            let _ = tx.send(Update::Sources(Box::new(source)));
                            Ok(())
                        })?;
                        Ok(String::new())
                    }
                    SourceTask::Prepare => {
                        crate::atlas::Configuration::host(workspace.path()).prepare(&control)?;
                        Ok("Atlas is ready. Use /atlas FILE.lean to read declarations.".into())
                    }
                }
            })();
            if let Ok(summary) = project.status(&workspace, &control) {
                let _ = tx.send(Update::Project(Box::new(summary)));
            }
            let failed = result.is_err();
            let text = result.unwrap_or_else(|error| {
                format!(
                    "Source preparation stopped: {error:#}. Rerun the source command explicitly."
                )
            });
            let _ = tx.send(Update::AnalysisDone { text, failed });
        });
    }
    pub(super) fn analysis_task(&mut self, task: Task) {
        if !matches!(task, Task::ApplyRepair(_) | Task::RejectRepair(_)) && !self.prepare_worker() {
            return;
        }
        if matches!(task, Task::ApplyRepair(_)) {
            self.source_snapshot = None;
            self.invalidate_analysis();
        }
        if matches!(task, Task::Analyze { .. }) {
            self.invalidate_analysis();
        }
        self.processes.flag().store(false, Ordering::SeqCst);
        self.last_stop = None;
        self.scroll_back = 0;
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.send(task).is_ok())
        {
            self.busy = true;
            self.turn_started = Some(Instant::now());
            self.activity = Some("Reading sources / meaning analysis".into());
            self.screen = Screen::Running;
        } else {
            self.worker = None;
            self.say(
                "Analysis worker unavailable. Run the command again.",
                Style::new().fg(Color::Red),
            );
        }
    }
    pub(super) fn invalidate_analysis(&mut self) {
        if let Some(record) = &mut self.analysis {
            if record.state == crate::analysis::State::Complete {
                record.state = crate::analysis::State::Stale;
            }
        }
    }
    pub(super) fn meaning_task(&self, arguments: &str) -> anyhow::Result<Task> {
        let (selection, requirement) = arguments.split_once(" -- ").context(
            "Use /analyze NAME -- requirement, or /analyze FILE:LINE NAME -- requirement",
        )?;
        let snapshot = self
            .source_snapshot
            .as_ref()
            .context("Read the source graph first: /atlas FILE.lean or /sources FILE.rs")?;
        let parts: Vec<_> = selection.split_whitespace().collect();
        let target = match parts.as_slice() {
            [name] => {
                anyhow::ensure!(
                    snapshot.graph_revision().is_some(),
                    "A declaration name needs a compiled graph; otherwise use FILE:LINE NAME"
                );
                let page = meden_app::view::SourcePage::from_snapshot(snapshot)?;
                let graph = page.graph.context("Compiled graph unavailable")?;
                let mut nodes = graph.declarations.iter().filter(|node| node.name == *name);
                let node = nodes
                    .next()
                    .context("Declaration not found in the current graph; use its full name")?;
                anyhow::ensure!(nodes.next().is_none(), "Declaration name is ambiguous");
                crate::semantic::Target {
                    file: node.source.file.clone(),
                    line: usize::try_from(
                        node.source
                            .line
                            .context("Declaration has no source location")?,
                    )?,
                    name: node.name.clone(),
                }
            }
            [location, name] => {
                let (file, line) = location.rsplit_once(':').context("Use FILE:LINE NAME")?;
                crate::semantic::Target {
                    file: file.to_owned(),
                    line: line.parse()?,
                    name: (*name).to_owned(),
                }
            }
            _ => anyhow::bail!(
                "Use /analyze NAME -- requirement, or /analyze FILE:LINE NAME -- requirement"
            ),
        };
        let input = crate::semantic::Input {
            source_revision: snapshot.sources().revision.clone(),
            graph_revision: snapshot.graph_revision().cloned(),
            target,
            requirement: requirement.trim().into(),
        };
        input.validate()?;
        Ok(Task::Analyze {
            snapshot: Box::new(snapshot.clone()),
            input,
        })
    }

    pub(super) fn observed_sources(&mut self, snapshot: crate::analysis::prepared::Snapshot) {
        self.invalidate_analysis();
        let result = (|| -> Result<String> {
            let page = meden_app::view::SourcePage::from_snapshot(&snapshot)?;
            let label = if snapshot.graph_revision().is_some() {
                "Source graph ready"
            } else {
                "Sources ready"
            };
            let mut text = format!(
                "{label}: {} file{}\nRevision: {}",
                snapshot.sources().files.len(),
                if snapshot.sources().files.len() == 1 {
                    ""
                } else {
                    "s"
                },
                snapshot.sources().revision
            );
            for detail in snapshot.details()? {
                let _ = write!(text, "\n{detail}");
            }
            if snapshot.graph_revision().is_some() {
                let graph = page.graph.context("Compiled graph unavailable")?;
                for node in graph.declarations.iter().take(80) {
                    let _ = write!(
                        text,
                        "\n{} · {}:{}",
                        node.name,
                        node.source.file,
                        node.source.line.unwrap_or(1)
                    );
                }
                if graph.declarations.len() > 80 {
                    text.push_str(
                        "\nMore declarations are present; use their full name with /analyze.",
                    );
                }
            }
            Ok(text)
        })();
        let mut text = match result {
            Ok(text) => text,
            Err(error) => {
                self.source_snapshot = None;
                self.say(
                    format!("Source observation unavailable: {error:#}"),
                    Style::new().fg(Color::Red),
                );
                return;
            }
        };
        text.push_str("\nUse /analyze NAME -- requirement (compiled graph), or /analyze FILE:LINE NAME -- requirement.");
        self.source_snapshot = Some(snapshot);
        self.say(text, Style::new().fg(Color::Gray));
    }
}

fn work_text(view: crate::handoff::View, selected: Option<&str>) -> Result<String> {
    if let Some(error) = view.error {
        anyhow::bail!("{error}");
    }
    let Some(document) = view.document else {
        return Ok("No portable workspace yet. Use /work init NAME, then add work items in the GUI or with meden work add.".into());
    };
    anyhow::ensure!(
        selected.is_none_or(|id| document.tasks.iter().any(|task| task.id == id)),
        "Unknown work item"
    );
    let mut text = format!("Portable workspace: {}\nRepository records are historical; no processor grant or native proof authority is restored.\nLean: {}\n", document.name, view.lean);
    for task in document
        .tasks
        .iter()
        .filter(|task| selected.is_none_or(|id| task.id == id))
    {
        let _ = write!(
            text,
            "\n{} · {}\nRequirement: {}\nModule: {}\nTheorems: {}\nNext: {}\n",
            task.id,
            task.title,
            task.requirement,
            task.module,
            task.theorems.join(", "),
            task.next
        );
    }
    let events = document
        .events
        .iter()
        .rev()
        .filter(|event| selected.is_none_or(|id| event.task == id))
        .take(8)
        .collect::<Vec<_>>();
    text.push_str("\nRetained history:\n");
    for event in events.into_iter().rev() {
        let _ = writeln!(
            text,
            "{} · {} · {}\n{}",
            event.id, event.task, event.time, event.note
        );
        if let Some(check) = &event.check {
            let _ = write!(text, "Recorded check: {} (historical)\nModule: {}\nSources: {}\nTermination: {}\n{}\n{}\n", if check.success { "passed" } else { "failed / interrupted" }, check.module, check.source_revision, check.termination, check.stdout, check.stderr);
            let _ = write!(
                text,
                "Checked requirement: {}\nLean executable: {}\nTheorem inspection:\n{}\n",
                check.requirement, check.engine_sha256, check.inspection
            );
        }
    }
    let _ = write!(
        text,
        "\nSaved conversations: {}\n",
        document.conversations.len()
    );
    for conversation in document.conversations.iter().rev().take(1) {
        for message in conversation.messages.iter().rev().take(4).rev() {
            let role = match message.speaker {
                crate::session::history::Speaker::User => "User",
                crate::session::history::Speaker::Assistant => "Assistant",
            };
            let _ = writeln!(text, "{role}: {}", message.text);
        }
    }
    for task in document
        .tasks
        .iter()
        .filter(|task| selected.is_none_or(|id| task.id == id))
    {
        let _ = writeln!(text, "Next for {}: {}", task.id, task.next);
    }
    text.push_str(
        "\nUse /work ID, /work check ID, /work resume ID, /work note ID -- text, or /work save.",
    );
    Ok(text)
}
pub(super) fn execute(
    session: &crate::session::Session,
    repairs: &mut crate::session::Repairs,
    task: Task,
    tx: &Sender<Update>,
) -> bool {
    if matches!(
        task,
        Task::PrepareRepair { .. } | Task::ApplyRepair(_) | Task::RejectRepair(_)
    ) {
        return super::repair::execute(session, repairs, task, tx);
    }
    let result = (|| -> Result<String> {
        match task {
            Task::Analyze { snapshot, input } => {
                let record = session.analyze(snapshot.as_ref(), input, &|record| {
                    let _ = tx.send(Update::Analysis(Box::new(record.clone())));
                })?;
                Ok(record.summary())
            }
            Task::Instruction(_)
            | Task::PrepareRepair { .. }
            | Task::ApplyRepair(_)
            | Task::RejectRepair(_) => unreachable!(),
        }
    })();
    let failed = result.is_err();
    let text = result.unwrap_or_else(|error| {
        format!("Analysis stopped: {error:#}. Use /analysis to inspect the retained outcome.")
    });
    if tx.send(Update::AnalysisDone { text, failed }).is_err() {
        return false;
    }
    true
}
