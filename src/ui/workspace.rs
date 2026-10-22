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
use unicode_width::UnicodeWidthStr;

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
        frame.render_widget(
            Paragraph::new(crate::tr!("Terminal too small (20 × 10 minimum)")),
            area,
        );
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
            crate::tr!("detached HEAD")
        } else {
            crate::tr!("unborn HEAD")
        });
    let count = app.working_file_count();
    let status = match count {
        Some(0) => crate::tr!("clean").to_owned(),
        Some(count) => crate::trf!("{count} changed", count = count),
        None => crate::tr!("changes present").to_owned(),
    };
    let operation = crate::git::recovery::kind(&app.repo.repo)
        .map(|k| {
            format!(
                " · {}",
                match k {
                    crate::git::recovery::Kind::Merge => crate::tr!("MERGING").to_owned(),
                    crate::git::recovery::Kind::Rebase => crate::tr!("REBASING").to_owned(),
                    crate::git::recovery::Kind::CherryPick => {
                        crate::tr!("CHERRY-PICK").to_owned()
                    }
                    crate::git::recovery::Kind::Revert => crate::tr!("REVERT").to_owned(),
                }
            )
        })
        .unwrap_or_default();
    let busy = if app.workspace.busy() || app.is_pushing() {
        crate::tr!(" · running Git…")
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
                crate::trf!(
                    " · {status}{operation}{busy}",
                    status = status,
                    operation = operation,
                    busy = busy
                ),
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
                    format!("{indent}{marker} {name}/"),
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
                Span::raw(indent),
                Span::styled(
                    row.index_status.to_string(),
                    Style::default().fg(index_color),
                ),
                Span::styled(
                    row.worktree_status.to_string(),
                    Style::default().fg(worktree_color),
                ),
                Span::styled(format!(" {name}"), Style::default().fg(path_color)),
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
            crate::tr!("No file changes")
        } else {
            crate::tr!("Clean working tree")
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
                    crate::tr!("No worktrees"),
                    crate::tr!("No submodules"),
                    crate::tr!("No remote branches"),
                    crate::tr!("No tags"),
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
                    crate::trf!(
                        " {id} · {read_only} ",
                        id = format!("{:.7}", oid),
                        read_only = crate::tr!("read-only")
                    ),
                    Style::default().fg(Color::Gray),
                )),
                app.layout.files,
            );
        }
    }
    render_tabs(
        frame,
        "2",
        &[
            crate::tr!("Files"),
            crate::tr!("Worktrees"),
            crate::tr!("Submodules"),
        ],
        app.workspace.tabs.files as usize,
        focus == Panel::Files,
        app.layout.files,
        &mut app.workspace.tabs.file_hits,
    );
    render_tabs(
        frame,
        "3",
        &[
            crate::tr!("Local"),
            crate::tr!("Remotes"),
            crate::tr!("Tags"),
        ],
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
            .block(pane_block(
                &crate::trf!("5 {stashes}", stashes = crate::tr!("Stashes")),
                focus == Panel::Stashes,
            ))
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
            Paragraph::new(crate::tr!("No stashes")).style(Style::default().fg(Color::Gray)),
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
        crate::trf!(
            "0 {diff} · {stash} {oid} · {read_only}",
            diff = crate::tr!("Diff"),
            stash = crate::tr!("stash"),
            oid = &oid.to_string()[..7],
            read_only = crate::tr!("read-only")
        )
    } else if focus == Panel::Stashes {
        crate::trf!(
            "0 {diff} · {no_stash_selected}",
            diff = crate::tr!("Diff"),
            no_stash_selected = crate::tr!("no stash selected")
        )
    } else if let Some(oid) = app.workspace.commit {
        crate::trf!(
            "0 {diff} · {commit} {oid}",
            diff = crate::tr!("Diff"),
            commit = crate::tr!("commit"),
            oid = &oid.to_string()[..7]
        )
    } else {
        crate::trf!(
            "0 {diff} · {stage_state}",
            diff = crate::tr!("Diff"),
            stage_state = if app.workspace.staged {
                crate::tr!("STAGED")
            } else {
                crate::tr!("UNSTAGED")
            }
        )
    };
    let hunks = crate::git::changes::hunks(&app.workspace.preview);
    if focus == Panel::Diff && !hunks.is_empty() {
        title.push_str(&crate::trf!(
            " · {hunk} {current}/{total}",
            hunk = crate::tr!("hunk"),
            current = app.workspace.hunk_index + 1,
            total = hunks.len()
        ));
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
    app.workspace.diff_height = app.layout.commit_detail.height.saturating_sub(3);
    if app.layout.commit_detail.height > 0 {
        let path = app
            .workspace
            .file_state
            .selected()
            .and_then(|i| app.workspace.files.get(i))
            .filter(|_| app.workspace.stash_preview.is_none())
            .map(|f| f.path.as_path());
        super::patch::draw(
            frame,
            app.layout.commit_detail,
            &title,
            path,
            &app.workspace.patch,
            super::patch::Viewport {
                scroll: app.workspace.scroll,
                horizontal: app.workspace.horizontal,
                selected: (focus == Panel::Diff).then_some(app.workspace.hunk_index),
                focused: focus == Panel::Diff,
            },
        );
    }
    let context = super::context_keys::line(app);
    let recovery_hint = match crate::git::recovery::kind(&app.repo.repo) {
        Some(crate::git::recovery::Kind::Merge) => Some(crate::tr!("MERGE")),
        Some(crate::git::recovery::Kind::Rebase) => Some(crate::tr!("REBASE")),
        Some(crate::git::recovery::Kind::CherryPick) => Some(crate::tr!("CHERRY-PICK")),
        Some(crate::git::recovery::Kind::Revert) => Some(crate::tr!("REVERT")),
        None => None,
    };
    let message = if let Some(kind) = recovery_hint {
        crate::trf!(
            "{kind}: m {continue_label} / {abort} · e {edit_conflict} · {message}",
            kind = kind,
            continue_label = crate::tr!("continue"),
            abort = crate::tr!("abort"),
            edit_conflict = crate::tr!("edit conflict"),
            message = app.get_message().unwrap_or(crate::tr!("? help"))
        )
    } else {
        if let Some(message) = app.get_message() {
            message.to_string()
        } else {
            crate::trf!(
                "Tab / h / l {focus} · 2 {files} · 3 {refs} · 4 {graph} · 5 {stash} · 6 {detail} · @ {extras} · ? {help}",
                focus = crate::tr!("focus"),
                files = crate::tr!("files"),
                refs = crate::tr!("refs"),
                graph = crate::tr!("graph"),
                stash = crate::tr!("stash"),
                detail = crate::tr!("detail"),
                extras = crate::tr!("extras"),
                help = crate::tr!("help")
            )
        }
    };
    frame.render_widget(
        Paragraph::new(vec![
            context,
            Line::styled(message, Style::default().fg(Color::Gray)),
        ]),
        footer,
    );
    if matches!(app.mode, AppMode::GitMenu(_)) {
        crate::git_actions::draw(frame, app);
    }
    match &app.mode {
        AppMode::Input {
            title,
            input,
            action: InputAction::Search,
        } => {
            let rect = super::bottom_rect(70, 12, area);
            app.layout.overlay = rect;
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
            app.layout.overlay = rect;
            frame.render_widget(InputDialog::new(title, input), rect);
        }
        AppMode::Confirm { message, .. } => {
            let width = (area.width * 4 / 5).clamp(1, 100);
            let lines = Paragraph::new(format!("  {message}"))
                .wrap(Wrap { trim: false })
                .line_count(width.saturating_sub(2).max(1));
            let height = (lines as u16 + 6).min(area.height.saturating_sub(2));
            let rect = Rect::new(
                (area.width - width) / 2,
                (area.height - height) / 2,
                width,
                height,
            );
            app.layout.overlay = rect;
            frame.render_widget(ConfirmDialog::new(message), rect);
        }
        AppMode::Error { message } => {
            let rect = super::centered_rect(85, 60, area);
            app.layout.overlay = rect;
            frame.render_widget(Clear, rect);
            frame.render_widget(
                Paragraph::new(crate::trf!(
                    "{message}\n\nEnter / Esc: {close}",
                    message = message,
                    close = crate::tr!("close")
                ))
                .wrap(Wrap { trim: false })
                .block(pane_block(crate::tr!("Error"), true)),
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
        app.layout.overlay = rect;
        frame.render_widget(Clear, rect);
        let lines = super::workspace_help::lines(rect.width.saturating_sub(2) as usize);
        let max_scroll = lines
            .len()
            .saturating_sub(rect.height.saturating_sub(2) as usize)
            .min(u16::MAX as usize) as u16;
        let help = Paragraph::new(lines).block(pane_block(crate::tr!("Help"), true).title_bottom(
            Line::from(vec![
                Span::styled(" j/k ↑↓ ", Style::default().fg(Color::Cyan)),
                Span::raw(crate::tr!("scroll · ")),
                Span::styled("Esc / ?", Style::default().fg(Color::Cyan)),
                Span::raw(crate::tr!(" close ")),
            ]),
        ));
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
    let prefix_width = (prefix.width().min(u16::MAX as usize) as u16).min(end - x);
    frame.render_widget(
        Paragraph::new(prefix.clone()).style(Style::default().fg(Color::Gray)),
        Rect::new(x, rect.y, prefix_width.min(end - x), 1),
    );
    x += prefix_width;
    for (index, label) in labels.iter().enumerate() {
        if x >= end {
            break;
        }
        let text = if index == active {
            format!("[{label}]")
        } else {
            format!(" {label} ")
        };
        let width = (text.width().min(u16::MAX as usize) as u16).min(end - x);
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
