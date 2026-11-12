//! Presentation only. Authority continues to use the last drawn fixed review
//! heading and the existing extracted approval selector.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, Paragraph, Wrap};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::{assurance, visible, App, Screen, Speaker, ToolLine, TranscriptLine};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::Rgb(146, 154, 168);

fn muted() -> Style {
    Style::new().fg(MUTED)
}
fn accent() -> Style {
    Style::new().fg(ACCENT).bold()
}

pub(super) fn draw(frame: &mut Frame<'_>, app: &mut App) {
    app.review_area = Rect::default();
    app.scroll_limit = 0;
    app.scroll_page = 0;
    app.review_frame = frame.area();
    // Respect the user's terminal background and keep text to a readable width.
    frame.render_widget(Block::default(), frame.area());
    let area = reading_area(frame.area());
    let bottom_rows = if app.screen == Screen::Instructing && !app.evidence_open() {
        let rows = app
            .instruction
            .view(usize::from(area.width.saturating_sub(2)))
            .lines
            .len();
        u16::try_from(rows)
            .unwrap_or(u16::MAX)
            .clamp(1, 5)
            .saturating_add(4)
    } else if is_review(&app.screen) {
        4
    } else {
        3
    };
    let areas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(bottom_rows),
    ])
    .split(area);
    let header = areas.first().copied().unwrap_or_default();
    let body = areas.get(1).copied().unwrap_or_default();
    let footer = areas.get(2).copied().unwrap_or_default();
    draw_header(frame, app, header);
    match app.screen {
        Screen::Picking => draw_picker(frame, app, body),
        Screen::ReviewingProcessor => draw_processor_review(frame, app, body),
        Screen::ReviewingContract => draw_contract_review(frame, app, body),
        Screen::ReviewingExport => draw_export_review(frame, app, body),
        Screen::Instructing | Screen::Running => {
            let body = draw_assurance(frame, app, body);
            draw_conversation(frame, app, body);
        }
    }
    if app.screen == Screen::Instructing && !app.evidence_open() {
        draw_input(frame, app, footer);
        if app.menu_open() && !app.help_open() && app.instruction.search_query().is_none() {
            draw_commands(frame, app, body);
        }
    } else {
        draw_controls(frame, app, footer);
    }
}

fn reading_area(area: Rect) -> Rect {
    let margin: u16 = if area.width >= 60 { 3 } else { 1 };
    let width = area.width.saturating_sub(margin.saturating_mul(2)).min(100);
    Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width) / 2),
        area.y,
        width,
        area.height,
    )
}

fn is_review(screen: &Screen) -> bool {
    matches!(
        screen,
        Screen::ReviewingProcessor | Screen::ReviewingContract | Screen::ReviewingExport
    )
}

fn draw_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let model = app
        .selected()
        .map_or("Choose a connection", |provider| provider.model.as_str());
    let status = match &app.project_status {
        Some(summary) if summary.verified => ("VERIFIED project kernel", Color::Green),
        Some(summary) if summary.accepted => ("Project kernel needs verification", Color::Yellow),
        Some(summary) if summary.proposed => ("Specification awaits your review", Color::Yellow),
        _ => ("Private working copy · offline tools", MUTED),
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("MEDEN", accent()),
                Span::raw("  /  "),
                Span::styled(visible(model), muted()),
            ]),
            Line::styled(status.0, Style::new().fg(status.1)),
        ]),
        area,
    );
}

fn draw_assurance(frame: &mut Frame<'_>, app: &App, area: Rect) -> Rect {
    if app.help_open() || app.evidence_open() {
        return area;
    }
    let lines = assurance::panel(app);
    if lines.is_empty() {
        return area;
    }
    // Preserve conversation space on short terminals. Status and the delivery
    // reason precede optional property previews, so those remain visible first.
    let height = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_add(2)
        .min(area.height.saturating_sub(5));
    if height < 5 {
        return area;
    }
    let parts = Layout::vertical([Constraint::Length(height), Constraint::Min(1)]).split(area);
    let panel = parts.first().copied().unwrap_or_default();
    let inner_width = panel.width.saturating_sub(2);
    let lines: Vec<_> = lines
        .into_iter()
        .map(|line| {
            Line::styled(
                ellipsize(&line.to_string(), usize::from(inner_width)),
                line.style,
            )
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(muted())
                .title(" Specification & evidence · ctrl+g "),
        ),
        panel,
    );
    parts.get(1).copied().unwrap_or_default()
}

fn draw_picker(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let areas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled("Choose a model connection", Style::new().bold()),
            Line::styled(
                "Use ↑↓ to choose. Enter opens the connection review.",
                muted(),
            ),
        ]),
        areas.first().copied().unwrap_or_default(),
    );
    let items: Vec<_> = app
        .providers
        .iter()
        .map(|provider| {
            ListItem::new(vec![
                Line::styled(visible(&provider.label), Style::new().bold()),
                Line::from(format!(
                    "  {}  ·  {}",
                    visible(&provider.model),
                    visible(&provider.id)
                )),
                Line::default(),
            ])
        })
        .collect();
    frame.render_stateful_widget(
        List::new(items)
            .highlight_symbol("› ")
            .highlight_style(Style::new().fg(Color::Black).bg(ACCENT))
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always),
        areas.get(1).copied().unwrap_or_default(),
        &mut app.picker,
    );
    frame.render_widget(
        Paragraph::new("You will review where context is sent before any conversation starts.")
            .style(muted())
            .wrap(Wrap { trim: false }),
        areas.get(2).copied().unwrap_or_default(),
    );
}

fn draw_conversation(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    if app.evidence_open() {
        render_scrolled(
            frame,
            app,
            Paragraph::new(assurance::details(app)).wrap(Wrap { trim: false }),
            area,
            false,
        );
        return;
    }
    if app.help_open() {
        let commands = super::commands::COMMANDS
            .iter()
            .map(|(name, description)| format!("{name:12} {description}"))
            .collect::<Vec<_>>()
            .join("\n");
        render_scrolled(
            frame,
            app,
            Paragraph::new(format!(
                "{}\n\nCommands\n\n{commands}",
                super::commands::SHORTCUTS
            ))
            .wrap(Wrap { trim: false }),
            area,
            false,
        );
        return;
    }
    if app.lines.is_empty() {
        frame.render_widget(Paragraph::new(vec![
            Line::default(),
            Line::styled("What would you like to build?", Style::new().bold()),
            Line::default(),
            Line::raw("Describe your app or ask a question in the message box below."),
            Line::raw("For formal verification, you review the specification before implementation."),
            Line::default(),
            Line::styled("Try: Build a vending machine that rejects purchases when balance is insufficient.", muted()),
            Line::default(),
            Line::styled("Files stay in a private working copy. /workspace shows its location.", muted()),
        ]).wrap(Wrap { trim: false }), area);
        return;
    }
    let mut lines = Vec::new();
    for entry in app.lines.iter().skip(app.view_start) {
        match entry {
            TranscriptLine::Message { speaker, text } => {
                let user = matches!(speaker, Speaker::User);
                lines.extend(bubble_lines(
                    text,
                    if user { "› " } else { "● " },
                    user,
                    area.width,
                    Style::new(),
                ));
                lines.push(Line::default());
            }
            TranscriptLine::Draft(text) => {
                lines.extend(bubble_lines(text, "● ", false, area.width, Style::new()));
                lines.push(Line::default());
            }
            TranscriptLine::Reasoning(text) => {
                lines.extend(bubble_lines(
                    text,
                    "∴ ",
                    false,
                    area.width,
                    muted().italic(),
                ));
                lines.push(Line::default());
            }
            TranscriptLine::Tool(tool) => draw_tool_lines(
                &mut lines,
                tool,
                app.show_tool_details,
                app.busy,
                area.width,
            ),
            TranscriptLine::Notice { text, style } => {
                lines.extend(
                    text.lines()
                        .map(|line| Line::styled(format!("· {line}"), *style)),
                );
                lines.push(Line::default());
            }
        }
    }
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    render_scrolled(frame, app, paragraph, area, true);
}

fn render_scrolled(
    frame: &mut Frame<'_>,
    app: &mut App,
    paragraph: Paragraph<'_>,
    area: Rect,
    from_bottom: bool,
) {
    let rows = u16::try_from(paragraph.line_count(area.width)).unwrap_or(u16::MAX);
    app.scroll_limit = rows.saturating_sub(area.height);
    app.scroll_page = area.height;
    app.scroll_back = app.scroll_back.min(app.scroll_limit);
    let offset = if from_bottom {
        app.scroll_limit.saturating_sub(app.scroll_back)
    } else {
        app.scroll_back
    };
    frame.render_widget(paragraph.scroll((offset, 0)), area);
}

fn draw_commands(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let commands = app.matching_commands();
    if commands.is_empty() {
        return;
    }
    let height = u16::try_from(commands.len())
        .unwrap_or(u16::MAX)
        .min(7)
        .saturating_add(2)
        .min(area.height);
    let popup = Rect::new(
        area.x,
        area.y.saturating_add(area.height.saturating_sub(height)),
        area.width,
        height,
    );
    frame.render_widget(Clear, popup);
    let items: Vec<_> = commands
        .iter()
        .map(|(name, description)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{name:12}"), Style::new().bold()),
                Span::raw((*description).to_owned()),
            ]))
        })
        .collect();
    let mut state = ratatui::widgets::ListState::default().with_selected(Some(app.menu_index));
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Commands · ↑↓ choose · enter run · tab complete "),
            )
            .highlight_symbol("› ")
            .highlight_style(Style::new().fg(Color::Black).bg(ACCENT)),
        popup,
        &mut state,
    );
}

/// Wrap content before adding the gutter, so continuation lines align with
/// the message rather than with its marker. Never split a Unicode grapheme.
fn bubble_lines(
    text: &str,
    marker: &str,
    is_user: bool,
    width: u16,
    base: Style,
) -> Vec<Line<'static>> {
    let available = usize::from(width.saturating_sub(2)).max(1);
    let mut rows = Vec::new();
    let source = if is_user {
        text.lines().map(|s| Line::raw(s.to_owned())).collect()
    } else {
        message_lines(text)
    };
    for line in source {
        let mut row = Vec::new();
        let mut used: usize = 0;
        for span in line.spans {
            for word in span.content.split_word_bounds() {
                if used > 0
                    && used.saturating_add(word.width()) > available
                    && !word.chars().all(char::is_whitespace)
                {
                    rows.push(Line::from(std::mem::take(&mut row)).style(line.style));
                    used = 0;
                }
                for grapheme in word.graphemes(true) {
                    let cells = grapheme.width();
                    if used.saturating_add(cells) > available {
                        rows.push(Line::from(std::mem::take(&mut row)).style(line.style));
                        used = 0;
                        if grapheme.chars().all(char::is_whitespace) {
                            continue;
                        }
                    }
                    row.push(Span::styled(grapheme.to_owned(), span.style));
                    used = used.saturating_add(cells);
                }
            }
        }
        rows.push(Line::from(row).style(line.style));
    }
    let style = if is_user {
        Style::new().fg(Color::White).bg(Color::Rgb(34, 39, 48))
    } else {
        base
    };
    rows.into_iter()
        .enumerate()
        .map(|(i, mut line)| {
            let mut spans = vec![Span::styled(
                if i == 0 {
                    marker.to_owned()
                } else {
                    "  ".into()
                },
                if is_user { accent() } else { base },
            )];
            spans.append(&mut line.spans);
            if is_user {
                let occupied: usize = spans.iter().map(Span::width).sum();
                spans.push(Span::raw(
                    " ".repeat(usize::from(width).saturating_sub(occupied)),
                ));
            }
            Line::from(spans).style(style.patch(line.style))
        })
        .collect()
}

fn message_lines(text: &str) -> Vec<Line<'static>> {
    let mut code = false;
    text.lines()
        .map(|line| {
            if line.starts_with("```") {
                code = !code;
                return Line::styled(
                    if code {
                        format!("  {}", line.trim_start_matches('`'))
                    } else {
                        String::new()
                    },
                    muted(),
                );
            }
            if code {
                Line::styled(format!("  {line}"), Style::new().fg(ACCENT))
            } else if let Some(heading) = line
                .strip_prefix("# ")
                .or_else(|| line.strip_prefix("## "))
                .or_else(|| line.strip_prefix("### "))
            {
                Line::styled(heading.to_owned(), Style::new().bold())
            } else {
                Line::raw(line.to_owned())
            }
        })
        .collect()
}

#[cfg(test)]
mod conversation_tests {
    use super::*;

    #[test]
    fn wrapped_messages_keep_the_gutter_and_unicode_graphemes() {
        let text = "hello 確認👩‍💻 e\u{301} abcdefghijklmnopqrstuvwxyz";
        for width in [10, 20, 40] {
            let lines = bubble_lines(text, "› ", true, width, Style::new());
            assert!(lines.iter().all(|line| line.width() <= usize::from(width)));
            let rendered: Vec<_> = lines.iter().map(ToString::to_string).collect();
            assert!(rendered.first().is_some_and(|line| line.starts_with("› ")));
            assert!(rendered.iter().skip(1).all(|line| line.starts_with("  ")));
            assert!(rendered.iter().any(|line| line.contains("👩‍💻")));
            assert!(rendered.iter().any(|line| line.contains("e\u{301}")));
        }
    }
}

fn draw_tool_lines(
    lines: &mut Vec<Line<'static>>,
    tool: &ToolLine,
    expanded: bool,
    busy: bool,
    width: u16,
) {
    let (symbol, color) = if tool.error {
        ("×", Color::Red)
    } else if tool.result.is_some() {
        ("✓", Color::Green)
    } else if busy {
        ("›", ACCENT)
    } else {
        ("–", MUTED)
    };
    let args: Option<serde_json::Value> = serde_json::from_str(&tool.arguments).ok();
    let detail = args
        .as_ref()
        .and_then(|value| value.get("path").or_else(|| value.get("command")))
        .and_then(serde_json::Value::as_str)
        .map(visible)
        .unwrap_or_default();
    let summary = if tool.name == "bash" {
        format!("$ {}", detail.lines().next().unwrap_or_default())
    } else if detail.is_empty() {
        tool.name.clone()
    } else {
        format!(
            "{}  {}",
            tool.name,
            detail.lines().next().unwrap_or_default()
        )
    };
    lines.push(Line::from(vec![
        Span::styled(format!("  {symbol} "), Style::new().fg(color)),
        Span::styled(
            ellipsize(&summary, usize::from(width.saturating_sub(4))),
            muted(),
        ),
    ]));
    if tool.name == "bash" {
        if expanded {
            lines.extend(
                detail
                    .lines()
                    .map(|line| Line::styled(format!("    $ {line}"), muted())),
            );
        }
        draw_command_output(lines, tool, expanded);
    } else if expanded || tool.error {
        if expanded {
            lines.push(Line::styled(
                format!("    args: {}", tool.arguments),
                muted(),
            ));
        }
        if let Some(result) = &tool.result {
            lines.extend(result.lines().map(|line| {
                Line::styled(
                    format!("    {line}"),
                    Style::new().fg(if tool.error { Color::Red } else { MUTED }),
                )
            }));
        }
        lines.push(Line::default());
    }
}

fn draw_command_output(lines: &mut Vec<Line<'static>>, tool: &ToolLine, expanded: bool) {
    let result: Option<serde_json::Value> = tool
        .result
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());
    let stdout = result
        .as_ref()
        .and_then(|v| v.get("stdout"))
        .and_then(serde_json::Value::as_str)
        .filter(|_| !tool.truncated)
        .map_or_else(|| tool.stdout.clone(), visible);
    let stderr = result
        .as_ref()
        .and_then(|v| v.get("stderr"))
        .and_then(serde_json::Value::as_str)
        .filter(|_| !tool.truncated)
        .map_or_else(|| tool.stderr.clone(), visible);
    let mut output: Vec<_> = stdout
        .lines()
        .map(|s| (s, muted()))
        .chain(stderr.lines().map(|s| (s, Style::new().fg(Color::Red))))
        .collect();
    let hidden = if expanded {
        0
    } else {
        output.len().saturating_sub(6)
    };
    if hidden > 0 {
        lines.push(Line::styled(
            format!("    … {hidden} earlier lines · ctrl+t to expand"),
            muted(),
        ));
        output.drain(..hidden);
    }
    lines.extend(
        output
            .into_iter()
            .map(|(text, style)| Line::styled(format!("    {text}"), style)),
    );
    if let Some(result) = &tool.result {
        let status = result_status(result);
        lines.push(Line::styled(
            format!("    {status}"),
            Style::new().fg(if tool.error { Color::Red } else { MUTED }),
        ));
    }
    if tool.truncated
        || result
            .as_ref()
            .and_then(|v| v.get("output_truncated"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    {
        lines.push(Line::styled(
            "    [Output truncated at capture limit]",
            muted(),
        ));
    }
    lines.push(Line::default());
}

fn result_status(result: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(result) {
        if let Some(code) = value.get("exit_code").and_then(serde_json::Value::as_i64) {
            return format!("exit {code}");
        }
        if let Some(status) = value.get("status").and_then(serde_json::Value::as_str) {
            return visible(status);
        }
    }
    visible(result)
}

fn ellipsize(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    let mut result = String::new();
    let mut used: usize = 0;
    for grapheme in text.graphemes(true) {
        let next = used.saturating_add(grapheme.width());
        if next > width.saturating_sub(1) {
            break;
        }
        result.push_str(grapheme);
        used = next;
    }
    if width > 0 {
        result.push('…');
    }
    result
}

fn draw_input(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let input = &app.instruction;
    let parts = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(area);
    let editor = parts.first().copied().unwrap_or_default();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(" Your message ");
    let inner = block.inner(editor);
    frame.render_widget(block, editor);
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
        if input.text().is_empty() {
            let placeholder = Rect::new(
                inner.x.saturating_add(2),
                inner.y,
                inner.width.saturating_sub(2),
                1,
            );
            frame.render_widget(
                Paragraph::new("Describe your app or ask a question…").style(muted()),
                placeholder,
            );
        }
        frame.set_cursor_position((
            inner
                .x
                .saturating_add(u16::try_from(view.column).unwrap_or(0)),
            inner
                .y
                .saturating_add(u16::try_from(view.row.saturating_sub(scroll)).unwrap_or(0)),
        ));
    }
    let hints = if let Some(query) = input.search_query() {
        vec![
            Line::styled(
                format!(
                    "history search: {query}{}",
                    if input.search_found() {
                        ""
                    } else {
                        " [no match]"
                    }
                ),
                accent(),
            ),
            Line::styled(
                if input.search_found() {
                    "enter use match · esc cancel · ctrl+r older match"
                } else {
                    "enter / esc return to draft"
                },
                muted(),
            ),
        ]
    } else if app.evidence_open() {
        vec![Line::styled(
            "ctrl+g / esc back · wheel / PgUp/PgDn evidence · /contract reviews acceptance",
            muted(),
        )]
    } else if app.help_open() {
        vec![Line::styled(
            "esc close help · wheel / PgUp/PgDn scroll",
            muted(),
        )]
    } else {
        vec![
            Line::styled(
                "enter send · shift+enter newline · ctrl+t transcript · ctrl+o copy",
                muted(),
            ),
            Line::styled(
                "ctrl+g evidence · / commands · ? shortcuts · wheel / PgUp/PgDn scroll",
                muted(),
            ),
        ]
    };
    frame.render_widget(
        Paragraph::new(hints),
        parts.get(1).copied().unwrap_or_default(),
    );
}

fn approval_visible(app: &App) -> bool {
    app.review_area.width > 3
        && app.review_rows > 0
        && app.review_rows <= app.review_area.height.saturating_sub(2)
}

fn activity_line(app: &App) -> Line<'static> {
    // Advance from elapsed time, not input events or token arrival. The main
    // loop redraws every 100ms even while the model or a tool is silent.
    let elapsed = app
        .turn_started
        .map(|start| start.elapsed())
        .unwrap_or_default();
    let ticks = elapsed.as_millis().saturating_div(100);
    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let index = usize::try_from(ticks.checked_rem(10).unwrap_or(0)).unwrap_or(0);
    let symbol = spinner.get(index).copied().unwrap_or("⠋");
    let stopping = app
        .processes
        .flag()
        .load(std::sync::atomic::Ordering::SeqCst);
    let label = if stopping {
        "Stopping · waiting for current work"
    } else {
        app.activity.as_deref().unwrap_or("Working")
    };
    Line::from(vec![
        Span::styled(
            format!("{symbol} {label}"),
            if stopping {
                Style::new().fg(Color::Yellow)
            } else {
                accent()
            },
        ),
        Span::styled(format!("  ·  {}s elapsed", elapsed.as_secs()), muted()),
    ])
}

fn draw_controls(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if app.screen == Screen::Instructing && app.evidence_open() {
        frame.render_widget(
            Paragraph::new("ctrl+g / esc back · wheel / PgUp/PgDn evidence").style(muted()),
            area,
        );
        return;
    }
    if is_review(&app.screen) && !approval_visible(app) {
        frame.render_widget(
            Paragraph::new("Approval unavailable:\nEnlarge terminal. n: back")
                .style(Style::new().fg(Color::Yellow))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let lines = match app.screen {
        Screen::Picking => vec![Line::from(vec![
            Span::styled(" enter ", Style::new().fg(Color::Black).bg(ACCENT).bold()),
            Span::raw(" Continue  "),
            Span::styled("↑↓ choose · q / esc quit", muted()),
        ])],
        Screen::Running => vec![
            activity_line(app),
            Line::styled(
                if app.evidence_open() {
                    "ctrl+g / esc back · wheel / PgUp/PgDn evidence · ctrl-c cancel"
                } else {
                    "esc / ctrl-c cancel · ctrl+g evidence · ctrl+t transcript · wheel / PgUp/PgDn scroll"
                },
                muted(),
            ),
        ],
        Screen::ReviewingProcessor => actions("Allow for this session", "n / esc: Back"),
        Screen::ReviewingExport => actions("Send once", "n: Reject · esc: Back · ←/→: Bytes"),
        Screen::ReviewingContract => {
            let accepted = app
                .contract_review
                .as_ref()
                .is_some_and(|review| review.accepted);
            if accepted {
                actions("Return to conversation", "n / esc: Back")
            } else {
                actions("Accept specification", "n / esc: Reject")
            }
        }
        Screen::Instructing => return,
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn actions(action: &str, alternative: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled(
                format!(" y: {action} "),
                Style::new().fg(Color::Black).bg(ACCENT).bold(),
            ),
            Span::raw(format!("  {alternative}")),
        ]),
        Line::styled(
            "wheel / PgUp/PgDn details · Enter and paste do not approve.",
            muted(),
        ),
    ]
}

#[derive(Clone, Copy)]
struct Review<'a> {
    title: &'a str,
    description: &'a str,
    tag: &'a str,
    heading: &'a str,
    details: &'a str,
}

fn draw_review(frame: &mut Frame<'_>, app: &mut App, area: Rect, review: Review<'_>) {
    let intro = Paragraph::new(vec![
        Line::styled(review.title, Style::new().bold()),
        Line::styled(review.description, muted()),
        Line::default(),
    ])
    .wrap(Wrap { trim: false });
    let intro_rows = u16::try_from(intro.line_count(area.width))
        .unwrap_or(u16::MAX)
        .min(5);
    let heading = Paragraph::new(visible(review.heading)).wrap(Wrap { trim: false });
    // Count the actual wrapped content before adding the single-cell border.
    app.review_rows =
        u16::try_from(heading.line_count(area.width.saturating_sub(2))).unwrap_or(u16::MAX);
    let parts = Layout::vertical([
        Constraint::Length(intro_rows),
        Constraint::Length(app.review_rows.saturating_add(2)),
        Constraint::Min(3),
    ])
    .split(area);
    frame.render_widget(intro, parts.first().copied().unwrap_or_default());
    let fixed = parts.get(1).copied().unwrap_or_default();
    let details = parts.get(2).copied().unwrap_or_default();
    app.review_area = fixed;
    frame.render_widget(
        heading.block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(ACCENT))
                .title(format!(" {} ", review.tag)),
        ),
        fixed,
    );
    render_scrolled(
        frame,
        app,
        Paragraph::new(visible(review.details))
            .style(muted())
            .wrap(Wrap { trim: false }),
        details,
        false,
    );
}

fn draw_processor_review(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let Some(gateway) = &app.gateway else {
        return;
    };
    let provider = gateway.provider();
    let heading = format!(
        "Destination: {}\nConnection: {}\nModel: {}\nCredential: {}\nScope: workspace conversation, files and tool results; one session, up to eight hours.",
        gateway.endpoint(), visible(&provider.id).replace('\n', "⏎"), visible(&provider.model).replace('\n', "⏎"),
        if provider.api_key.is_some() { "host-configured account" } else { "none" });
    let title = format!(
        "Allow {} to receive workspace context?",
        visible(&provider.label).replace('\n', "⏎")
    );
    let details = format!(
        "\nAccount configuration: {}\n\nCode execution and proof checks stay offline.",
        visible(&provider.note).replace('\n', "⏎")
    );
    draw_review(
        frame,
        app,
        area,
        Review {
            title: &title,
            description:
                "Your messages, file contents and tool results will be shared with this connection.",
            tag: "Connection details",
            heading: &heading,
            details: &details,
        },
    );
}

fn draw_contract_review(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let Some(review) = &app.contract_review else {
        return;
    };
    let heading = format!(
        "Contract revision: {}\nContract SHA-256: {}",
        review.reference.revision, review.reference.contract
    );
    let details = format!(
        "\n{}{}\n\nAccepting continues development under this exact contract.",
        assurance::changes(review),
        review.review
    );
    let title = if review.accepted {
        "Accepted project specification"
    } else {
        "Review the proposed specification"
    };
    draw_review(
        frame,
        app,
        area,
        Review {
            title,
            description: "Check the intended behavior and assumptions before continuing.",
            tag: "Specification identity",
            heading: &heading,
            details: &details,
        },
    );
}

fn draw_export_review(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let Some(entry) = app
        .reviewing_export
        .as_ref()
        .and_then(|reference| app.exports.get(reference).ok())
    else {
        return;
    };
    let (Ok(summary), Ok(preview)) = (entry.summary(), entry.preview(app.export_offset)) else {
        return;
    };
    let heading = format!("Export #{}\nDestination: {}\nMethod: POST · Content-Type: application/octet-stream\nFrozen body: {} bytes\nAccepted contract: {}\nAttempt: one POST; no redirects or automatic retries", summary.reference.id, summary.target, summary.bytes, summary.contract.as_deref().unwrap_or("ordinary file; no project contract"));
    let details = format!("State: {}\nSource name (untrusted): {}\nByte offset: {}\n\n{}\nThese exact payload bytes are retained outside the worker. Current project source and evidence are checked again before dispatch.", summary.state, visible(&summary.source).replace('\n', "⏎"), app.export_offset, preview);
    draw_review(
        frame,
        app,
        area,
        Review {
            title: "Send this artifact to the destination below?",
            description:
                "Approval sends these frozen bytes once. Review the payload with PgUp/PgDn.",
            tag: "Delivery details",
            heading: &heading,
            details: &details,
        },
    );
}
