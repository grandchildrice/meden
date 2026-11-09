//! Instruction-only JSON Lines. Tools execute inside Meden's private copy.

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use meden_agent::conversation::{Event, Status, ToolCall};
use meden_llm::Gateway;
use meden_runtime::process::Control;
use meden_runtime::workspace::Workspace;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::exports::{Exports, Reference, Summary};
use crate::session::Session;

const INPUT_LIMIT: u64 = 1_048_576;

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Instruction(String),
    ListExports(()),
    ReviewExport(Reference),
    ApproveExport(Reference),
    RejectExport(Reference),
    ReviewContract(crate::project::Reference),
    AcceptContract(crate::project::Reference),
    RejectContract(crate::project::Reference),
    ProjectStatus(()),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Out<'a> {
    Project {
        summary: &'a crate::project::Summary,
    },
    Exports {
        exports: Vec<Summary>,
    },
    ExportReview {
        summary: Summary,
        method: &'static str,
        content_type: &'static str,
        body_hex: String,
    },
    ExportResult {
        summary: Option<Summary>,
        http_status: Option<u16>,
        error: Option<String>,
    },
    Ready {
        workspace: &'a Path,
        processor: &'a str,
    },
    Asking {
        attempt: usize,
    },
    Assistant {
        text: &'a str,
        tool_calls: &'a [ToolCall],
    },
    Decision {
        id: &'a str,
        tool: &'a str,
        allowed: bool,
        rule: Option<&'static str>,
        reason: Option<&'a str>,
    },
    ToolResult {
        id: &'a str,
        tool: &'a str,
        text: &'a str,
        error: bool,
    },
    Conversation {
        messages: &'a [Value],
    },
    Done {
        outcome: &'static str,
        answer: Option<&'a str>,
        error: Option<&'a str>,
    },
}

fn send(output: &mut impl Write, message: &Out<'_>) -> Result<()> {
    serde_json::to_writer(&mut *output, message)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

pub fn run(
    first: Command,
    input: &mut impl BufRead,
    output: &mut impl Write,
    workspace: &Arc<Workspace>,
    gateway: Arc<Gateway>,
    processes: &Arc<Control>,
    pages: &Arc<meden_runtime::pages::Pages>,
) -> Result<()> {
    send(
        output,
        &Out::Ready {
            workspace: workspace.path(),
            processor: gateway.endpoint(),
        },
    )?;
    let exports = Arc::new(Exports::new(workspace.path()));
    let project = Arc::new(crate::project::Project::new(workspace.path()));
    let mut session = Session::new(
        Arc::clone(workspace),
        gateway,
        Arc::clone(processes),
        &exports,
        pages,
        &project,
    )?;
    let mut next = Some(first);
    while let Some(command) = next {
        let Command::Instruction(instruction) = command else {
            match command {
                Command::ReviewContract(ref r) => send(
                    output,
                    &Out::Project {
                        summary: &project.review(r)?,
                    },
                )?,
                Command::AcceptContract(ref r) => {
                    send(
                        output,
                        &Out::Project {
                            summary: &project.accept(r)?,
                        },
                    )?;
                    next = Some(Command::Instruction(project.continuation()?));
                    continue;
                }
                Command::RejectContract(ref r) => send(
                    output,
                    &Out::Project {
                        summary: &project.reject(r)?,
                    },
                )?,
                Command::ProjectStatus(()) => send(
                    output,
                    &Out::Project {
                        summary: &project.status(workspace, processes)?,
                    },
                )?,
                _ => export_command(command, &exports, output)?,
            }
            next = read_command(input)?;
            continue;
        };
        let status = run_instruction(&mut session, &instruction, output, processes, &project)?;
        if status == Status::Cancelled {
            return Ok(());
        }
        next = read_command(input)?;
    }
    Ok(())
}

fn run_instruction(
    session: &mut Session,
    instruction: &str,
    output: &mut impl Write,
    processes: &Control,
    project: &crate::project::Project,
) -> Result<Status> {
    let writer = RefCell::new(&mut *output);
    let failure = RefCell::new(None);
    let observer = |event: Event<'_>| {
        if failure.borrow().is_some() {
            return;
        }
        let message = match event {
            Event::Asking(attempt) => Out::Asking { attempt },
            Event::Assistant(reply) => Out::Assistant {
                text: reply.content.as_deref().unwrap_or_default(),
                tool_calls: &reply.tool_calls,
            },
            Event::Decision { call, refusal } => Out::Decision {
                id: &call.id,
                tool: &call.function.name,
                allowed: refusal.is_none(),
                rule: refusal.map(|(rule, _)| rule),
                reason: refusal.map(|(_, reason)| reason),
            },
            Event::ToolResult { call, text, error } => Out::ToolResult {
                id: &call.id,
                tool: &call.function.name,
                text,
                error,
            },
        };
        if let Err(error) = send(&mut *writer.borrow_mut(), &message) {
            *failure.borrow_mut() = Some(error);
            project.cancel(processes);
        }
    };
    let status = session.run(instruction, &observer);
    if let Some(error) = failure.into_inner() {
        return Err(error);
    }
    send(
        output,
        &Out::Conversation {
            messages: session.messages(),
        },
    )?;
    let done = match &status {
        Status::Done(answer) => Out::Done {
            outcome: "done",
            answer: Some(answer),
            error: None,
        },
        Status::Failed(error) => Out::Done {
            outcome: "failed",
            answer: None,
            error: Some(error),
        },
        Status::Incomplete => Out::Done {
            outcome: "incomplete",
            answer: None,
            error: None,
        },
        Status::Cancelled => Out::Done {
            outcome: "cancelled",
            answer: None,
            error: None,
        },
        Status::NeedsReview => Out::Done {
            outcome: "needs_review",
            answer: None,
            error: None,
        },
    };
    let summary = session.project_status()?;
    send(output, &Out::Project { summary: &summary })?;
    send(output, &done)?;
    Ok(status)
}

/// One bounded JSON object per line. Unknown fields cannot register tools,
/// supply host tool results, change policy or create an approval.
pub fn read_command(input: &mut impl BufRead) -> Result<Option<Command>> {
    loop {
        let mut line = String::new();
        let bytes = std::io::Read::take(&mut *input, INPUT_LIMIT + 1).read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }
        anyhow::ensure!(
            u64::try_from(bytes)? <= INPUT_LIMIT,
            "instruction exceeds input budget"
        );
        if line.trim().is_empty() {
            continue;
        }
        return serde_json::from_str(&line)
            .map(Some)
            .context("expected one host command per JSON object");
    }
}

fn export_command(command: Command, exports: &Exports, output: &mut impl Write) -> Result<()> {
    let result = (|| -> Result<Out<'_>> {
        Ok(match command {
            Command::ListExports(()) => Out::Exports {
                exports: exports.list()?,
            },
            Command::ReviewExport(reference) => {
                let entry = exports.review(&reference)?;
                Out::ExportReview {
                    summary: entry.summary()?,
                    method: "POST",
                    content_type: "application/octet-stream",
                    body_hex: entry.hex(0, usize::MAX)?,
                }
            }
            Command::ApproveExport(reference) => {
                let (summary, http_status) = exports.send(&reference)?;
                Out::ExportResult {
                    summary: Some(summary),
                    http_status,
                    error: None,
                }
            }
            Command::RejectExport(reference) => Out::ExportResult {
                summary: Some(exports.reject(&reference)?),
                http_status: None,
                error: None,
            },
            Command::Instruction(_) => {
                anyhow::bail!("instruction cannot be dispatched as an export")
            }
            Command::ReviewContract(_)
            | Command::AcceptContract(_)
            | Command::RejectContract(_)
            | Command::ProjectStatus(()) => {
                anyhow::bail!("project command cannot be dispatched as an export")
            }
        })
    })();
    let response = result.unwrap_or_else(|error| Out::ExportResult {
        summary: None,
        http_status: None,
        error: Some(error.to_string()),
    });
    send(output, &response)
}
