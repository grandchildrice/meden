//! Public TUI adapter for the shared source/meaning service.
use super::{App, Screen, Task, Update};
use anyhow::Result;
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
                        let checked = crate::handoff::check(&workspace, &control, &id);
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
    pub(super) fn analysis_task(&mut self, task: Task) {
        if !matches!(task, Task::ApplyRepair(_) | Task::RejectRepair(_)) && !self.prepare_worker() {
            return;
        }
        if matches!(task, Task::ApplyRepair(_) | Task::Sources { .. }) {
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
        use anyhow::Context;
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
                let atlas = snapshot.atlas.as_ref().context(
                    "A declaration name needs an Atlas graph; otherwise use FILE:LINE NAME",
                )?;
                let mut nodes = atlas.graph.nodes.iter().filter(|node| node.id == *name);
                let node = nodes
                    .next()
                    .context("Declaration not found in the current graph; use its full name")?;
                anyhow::ensure!(nodes.next().is_none(), "Declaration name is ambiguous");
                crate::semantic::Target {
                    file: node.file_path.clone(),
                    line: node
                        .line_start
                        .context("Declaration has no source location")?,
                    name: node.id.clone(),
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
            source_revision: snapshot.revision.clone(),
            graph_revision: snapshot.atlas.as_ref().map(|atlas| atlas.revision.clone()),
            target,
            requirement: requirement.trim().into(),
        };
        input.validate()?;
        Ok(Task::Analyze {
            snapshot: Box::new(snapshot.clone()),
            input,
        })
    }

    pub(super) fn observed_sources(&mut self, snapshot: crate::analysis::sources::Snapshot) {
        self.invalidate_analysis();
        let mut text = format!(
            "Source graph ready: {} files\nRevision: {}",
            snapshot.files.len(),
            snapshot.revision
        );
        if let Some(collection) = &snapshot.collection {
            let _ = write!(
                text,
                "\nSelected roots: {}\nExternal modules (not expanded): {}",
                collection.roots.join(", "),
                collection.external_modules.join(", ")
            );
            for package in &collection.packages {
                let _ = write!(
                    text,
                    "\nLake package: {} · {}",
                    package.name,
                    package.source_roots.join(", ")
                );
            }
        }
        if let Some(atlas) = &snapshot.atlas {
            for node in atlas.graph.nodes.iter().take(80) {
                let _ = write!(
                    text,
                    "\n{} · {}:{}",
                    node.id,
                    node.file_path,
                    node.line_start.unwrap_or(1)
                );
            }
            if atlas.graph.nodes.len() > 80 {
                text.push_str(
                    "\nMore declarations are present; use their full name with /analyze.",
                );
            }
        }
        text.push_str("\nUse /analyze NAME -- requirement (Atlas), or /analyze FILE:LINE NAME -- requirement.");
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
    repairs: &mut crate::analysis::repair::Queue,
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
            Task::Sources { paths, atlas } => {
                let snapshot = session.sources(&paths, atlas)?;
                let _ = tx.send(Update::Sources(Box::new(snapshot)));
                Ok(String::new())
            }
            Task::PrepareAtlas => {
                session.prepare_atlas()?;
                Ok("Atlas is ready. Use /atlas FILE.lean to read declarations.".into())
            }
            Task::Analyze { snapshot, input } => {
                let record = session.analyze(&snapshot, input, &|record| {
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
