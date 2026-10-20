//! Centered commit dialog with staged review and message editing.
use super::pane_block;
use crate::{
    app::App,
    commit_composer::{Editor, Field},
};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let width = area.width.saturating_sub(4).min(112);
    let height = area.height.saturating_sub(4).min(28);
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    app.workspace.composer.rect = rect;
    frame.render_widget(Clear, rect);
    let busy = app.workspace.busy();
    let title = if busy {
        "Committing…".to_string()
    } else {
        format!(
            "{} · {}",
            if let Some((_, only)) = app.workspace.composer.amend.as_ref() {
                if *only {
                    "Reword HEAD"
                } else {
                    "Amend HEAD"
                }
            } else {
                "Commit"
            },
            app.repo.head_name().unwrap_or_else(|| "HEAD".into())
        )
    };
    frame.render_widget(pane_block(&title, true), rect);
    let content = rect.inner(Margin::new(1, 1));
    let footer = Rect::new(
        content.x,
        content.bottom().saturating_sub(2),
        content.width,
        2.min(content.height),
    );
    let content = Rect::new(
        content.x,
        content.y,
        content.width,
        content.height.saturating_sub(2),
    );
    let parts = if content.width >= 86 {
        Layout::horizontal([Constraint::Percentage(52), Constraint::Percentage(48)]).split(content)
    } else {
        Layout::vertical([
            Constraint::Min(5),
            Constraint::Length((content.height / 3).max(3)),
        ])
        .split(content)
    };
    let inner = parts[0];
    app.workspace.composer.diff_rect = parts[1];
    app.workspace.diff_height = parts[1].height.saturating_sub(3);
    let path = app
        .workspace
        .file_state
        .selected()
        .and_then(|i| app.workspace.files.get(i))
        .map(|f| f.path.as_path());
    super::patch::draw(
        frame,
        parts[1],
        "STAGED diff",
        path,
        &app.workspace.patch,
        super::patch::Viewport {
            scroll: app.workspace.scroll,
            ..Default::default()
        },
    );
    let field = app.workspace.composer.field;
    let compact = inner.height < 14;
    let error_height = if app.workspace.commit_error.is_some() {
        (inner.height / 3).clamp(2, 6)
    } else {
        0
    };
    let list_height = if compact {
        if field == Field::Review {
            inner.height.saturating_sub(3 + error_height)
        } else {
            0
        }
    } else {
        7
    };
    let rows = Layout::vertical([
        Constraint::Length(list_height),
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(error_height),
    ])
    .split(inner);
    app.workspace.composer.fields = [rows[1], rows[2], rows[0]];
    app.workspace.composer.error_rect = rows[3];
    let c = &mut app.workspace.composer;
    let visible = rows[0].height.saturating_sub(2) as usize;
    if c.selected < c.list_top {
        c.list_top = c.selected;
    }
    if visible > 0 && c.selected >= c.list_top + visible {
        c.list_top = c.selected + 1 - visible;
    }
    let staged: Vec<Line> = c
        .staged
        .iter()
        .enumerate()
        .skip(c.list_top)
        .map(|(position, index)| {
            let file = &app.workspace.files[*index];
            Line::from(vec![
                Span::styled(
                    if position == c.selected { "› " } else { "  " },
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{} ", file.index_status),
                    Style::default().fg(super::workspace::change_color(file.index_status)),
                ),
                Span::raw(file.path.to_string_lossy().into_owned()),
                Span::styled(
                    if file.unstaged { " · partial" } else { "" },
                    Style::default().fg(Color::Yellow),
                ),
            ])
            .style(if position == c.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            })
        })
        .collect();
    let count = c.staged.len();
    frame.render_widget(
        Paragraph::new(staged).block(pane_block(
            &format!("Staged · {count} files"),
            field == Field::Review,
        )),
        rows[0],
    );
    let len = c.subject.text.chars().count();
    let subject_title = format!(
        "Subject · {len}/72{}",
        if len > 72 { " · long" } else { "" }
    );
    editor(
        frame,
        &mut c.subject,
        rows[1],
        &subject_title,
        field == Field::Subject && !busy,
        "Describe this change",
    );
    editor(
        frame,
        &mut c.body,
        rows[2],
        "Body (optional)",
        field == Field::Body && !busy,
        "Why is this change needed?",
    );
    if let Some(error) = &app.workspace.commit_error {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .scroll((app.workspace.composer.error_scroll, 0))
                .wrap(ratatui::widgets::Wrap { trim: false })
                .block(pane_block("Git output · Alt-j/k scroll", true))
                .style(Style::default().fg(Color::LightRed)),
            rows[3],
        );
    }

    let hint = match field {
        Field::Subject => "Enter commit · Tab body · Shift-Tab review",
        Field::Body => "Enter newline · Tab review · Ctrl-s commit",
        Field::Review => "j/k review · Space unstage · Enter write message",
    };
    frame.render_widget(Clear, footer);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                if busy {
                    "Running Git hooks / signing…"
                } else {
                    hint
                },
                Style::default().fg(Color::Cyan),
            ),
            Line::from("Ctrl-d/u scroll diff · Esc keep draft · c resume"),
        ]),
        footer,
    );
}

fn editor(
    frame: &mut Frame,
    editor: &mut Editor,
    rect: Rect,
    title: &str,
    focused: bool,
    placeholder: &str,
) {
    if rect.width < 3 || rect.height < 3 {
        return;
    }
    let inner = rect.inner(Margin::new(1, 1));
    let (row, col) = editor.row_col();
    let line = editor.text.split('\n').nth(row).unwrap_or("");
    let prefix: String = line.chars().take(col).collect();
    let column = prefix.width();
    if row < editor.top {
        editor.top = row;
    }
    if row >= editor.top + inner.height as usize {
        editor.top = row + 1 - inner.height as usize;
    }
    if column < editor.left {
        editor.left = column;
    }
    if column >= editor.left + inner.width as usize {
        editor.left = column + 1 - inner.width as usize;
    }
    let text = if editor.text.is_empty() {
        placeholder
    } else {
        editor.text.as_str()
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(if editor.text.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            })
            .scroll((
                editor.top.min(u16::MAX as usize) as u16,
                editor.left.min(u16::MAX as usize) as u16,
            ))
            .block(pane_block(title, focused)),
        rect,
    );
    if focused {
        let x = inner.x + (column - editor.left) as u16;
        let y = inner.y + (row - editor.top) as u16;
        // Keep a visible caret in debug captures as well as the live terminal.
        let cell = &mut frame.buffer_mut()[(x, y)];
        cell.set_style(Style::default().add_modifier(Modifier::REVERSED));
        frame.set_cursor_position((x, y));
    }
}
