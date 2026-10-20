use super::{
    dialog::{ConfirmDialog, InputDialog},
    graph_view::GraphViewWidget,
    pane_block,
};
use crate::{
    app::{App, AppMode, InputAction, LayoutMap},
    workspace::Panel,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    app.workspace_tick();
    let area = frame.area();
    app.layout = LayoutMap::default();
    app.workspace.status_rect = Rect::default();
    app.workspace.branches_rect = Rect::default();
    app.workspace.stash_rect = Rect::default();
    app.status_hints.clear();
    if area.width < 20 || area.height < 10 {
        frame.render_widget(Paragraph::new("Terminal too small (20 × 10 minimum)"), area);
        return;
    }
    let header = Rect::new(area.x, area.y, area.width, 1);
    let body = Rect::new(area.x, area.y + 1, area.width, area.height - 3);
    let footer = Rect::new(area.x, body.bottom(), area.width, 2);
    app.layout.status_bar = footer;
    app.workspace.status_rect = header;
    let focus = app.workspace.focus;
    if area.width < 80 || area.height < 18 {
        // The graph never disappears: compact terminals reserve a lower region
        // for the current list or explicitly opened diff.
        let auxiliary = if focus == Panel::Graph && app.workspace.diff_visible {
            Some(Panel::Diff)
        } else if matches!(focus, Panel::Graph | Panel::Status) {
            None
        } else {
            Some(focus)
        };
        if let Some(panel) = auxiliary {
            let rows = Layout::vertical([
                Constraint::Percentage(if panel == Panel::Diff { 40 } else { 50 }),
                Constraint::Min(3),
            ])
            .split(body);
            app.layout.graph = rows[0];
            match panel {
                Panel::Files => app.layout.files = rows[1],
                Panel::Branches => app.workspace.branches_rect = rows[1],
                Panel::Stashes => app.workspace.stash_rect = rows[1],
                Panel::Diff => app.layout.commit_detail = rows[1],
                _ => {}
            }
        } else {
            app.layout.graph = body;
        }
    } else {
        let sidebar_width = (area.width / 4).clamp(28, 38);
        let columns = Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(40)])
            .split(body);
        let file_height = if app.workspace.files.is_empty() {
            3
        } else {
            (app.workspace
                .files
                .len()
                .saturating_add(2)
                .min(u16::MAX as usize) as u16)
                .clamp(4, (body.height / 2).max(4))
        };
        let stash_height = if app.workspace.stashes.is_empty() {
            3
        } else {
            (app.workspace
                .stashes
                .len()
                .saturating_add(2)
                .min(u16::MAX as usize) as u16)
                .clamp(3, (body.height / 4).max(3))
        };
        let left = Layout::vertical([
            Constraint::Length(file_height),
            Constraint::Min(4),
            Constraint::Length(stash_height),
        ])
        .split(columns[0]);
        app.layout.files = left[0];
        app.workspace.branches_rect = left[1];
        app.workspace.stash_rect = left[2];
        if app.workspace.diff_visible {
            let right = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(columns[1]);
            app.layout.graph = right[0];
            app.layout.commit_detail = right[1];
        } else {
            app.layout.graph = columns[1];
        }
    }
    let repo_name = std::path::Path::new(&app.repo_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let branch = app
        .head_name
        .as_deref()
        .unwrap_or(if app.repo.repo.head().is_ok() {
            "detached HEAD"
        } else {
            "unborn HEAD"
        });
    let count = app.working_file_count();
    let status = match count {
        Some(0) => "clean".to_owned(),
        Some(count) => format!("{count} changed"),
        None => "changes present".to_owned(),
    };
    let operation = crate::git::recovery::kind(&app.repo.repo)
        .map(|k| {
            format!(
                " · {}",
                match k {
                    crate::git::recovery::Kind::Merge => "MERGING".to_owned(),
                    crate::git::recovery::Kind::Rebase => "REBASING".to_owned(),
                    _ => k.command().to_uppercase(),
                }
            )
        })
        .unwrap_or_default();
    let busy = if app.workspace.busy() || app.is_pushing() {
        " · running Git…"
    } else {
        ""
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {repo_name} "),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" / {branch}"), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!(" · {status}{operation}{busy}"),
                Style::default().fg(if operation.is_empty() {
                    Color::Gray
                } else {
                    Color::Yellow
                }),
            ),
        ]))
        .style(if focus == Panel::Status {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        }),
        header,
    );
    let files_title = match app.workspace.commit {
        Some(oid) => format!("2 Files · {} · {:.7}", app.workspace.files.len(), oid),
        None => format!("2 Working files · {}", app.workspace.files.len()),
    };
    let rows: Vec<ListItem> = app
        .workspace
        .files
        .iter()
        .map(|row| {
            let label = if app.workspace.commit.is_some() {
                format!("  {}", row.path.display())
            } else {
                format!(
                    "{}{} {}",
                    if row.conflict {
                        '!'
                    } else if row.staged {
                        '+'
                    } else {
                        ' '
                    },
                    if row.untracked {
                        '?'
                    } else if row.unstaged {
                        '*'
                    } else {
                        ' '
                    },
                    row.path.display()
                )
            };
            ListItem::new(label).style(if row.conflict {
                Style::default().fg(Color::Red)
            } else if row.staged && !row.unstaged {
                Style::default().fg(Color::Green)
            } else if row.untracked {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
        })
        .collect();
    frame.render_stateful_widget(
        List::new(rows)
            .block(pane_block(&files_title, focus == Panel::Files))
            .highlight_style(selection_style(focus == Panel::Files))
            .highlight_symbol(if focus == Panel::Files { "› " } else { "  " }),
        app.layout.files,
        &mut app.workspace.file_state,
    );
    if app.workspace.files.is_empty() {
        let text = if app.workspace.commit.is_some() {
            "No file changes"
        } else {
            "Clean working tree"
        };
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(Color::Gray)),
            app.layout.files.inner(ratatui::layout::Margin::new(1, 1)),
        );
    }
    let rows: Vec<ListItem> = app
        .branches
        .iter()
        .map(|b| {
            ListItem::new(format!(
                "{} {}",
                if b.is_head { '*' } else { ' ' },
                super::graph_view::truncate_with_ellipsis(
                    &b.name,
                    app.workspace.branches_rect.width.saturating_sub(6) as usize
                )
            ))
            .style(if b.is_head {
                Style::default().fg(Color::Green)
            } else if b.is_remote {
                Style::default().fg(Color::Gray)
            } else {
                Style::default()
            })
        })
        .collect();
    if app.workspace.branch_state.selected().is_none() && !rows.is_empty() {
        app.workspace.branch_state.select(Some(0));
    }
    frame.render_stateful_widget(
        List::new(rows)
            .block(pane_block("3 Branches", focus == Panel::Branches))
            .highlight_style(selection_style(focus == Panel::Branches))
            .highlight_symbol(if focus == Panel::Branches {
                "› "
            } else {
                "  "
            }),
        app.workspace.branches_rect,
        &mut app.workspace.branch_state,
    );
    let stashes: Vec<ListItem> = app
        .workspace
        .stashes
        .iter()
        .map(|entry| ListItem::new(format!("{} {}", entry.selector, entry.message)))
        .collect();
    frame.render_stateful_widget(
        List::new(stashes)
            .block(pane_block("5 Stashes", focus == Panel::Stashes))
            .highlight_style(selection_style(focus == Panel::Stashes))
            .highlight_symbol(if focus == Panel::Stashes {
                "› "
            } else {
                "  "
            }),
        app.workspace.stash_rect,
        &mut app.workspace.stash_state,
    );
    if app.workspace.stashes.is_empty() {
        frame.render_widget(
            Paragraph::new("No stashes").style(Style::default().fg(Color::Gray)),
            app.workspace
                .stash_rect
                .inner(ratatui::layout::Margin::new(1, 1)),
        );
    }
    if app.layout.graph.width > 0 {
        frame.render_stateful_widget(
            GraphViewWidget::new(app, app.layout.graph.width),
            app.layout.graph,
            &mut app.graph_list_state,
        );
    }
    if focus == Panel::Graph && app.layout.graph.height >= 12 {
        super::render_branch_info_popup(frame, app, app.layout.graph);
    }
    let mut title = if let Some(oid) = app.workspace.stash_preview {
        format!("0 Diff · stash {} · read-only", &oid.to_string()[..7])
    } else if focus == Panel::Stashes {
        "0 Diff · no stash selected".into()
    } else if let Some(oid) = app.workspace.commit {
        format!("0 Diff · commit {}", &oid.to_string()[..7])
    } else {
        format!(
            "0 Diff · {}",
            if app.workspace.staged {
                "STAGED"
            } else {
                "UNSTAGED"
            }
        )
    };
    let hunks = crate::git::changes::hunks(&app.workspace.preview);
    let selected_hunk = hunks.get(app.workspace.hunk_index);
    if focus == Panel::Diff && !hunks.is_empty() {
        title.push_str(&format!(
            " · hunk {}/{}",
            app.workspace.hunk_index + 1,
            hunks.len()
        ));
    }
    if let Some(file) = app
        .workspace
        .file_state
        .selected()
        .and_then(|i| app.workspace.files.get(i))
    {
        if app.workspace.stash_preview.is_none() {
            title.push_str(&format!(" · {}", file.path.display()));
        }
    }
    app.workspace.diff_height = app.layout.commit_detail.height.saturating_sub(2);
    if app.layout.commit_detail.height > 0 {
        let lines: Vec<Line> = app
            .workspace
            .preview
            .lines()
            .enumerate()
            .map(|(index, line)| {
                let color = if line.starts_with('+') {
                    Color::Green
                } else if line.starts_with('-') {
                    Color::Red
                } else if line.starts_with("@@") {
                    Color::Cyan
                } else {
                    Color::Reset
                };
                let mut style = Style::default().fg(color);
                if focus == Panel::Diff && selected_hunk.is_some_and(|range| range.start == index) {
                    style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                }
                Line::styled(line.to_string(), style)
            })
            .collect();
        frame.render_widget(
            Paragraph::new(lines)
                .scroll((app.workspace.scroll, app.workspace.horizontal))
                .block(pane_block(&title, focus == Panel::Diff)),
            app.layout.commit_detail,
        );
    }
    let hints = match crate::git::recovery::kind(&app.repo.repo) {
        Some(crate::git::recovery::Kind::Merge) => {
            "MERGE: C continue · A abort · e edit file · Space stage resolution · 2 files · ? help"
        }
        Some(crate::git::recovery::Kind::Rebase) => {
            "REBASE: C continue · A abort · e edit file · Space stage resolution · 2 files · ? help"
        }
        Some(crate::git::recovery::Kind::CherryPick) => {
            "CHERRY-PICK: C continue · A abort · e edit file · Space stage resolution · 2 files · ? help"
        }
        Some(crate::git::recovery::Kind::Revert) => {
            "REVERT: C continue · A abort · e edit file · Space stage resolution · 2 files · ? help"
        }
        None => {
            "? help  Tab focus  2 files  3 branches  4 graph  5 stash  0 diff"
        }
    };
    let graph_hint = format!(
        "{} commits{} · Enter diff · y pick · V revert",
        app.commits.len(),
        if app.history_has_more {
            " · + load more"
        } else {
            " · all loaded"
        }
    );
    let message = app.get_message().unwrap_or(if focus == Panel::Stashes {
        "Space: apply · p: pop · d: drop · Enter: diff · s: save stash"
    } else if focus == Panel::Graph {
        &graph_hint
    } else if focus == Panel::Branches {
        "Enter checkout · M merge · r rebase · b create · d delete"
    } else if focus == Panel::Diff
        && app.workspace.commit.is_none()
        && app.workspace.stash_preview.is_none()
    {
        "Esc close diff · Space stage · [ / ] hunk · v side"
    } else if focus == Panel::Diff {
        "j/k scroll · H/L horizontal · Esc close diff"
    } else if app.workspace.commit.is_some() {
        "Enter diff · j/k file · 2 working files · Esc close diff"
    } else {
        "Space stage · Enter diff · v side · c commit · d discard"
    });
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(hints, Style::default().fg(Color::Gray)),
            Line::styled(message, Style::default().fg(Color::Cyan)),
        ]),
        footer,
    );
    match &app.mode {
        AppMode::Input {
            title,
            input,
            action: InputAction::Search,
        } => {
            let rect = super::bottom_rect(70, 12, area);
            frame.render_widget(
                super::search_dropdown::SearchDropdown::new(
                    input,
                    app.search_results(),
                    &app.branch_positions,
                    app.search_selection(),
                ),
                rect,
            );
            let _ = title;
        }
        AppMode::Input { title, input, .. } => {
            if let Some(error) = &app.workspace.commit_error {
                let rect = super::centered_rect(90, 80, area);
                frame.render_widget(Clear, rect);
                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(5), Constraint::Min(1)])
                    .split(rect);
                frame.render_widget(InputDialog::new(title, input), rows[0]);
                frame.render_widget(
                    Paragraph::new(error.as_str())
                        .wrap(Wrap { trim: false })
                        .block(pane_block("Git output", false)),
                    rows[1],
                );
            } else {
                let rect = super::centered_rect(80, 30, area);
                frame.render_widget(InputDialog::new(title, input), rect);
            }
        }
        AppMode::Confirm { message, action } => {
            let height = if matches!(
                action,
                crate::app::ConfirmAction::Integrate(_)
                    | crate::app::ConfirmAction::CommitOperation(_)
            ) {
                80
            } else {
                30
            };
            frame.render_widget(
                ConfirmDialog::new(message),
                super::centered_rect(80, height, area),
            );
        }
        AppMode::Error { message } => {
            let rect = super::centered_rect(85, 60, area);
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(format!("{message}\n\nEnter / Esc: close"))
                    .wrap(Wrap { trim: false })
                    .block(pane_block("Error", true)),
                rect,
            );
        }
        _ => {}
    }
    if app.workspace.help {
        let rect = super::centered_rect(90, 90, area);
        frame.render_widget(Clear, rect);
        let lines = wrap_help("j/k: scroll help · Esc: close\n1 status · 2 working files · 3 branches · 4 graph · 5 stashes · 0 diff\nTab / Shift-Tab / h / l: move focus\nj / k: select / scroll · Ctrl-d / Ctrl-u: page\nSpace: stage / unstage file; checkout branch\nEnter: open diff; graph → commit files + diff; branch → checkout\nv: staged / unstaged diff · H / L: horizontal scroll\nc: commit staged changes (Git hooks and signing)\na: stage all · u: unstage all (working files only)\nb: create branch · d: delete branch (branches only)\nBranches / Graph: M merge into current · r rebase current onto selected\nGraph: y cherry-pick · V revert · + load 500 more commits\n/: search branches · f: fetch · p: pull · P: push · R: refresh\no: remote visibility · t: tags\nEsc: close diff / working files · q: quit · ?: close help\n\nCommit files are read-only. Use 2 to return to working files.\ns: save stash including untracked files\n5 Stashes: Space apply · p pop · d drop · Enter diff\nFiles: d discards unstaged changes (confirmation; index kept)\nDiff: [ / ] select hunk · Space stage/unstage selected hunk\nHunks: modified UTF-8 regular files; other changes use file stage.\ne: edit working file using Git editor settings (wait for editor exit)\nGit operation: C continue · A abort (confirmation)\nConflicted file: Space marks current contents resolved (confirmation)", rect.width.saturating_sub(2) as usize);
        let max_scroll = lines
            .len()
            .saturating_sub(rect.height.saturating_sub(2) as usize)
            .min(u16::MAX as usize) as u16;
        let help = Paragraph::new(lines).block(pane_block("Help · j/k scroll", true));
        app.help_scroll = app.help_scroll.min(max_scroll);
        frame.render_widget(help.scroll((app.help_scroll, 0)), rect);
    }
}

fn selection_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn wrap_help(text: &str, width: usize) -> Vec<Line<'static>> {
    use unicode_width::UnicodeWidthStr;
    let mut lines = Vec::new();
    for source in text.lines() {
        let mut line = String::new();
        for word in source.split_whitespace() {
            if !line.is_empty() && line.width() + 1 + word.width() > width {
                lines.push(Line::from(std::mem::take(&mut line)));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(Line::from(line));
    }
    lines
}
