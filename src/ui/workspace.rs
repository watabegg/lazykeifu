use super::{
    dialog::{ConfirmDialog, InputDialog},
    graph_view::GraphViewWidget,
    pane_block,
};
use crate::{
    app::{App, AppMode, InputAction, LayoutMap},
    workspace::Panel,
    workspace_tabs::{FilesTab, RefsTab},
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    app.workspace_tick();
    if app.workspace.tabs.files != FilesTab::Files || app.workspace.tabs.refs != RefsTab::Local {
        if let Err(error) = app.workspace_load_tabs() {
            app.set_message(error.to_string());
        }
    }
    app.workspace.tabs.file_hits.clear();
    app.workspace.tabs.ref_hits.clear();
    let area = frame.area();
    app.layout = LayoutMap::default();
    app.workspace.status_rect = Rect::default();
    app.workspace.branches_rect = Rect::default();
    app.workspace.stash_rect = Rect::default();
    app.workspace.detail_rect = Rect::default();
    app.workspace.help_rect = Rect::default();
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
            Some(Panel::CommitDetail)
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
                Panel::CommitDetail => app.workspace.detail_rect = rows[1],
                _ => {}
            }
        } else {
            app.layout.graph = body;
        }
    } else {
        let sidebar_width = (area.width * 2 / 5).clamp(36, 64);
        let columns = Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(40)])
            .split(body);
        // Stable proportions: repository contents and tabs must not move pane boundaries.
        let file_height = body.height / 2;
        let stash_height = (body.height / 5).max(3);
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
            let right = Layout::vertical([
                Constraint::Min(5),
                Constraint::Length((body.height / 3).clamp(7, 12)),
            ])
            .split(columns[1]);
            app.layout.graph = right[0];
            app.workspace.detail_rect = right[1];
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
    let rows: Vec<ListItem> = app
        .workspace
        .tree
        .nodes
        .iter()
        .map(|node| {
            let indent = "  ".repeat(node.depth);
            let name = node.path.file_name().unwrap_or_default().to_string_lossy();
            let Some(index) = node.file else {
                let marker = if app.workspace.tree.closed.contains(&node.path) {
                    "▸"
                } else {
                    "▾"
                };
                return ListItem::new(Line::styled(
                    format!("   {indent}{marker} {name}/"),
                    Style::default().fg(Color::Cyan),
                ));
            };
            let Some(row) = app.workspace.files.get(index) else {
                return ListItem::new("");
            };
            let history = app.workspace.commit.is_some();
            let status_color = |status| {
                if row.conflict {
                    Color::LightRed
                } else {
                    change_color(status)
                }
            };
            let index_color = status_color(row.index_status);
            let worktree_color = status_color(row.worktree_status);
            let path_color = if row.conflict {
                Color::Red
            } else if !history && row.staged && !row.unstaged {
                Color::Green
            } else {
                Color::Reset
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    row.index_status.to_string(),
                    Style::default().fg(index_color),
                ),
                Span::styled(
                    row.worktree_status.to_string(),
                    Style::default().fg(worktree_color),
                ),
                Span::styled(format!(" {indent}{name}"), Style::default().fg(path_color)),
            ]))
        })
        .collect();
    frame.render_stateful_widget(
        List::new(rows)
            .block(pane_block("", focus == Panel::Files))
            .highlight_style(selection_style(focus == Panel::Files))
            .highlight_symbol(if focus == Panel::Files { "› " } else { "  " }),
        app.layout.files,
        &mut app.workspace.tree.state,
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
        .filter(|b| !b.is_remote)
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
            .block(pane_block("", focus == Panel::Branches))
            .highlight_style(selection_style(focus == Panel::Branches))
            .highlight_symbol(if focus == Panel::Branches {
                "› "
            } else {
                "  "
            }),
        app.workspace.branches_rect,
        &mut app.workspace.branch_state,
    );
    for (panel, rect) in [
        (Panel::Files, app.layout.files),
        (Panel::Branches, app.workspace.branches_rect),
    ] {
        if let Some(slot) = app.workspace.tabs.slot(panel) {
            let entries = &app.workspace.tabs.rows[slot];
            let rows: Vec<ListItem> = entries
                .iter()
                .map(|row| {
                    let content = vec![Line::from(row.label.clone())];
                    ListItem::new(content)
                })
                .collect();
            frame.render_widget(Clear, rect);
            frame.render_stateful_widget(
                List::new(rows)
                    .block(pane_block("", focus == panel))
                    .highlight_style(selection_style(focus == panel))
                    .highlight_symbol(if focus == panel { "› " } else { "  " }),
                rect,
                &mut app.workspace.tabs.states[slot],
            );
            if entries.is_empty() {
                let empty = [
                    "No worktrees",
                    "No submodules",
                    "No remote branches",
                    "No tags",
                ][slot];
                frame.render_widget(
                    Paragraph::new(empty).style(Style::default().fg(Color::Gray)),
                    rect.inner(ratatui::layout::Margin::new(1, 1)),
                );
            }
        }
    }
    if app.workspace.tabs.files == FilesTab::Files {
        if let Some(oid) = app.workspace.commit {
            frame.render_widget(
                pane_block("", focus == Panel::Files).title_bottom(Line::styled(
                    format!(" {:.7} · read-only ", oid),
                    Style::default().fg(Color::Gray),
                )),
                app.layout.files,
            );
        }
    }
    render_tabs(
        frame,
        "2",
        &["Files", "Worktrees", "Submodules"],
        app.workspace.tabs.files as usize,
        focus == Panel::Files,
        app.layout.files,
        &mut app.workspace.tabs.file_hits,
    );
    render_tabs(
        frame,
        "3",
        &["Local", "Remotes", "Tags"],
        app.workspace.tabs.refs as usize,
        focus == Panel::Branches,
        app.workspace.branches_rect,
        &mut app.workspace.tabs.ref_hits,
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
    if app.workspace.detail_rect.height > 0 {
        let rect = app.workspace.detail_rect;
        let widget = super::commit_detail::CommitDetailWidget::new(app)
            .with_focus(focus == Panel::CommitDetail);
        let max = widget
            .estimated_height(rect.width.saturating_sub(2))
            .saturating_sub(rect.height.saturating_sub(2));
        app.detail_scroll = app.detail_scroll.min(max);
        frame.render_widget(widget.with_scroll(app.detail_scroll), rect);
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
    let context = super::context_keys::line(app);
    let recovery_hint = match crate::git::recovery::kind(&app.repo.repo) {
        Some(crate::git::recovery::Kind::Merge) => Some("MERGE"),
        Some(crate::git::recovery::Kind::Rebase) => Some("REBASE"),
        Some(crate::git::recovery::Kind::CherryPick) => Some("CHERRY-PICK"),
        Some(crate::git::recovery::Kind::Revert) => Some("REVERT"),
        None => None,
    };
    let message = if let Some(kind) = recovery_hint {
        format!(
            "{kind}: C continue · A abort · e edit conflict · {}",
            app.get_message().unwrap_or("? help")
        )
    } else {
        app.get_message()
            .unwrap_or(
                "Tab / h / l focus · 2 files · 3 refs · 4 graph · 5 stash · 6 detail · ? help",
            )
            .to_string()
    };
    frame.render_widget(
        Paragraph::new(vec![
            context,
            Line::styled(message, Style::default().fg(Color::Gray)),
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
        AppMode::Input {
            action: InputAction::CommitMessage,
            ..
        } => {
            super::commit_composer::draw(frame, app);
        }
        AppMode::Input { title, input, .. } => {
            let rect = super::centered_rect(80, 30, area);
            frame.render_widget(InputDialog::new(title, input), rect);
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
        let width = area.width.saturating_sub(4).min(88);
        let height = area.height.saturating_sub(4).min(24);
        let rect = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        app.workspace.help_rect = rect;
        frame.render_widget(Clear, rect);
        let lines = super::workspace_help::lines(rect.width.saturating_sub(2) as usize);
        let max_scroll = lines
            .len()
            .saturating_sub(rect.height.saturating_sub(2) as usize)
            .min(u16::MAX as usize) as u16;
        let help =
            Paragraph::new(lines).block(pane_block("Help", true).title_bottom(Line::from(vec![
                Span::styled(" j/k ↑↓ ", Style::default().fg(Color::Cyan)),
                Span::raw("scroll · "),
                Span::styled("Esc / ?", Style::default().fg(Color::Cyan)),
                Span::raw(" close "),
            ])));
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

/// Keep the change-kind palette consistent with keifu's file list.
pub(super) fn change_color(status: char) -> Color {
    match status {
        'A' => Color::Green,
        'M' | 'T' => Color::Yellow,
        'D' => Color::Red,
        'R' | 'C' => Color::Cyan,
        'U' => Color::LightRed,
        '?' => Color::Magenta,
        _ => Color::Reset,
    }
}

// Fixed-width tab slots: selecting a tab never moves its neighbours. The same
// clipped rectangle is used to draw each label and to handle pointer input.
#[allow(clippy::too_many_arguments)]
fn render_tabs(
    frame: &mut Frame,
    number: &str,
    labels: &[&str],
    active: usize,
    focused: bool,
    rect: Rect,
    hits: &mut Vec<(Rect, usize)>,
) {
    hits.clear();
    if rect.width < 3 || rect.height == 0 {
        return;
    }
    let end = rect.right() - 1;
    let prefix = format!(" {number} ");
    let mut x = rect.x + 1;
    frame.render_widget(
        Paragraph::new(prefix.clone()).style(Style::default().fg(Color::Gray)),
        Rect::new(x, rect.y, (prefix.len() as u16).min(end - x), 1),
    );
    x += prefix.len() as u16;
    for (index, label) in labels.iter().enumerate() {
        if x >= end {
            break;
        }
        let text = if index == active {
            format!("[{label}]")
        } else {
            format!(" {label} ")
        };
        let width = (text.len() as u16).min(end - x);
        let target = Rect::new(x, rect.y, width, 1);
        let style = if index == active {
            Style::default().fg(Color::Cyan).add_modifier(if focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            })
        } else {
            Style::default().fg(Color::Gray)
        };
        frame.render_widget(Paragraph::new(text).style(style), target);
        hits.push((target, index));
        x += width;
    }
}
