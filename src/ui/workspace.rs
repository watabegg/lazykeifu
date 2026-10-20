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
    text::Line,
    widgets::{Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    app.workspace_tick();
    let area = frame.area();
    app.layout = LayoutMap::default();
    app.workspace.status_rect = Rect::default();
    app.workspace.branches_rect = Rect::default();
    app.status_hints.clear();
    if area.width < 20 || area.height < 6 {
        frame.render_widget(Paragraph::new("Terminal too small"), area);
        return;
    }
    let body = Rect {
        height: area.height.saturating_sub(2),
        ..area
    };
    let footer = Rect::new(area.x, body.bottom(), area.width, 2);
    app.layout.status_bar = footer;
    let focus = app.workspace.focus;
    if area.width < 80 || area.height < 18 {
        match focus {
            Panel::Status => app.workspace.status_rect = body,
            Panel::Files => app.layout.files = body,
            Panel::Branches => app.workspace.branches_rect = body,
            Panel::Graph => app.layout.graph = body,
            Panel::Diff => app.layout.commit_detail = body,
        }
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(body);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Percentage(30),
            ])
            .split(columns[0]);
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(columns[1]);
        app.workspace.status_rect = left[0];
        app.layout.files = left[1];
        app.workspace.branches_rect = left[2];
        app.layout.graph = right[0];
        app.layout.commit_detail = right[1];
    }
    let status = format!(
        "{} · {} files{}{}",
        app.head_name.as_deref().unwrap_or("unborn HEAD"),
        app.workspace.files.len(),
        if app.workspace.busy() || app.is_pushing() {
            " · busy"
        } else {
            ""
        },
        match app.repo.repo.state() {
            git2::RepositoryState::Clean => "",
            git2::RepositoryState::Merge => " · MERGING",
            git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge => " · REBASING",
            _ => " · Git operation in progress",
        }
    );
    frame.render_widget(
        Paragraph::new(status).block(pane_block("1 Status", focus == Panel::Status)),
        app.workspace.status_rect,
    );
    let files_title = match app.workspace.commit {
        Some(oid) => format!("Commit files {} · 2 worktree", &oid.to_string()[..7]),
        None => "2 Working files · index/worktree".into(),
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
            } else {
                Style::default()
            })
        })
        .collect();
    frame.render_stateful_widget(
        List::new(rows)
            .block(pane_block(&files_title, focus == Panel::Files))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› "),
        app.layout.files,
        &mut app.workspace.file_state,
    );
    let rows: Vec<ListItem> = app
        .branches
        .iter()
        .map(|b| ListItem::new(format!("{} {}", if b.is_head { '*' } else { ' ' }, b.name)))
        .collect();
    if app.workspace.branch_state.selected().is_none() && !rows.is_empty() {
        app.workspace.branch_state.select(Some(0));
    }
    frame.render_stateful_widget(
        List::new(rows)
            .block(pane_block("3 Branches", focus == Panel::Branches))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("› "),
        app.workspace.branches_rect,
        &mut app.workspace.branch_state,
    );
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
    let title = if let Some(oid) = app.workspace.commit {
        format!("0 Diff · commit {}", &oid.to_string()[..7])
    } else {
        format!(
            "0 Diff · {} · v switch",
            if app.workspace.staged {
                "STAGED"
            } else {
                "UNSTAGED"
            }
        )
    };
    app.workspace.diff_height = app.layout.commit_detail.height.saturating_sub(2);
    let lines: Vec<Line> = app
        .workspace
        .preview
        .lines()
        .map(|line| {
            let color = if line.starts_with('+') {
                Color::Green
            } else if line.starts_with('-') {
                Color::Red
            } else if line.starts_with("@@") {
                Color::Cyan
            } else {
                Color::Reset
            };
            Line::styled(line.to_string(), Style::default().fg(color))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((app.workspace.scroll, app.workspace.horizontal))
            .block(pane_block(&title, focus == Panel::Diff)),
        app.layout.commit_detail,
    );
    let hints =
        "2 files 3 branches 4 graph 0 diff · Tab/h/l focus · Space stage · c commit · ? help";
    let message = app
        .get_message()
        .unwrap_or("Enter: details · v: staged/unstaged · Esc: worktree · q: quit");
    frame.render_widget(Paragraph::new(format!("{hints}\n{message}")), footer);
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
        AppMode::Confirm { message, .. } => frame.render_widget(
            ConfirmDialog::new(message),
            super::centered_rect(80, 30, area),
        ),
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
        frame.render_widget(Paragraph::new("1 status · 2 working files · 3 branches · 4 graph · 0 diff\nTab / Shift-Tab / h / l: move focus\nj / k: select / scroll · Ctrl-d / Ctrl-u: page\nSpace: stage / unstage file; checkout branch\nEnter: file → diff; graph → commit files; branch → checkout\nv: staged / unstaged diff · H / L: horizontal scroll\nc: commit staged changes (Git hooks and signing)\na: stage all · u: unstage all (working files only)\nb: create branch · d: delete branch (branches only)\n/: search branches · f: fetch · p: pull · P: push · R: refresh\no: remote visibility · t: tags\nEsc: working files · q: quit · ?: close help\n\nCommit files are read-only. Use 2 to return to working files.\nStash and hunk staging are not implemented yet.").wrap(Wrap { trim: false }).block(pane_block("Help",true)),rect);
    }
}
