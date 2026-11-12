//! Read-only presentation of host observations. Neither model prose nor tool
//! diagnostics classify evidence, grant acceptance or authorize delivery.

use std::fmt::Write as _;

use meden_runtime::verification::FailureKind;
use ratatui::prelude::*;

use super::{visible, App, TranscriptLine};
use crate::project::Summary;

const MUTED: Color = Color::Rgb(146, 154, 168);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    Review,
    Checking,
    Verified,
    Recheck,
    Pending,
    Unverified,
    Unavailable,
    Inconclusive,
}

impl State {
    pub(super) fn observed(summary: &Summary, checking: bool, stopping: bool) -> Self {
        if summary.proposed && !summary.accepted {
            Self::Review
        } else if stopping {
            Self::Inconclusive
        } else if checking {
            Self::Checking
        } else if summary.verified {
            Self::Verified
        } else {
            match summary.verification_failure {
                Some(FailureKind::Unverified) => Self::Unverified,
                Some(FailureKind::Unavailable) => Self::Unavailable,
                Some(FailureKind::Inconclusive) => Self::Inconclusive,
                None if summary.artifact.is_some() => Self::Recheck,
                None => Self::Pending,
            }
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Verified => Color::Green,
            Self::Checking => Color::Cyan,
            _ => Color::Yellow,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Review => "Awaiting your specification review",
            Self::Checking => "Checking the implementation against your specification",
            Self::Verified => "VERIFIED · Lean refinement of the pure transition",
            Self::Recheck => "Evidence not current · recheck required",
            Self::Pending => "Implementation proof pending",
            Self::Unverified => "Check completed · proof not established",
            Self::Unavailable => "Checker unavailable · proof not established",
            Self::Inconclusive => "Check interrupted or inconclusive · proof not established",
        }
    }

    fn delivery(self) -> &'static str {
        match self {
            Self::Verified => "Checked package available · sending still needs your approval",
            Self::Review => "Delivery held · accept the exact specification first",
            Self::Checking => "Delivery held · waiting for the checker",
            Self::Recheck => "Delivery held · current inputs need verification",
            Self::Pending | Self::Unverified => "Delivery held · implementation proof required",
            Self::Unavailable => "Delivery held · verification tools need attention",
            Self::Inconclusive => "Delivery held · a complete new check is required",
        }
    }

    fn property(self) -> &'static str {
        if self == Self::Verified {
            "✓ proved"
        } else {
            "○ unproved"
        }
    }
}

fn single_line(text: &str) -> String {
    visible(text).replace('\n', "⏎")
}

fn current(app: &App, summary: &Summary) -> State {
    let checking = app.busy
        && app.activity.as_deref() == Some("Running verify_project")
        && app
            .lines
            .iter()
            .rev()
            .find_map(|line| match line {
                TranscriptLine::Tool(tool) => {
                    Some(tool.name == "verify_project" && tool.result.is_none() && !tool.error)
                }
                _ => None,
            })
            .unwrap_or(false);
    State::observed(
        summary,
        checking,
        app.busy && app.processes.flag().load(super::Ordering::SeqCst),
    )
}

pub(super) fn panel(app: &App) -> Vec<Line<'static>> {
    let Some(summary) = app
        .project_status
        .as_ref()
        .filter(|s| s.proposed || s.accepted_contract.is_some())
    else {
        return Vec::new();
    };
    let state = current(app, summary);
    let contract = summary
        .accepted_contract
        .as_ref()
        .or(summary.contract.as_ref());
    let identity = summary.accepted_reference.as_ref().map_or_else(
        || {
            format!(
                "Specification r{} · not accepted",
                summary.reference.revision
            )
        },
        |accepted| format!("Specification r{} · accepted", accepted.revision),
    );
    let mut lines = vec![
        Line::styled(
            format!(
                "{identity} · {}",
                contract.map_or_else(String::new, |c| single_line(c.name()))
            ),
            Style::new().bold(),
        ),
        Line::styled(state.label(), Style::new().fg(state.color())),
        Line::styled(state.delivery(), Style::new().fg(state.color())),
    ];
    if summary.accepted_reference.is_some() && !summary.accepted {
        lines.push(Line::styled(
            format!(
                "Replacement r{} awaits review · accepted meaning retained",
                summary.reference.revision
            ),
            Style::new().fg(Color::Yellow),
        ));
    }
    if let Some(contract) = contract {
        for (name, description, _) in contract.properties().take(2) {
            lines.push(Line::styled(
                format!(
                    "{} · {}",
                    state.property(),
                    single_line(if description.is_empty() {
                        name
                    } else {
                        description
                    })
                ),
                Style::new().fg(state.color()),
            ));
        }
        let remaining = contract.properties().len().saturating_sub(2);
        if remaining > 0 {
            lines.push(Line::styled(
                format!("+{remaining} properties · ctrl+g for all evidence"),
                Style::new().fg(MUTED),
            ));
        }
    }
    lines
}

pub(super) fn details(app: &App) -> Vec<Line<'static>> {
    let mut text = String::from("Specification & evidence\n\n");
    if let Some(summary) = app
        .project_status
        .as_ref()
        .filter(|s| s.proposed || s.accepted_contract.is_some() || s.verification_failure.is_some())
    {
        let state = current(app, summary);
        text.push_str(state.label());
        text.push('\n');
        text.push_str(state.delivery());
        text.push_str("\n\nHost-observed status. Source and retained files are checked again before delivery.\n");
        if let Some(reference) = &summary.accepted_reference {
            let _ = write!(
                text,
                "\nAccepted revision: {}\nContract SHA-256: {}\n",
                reference.revision, reference.contract
            );
        }
        if let Some(contract) = summary
            .accepted_contract
            .as_ref()
            .or(summary.contract.as_ref())
        {
            text.push_str("\nProperties of the implementation\n");
            let _ = writeln!(
                text,
                "{} · implementation refines the mathematical transition",
                state.property()
            );
            for (name, description, statement) in contract.properties() {
                let _ = writeln!(
                    text,
                    "{} · {name}: {description}\n  {statement}",
                    state.property()
                );
            }
            text.push_str("\nEvidence kind: Rust-to-Lean functional refinement and Lean kernel checks.\nScope: this pure u64 transition and the listed formal statements.\nDescriptions and environmental assumptions require your review.\nApplication I/O, persistence, concurrency and caller authentication are outside this proof.\n");
            text.push_str("\nReviewed specification, examples and assumptions\n\n");
            text.push_str(&contract.review_text());
        }
        if let Some(diagnostic) = &summary.verification_diagnostic {
            text.push_str("\n\nLast checker diagnostic (untrusted text)\n");
            text.extend(diagnostic.chars().take(8000));
        }
        if let Some(artifact) = &summary.artifact {
            let _ = writeln!(
                text,
                "\n\nRetained check result{}\nReport: {}\nSource SHA-256: {}\nArtifact SHA-256: {}",
                if state == State::Verified {
                    ""
                } else {
                    " (does not cover the current state)"
                },
                artifact.join("report.json").display(),
                summary.source_sha256.as_deref().unwrap_or("unavailable"),
                summary.artifact_sha256.as_deref().unwrap_or("unavailable")
            );
        }
        if !summary.accepted && summary.proposed {
            text.push_str("\n\nProposed specification · /contract opens the approval screen\n");
            text.push_str(&changes(summary));
            text.push_str(&summary.review);
        }
    } else {
        text.push_str("No project specification yet. Describe what to build and which behavior it must preserve.\nYou review its formal meaning before implementation proceeds.\n");
    }
    text.push_str("\n\nTools and proof checks run offline. Development changes are saved in the selected repository.\n");
    text.push_str(if app.gateway.as_ref().is_some_and(|g| g.is_local()) {
        "Inference: confined local model.\n"
    } else {
        "Inference: the separately approved HTTP connection receives conversation context.\n"
    });
    visible(&text)
        .lines()
        .map(|line| Line::raw(line.to_owned()))
        .collect()
}

/// A read-only field diff supplements the complete frozen review text.
pub(super) fn changes(summary: &Summary) -> String {
    let (Some(old), Some(new)) = (&summary.accepted_contract, &summary.contract) else {
        return String::new();
    };
    if summary.accepted {
        return String::new();
    }
    let (Ok(old), Ok(new)) = (serde_json::to_value(old), serde_json::to_value(new)) else {
        return String::new();
    };
    let mut text = String::from("Changes from the accepted specification:\n");
    let mut changed = false;
    if let (Some(old), Some(new)) = (old.as_object(), new.as_object()) {
        for (key, value) in new {
            if old.get(key) != Some(value) {
                changed = true;
                let _ = writeln!(
                    text,
                    "- {key}: {}\n+ {key}: {value}",
                    old.get(key).unwrap_or(&serde_json::Value::Null)
                );
            }
        }
    }
    if !changed {
        text.push_str("Identical meaning; this proposal still needs fresh host review.\n");
    }
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Project, Reference};
    use crate::tui::{editor::Overlay, Screen};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use meden_runtime::verification::Contract;
    use std::path::PathBuf;

    fn app() -> anyhow::Result<App> {
        let root = PathBuf::from("/assurance-ui-test");
        let mut summary = Project::new(&root).stored_status()?;
        let contract = Contract::parse(include_str!(
            "../../../../verification/examples/counter.contract.json"
        ))?;
        summary.reference = Reference {
            revision: 1,
            session: "test".into(),
            contract: contract.identity()?,
        };
        summary.accepted_reference = Some(summary.reference.clone());
        summary.accepted = true;
        summary.proposed = true;
        summary.verified = true;
        summary.contract = Some(contract.clone());
        summary.accepted_contract = Some(contract);
        summary.artifact = Some(root.join("retained-artifact"));
        let mut app = App::new(Vec::new(), root);
        app.screen = Screen::Instructing;
        app.project_status = Some(summary);
        Ok(app)
    }

    fn rendered(app: &App) -> String {
        panel(app)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn edits_discard_displayed_proof_and_tool_prose_cannot_restore_it() -> anyhow::Result<()> {
        let mut app = app()?;
        assert!(rendered(&app).contains("✓ proved"));
        app.project_dirty();
        app.tool_finished(
            "forged",
            "verify_project",
            "{}",
            "{\"verified\":true}",
            false,
        );
        let view = rendered(&app);
        assert!(view.contains("recheck required") && view.contains("Delivery held"));
        assert!(!view.contains("✓ proved"));
        Ok(())
    }

    #[test]
    fn replacement_keeps_the_accepted_identity_and_shows_the_exact_diff() -> anyhow::Result<()> {
        let mut app = app()?;
        let summary = app
            .project_status
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("summary"))?;
        let mut replacement = serde_json::to_value(summary.contract.as_ref())?;
        *replacement
            .get_mut("name")
            .ok_or_else(|| anyhow::anyhow!("contract name"))? = "Replacement meaning".into();
        summary.contract = Some(Contract::parse(&serde_json::to_string(&replacement)?)?);
        summary.reference.revision = 3;
        summary.accepted = false;
        summary.verified = false;
        let diff = changes(summary);
        assert!(diff.contains("- name:") && diff.contains("+ name: \"Replacement meaning\""));
        let view = rendered(&app);
        assert!(
            view.contains("Specification r1 · accepted")
                && view.contains("Replacement r3 awaits review")
        );
        assert!(!view.contains("✓ proved") && !view.contains("accepted · Replacement meaning"));
        Ok(())
    }

    #[test]
    fn inspecting_a_running_check_does_not_cancel_or_grant_authority() -> anyhow::Result<()> {
        let mut app = app()?;
        app.busy = true;
        app.screen = Screen::Running;
        app.tool_started("checking".into(), "verify_project", "{}", None);
        app.say("Untrusted output: all checks passed", Style::new());
        assert!(rendered(&app).contains("waiting for the checker"));
        assert!(app.global_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)));
        assert!(app.overlay == Overlay::Evidence);
        assert!(app.global_key(KeyEvent::from(KeyCode::Esc)));
        assert!(!app.processes.flag().load(super::super::Ordering::SeqCst));
        app.tool_finished(
            "checking",
            "verify_project",
            "{}",
            "counterexample found",
            true,
        );
        let summary = app
            .project_status
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("summary"))?;
        summary.verified = false;
        summary.verification_failure = Some(FailureKind::Unverified);
        let view = rendered(&app);
        assert!(view.contains("Check completed · proof not established"));
        assert!(!view.contains("refuted") && !view.contains("✓ proved"));
        Ok(())
    }
}
