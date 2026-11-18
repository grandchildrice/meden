//! Public TUI adapter for the shared source/meaning service.
use super::{App, Screen, Task, Update};
use anyhow::Result;
use ratatui::prelude::*;
use std::fmt::Write as _;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Instant;

impl App {
    pub(super) fn analysis_task(&mut self, task: Task) {
        if !self.prepare_worker() {
            return;
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
        let mut text = format!("Source graph ready: {} files", snapshot.files.len());
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
pub(super) fn execute(session: &crate::session::Session, task: Task, tx: &Sender<Update>) -> bool {
    let result = (|| -> Result<String> {
        match task {
            Task::Sources { paths, atlas } => {
                let snapshot = session.sources(&paths, atlas)?;
                let _ = tx.send(Update::Sources(snapshot));
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
            Task::Instruction(_) => unreachable!(),
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
