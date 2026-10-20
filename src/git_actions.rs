//! Contextual Git actions and reflog navigation.
use crate::{
    action::Action,
    app::{App, AppMode, ConfirmAction, InputAction},
    git::history::{Rename, Reset, ResetMode, Snapshot},
    workspace::Panel,
};
use anyhow::{bail, Context, Result};
use git2::Oid;
use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
#[derive(Clone, Debug)]
pub enum Choice {
    NewCommit,
    Amend(bool),
    Undo,
    Reset(Oid, ResetMode),
    ResetMenu(Oid),
    Reflog,
    Tag(Oid),
    Rename(Rename),
}
#[derive(Clone, Debug)]
pub struct Menu {
    pub title: String,
    pub rows: Vec<(String, Choice)>,
    pub selected: usize,
    pub offset: usize,
    pub rect: Rect,
}
impl App {
    fn git_menu(&mut self, title: String, rows: Vec<(String, Choice)>) {
        self.mode = AppMode::GitMenu(Menu {
            title,
            rows,
            selected: 0,
            offset: 0,
            rect: Rect::default(),
        });
    }
    pub(crate) fn open_git_menu(&mut self) -> Result<()> {
        let target = if matches!(self.workspace.focus, Panel::Graph | Panel::CommitDetail) {
            self.graph_list_state
                .selected()
                .and_then(|i| self.graph_layout.nodes.get(i))
                .and_then(|n| n.commit.as_ref())
                .map(|c| c.oid)
        } else if self.workspace.focus == Panel::Branches {
            self.workspace_selected_branch()
                .map(|b| b.tip_oid)
                .or(self.workspace.commit)
        } else {
            self.repo.repo.head().ok().and_then(|h| h.target())
        };
        let mut rows = vec![
            (
                "Amend HEAD · staged changes + message".into(),
                Choice::Amend(false),
            ),
            (
                "Reword HEAD · message only; keep staged changes".into(),
                Choice::Amend(true),
            ),
            (
                "Undo last commit · keep changes staged".into(),
                Choice::Undo,
            ),
            (
                "Reflog · find and restore previous HEAD".into(),
                Choice::Reflog,
            ),
        ];
        if let Some(oid) = target {
            rows.push((
                format!("Reset current branch to {}…", &oid.to_string()[..7]),
                Choice::ResetMenu(oid),
            ));
            rows.push((
                format!("Create tag at {}…", &oid.to_string()[..7]),
                Choice::Tag(oid),
            ));
        }
        if self.workspace.focus == Panel::Branches {
            if let Some(branch) = self.workspace_selected_branch().filter(|b| !b.is_remote) {
                rows.push((
                    format!("Rename branch {}…", branch.name),
                    Choice::Rename(Rename {
                        old: branch.name.clone(),
                        oid: branch.tip_oid,
                    }),
                ));
            }
        }
        if self.workspace.composer.amend.is_some() {
            rows.push(("New commit · leave amend draft".into(), Choice::NewCommit));
        }
        self.git_menu("Git actions · j/k select · Enter open".into(), rows);
        Ok(())
    }
    pub(crate) fn git_menu_action(&mut self, action: Action) -> Result<()> {
        let AppMode::GitMenu(menu) = &mut self.mode else {
            return Ok(());
        };
        match action {
            Action::Cancel | Action::Quit | Action::GitMenu => self.mode = AppMode::Normal,
            Action::MoveDown | Action::PageDown => {
                menu.selected = (menu.selected + if action == Action::PageDown { 10 } else { 1 })
                    .min(menu.rows.len().saturating_sub(1))
            }
            Action::MoveUp | Action::PageUp => {
                menu.selected =
                    menu.selected
                        .saturating_sub(if action == Action::PageUp { 10 } else { 1 })
            }
            Action::GoToTop => menu.selected = 0,
            Action::GoToBottom => menu.selected = menu.rows.len().saturating_sub(1),
            Action::Checkout | Action::Confirm | Action::StageToggle => {
                let Some((_, choice)) = menu.rows.get(menu.selected).cloned() else {
                    return Ok(());
                };
                self.choose_git_action(choice)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn choose_git_action(&mut self, choice: Choice) -> Result<()> {
        match choice {
            Choice::NewCommit => {
                self.workspace.composer = Default::default();
                self.mode = AppMode::Normal;
                self.open_composer()?;
            }
            Choice::Amend(message_only) => {
                let snapshot = Snapshot::capture(&self.repo.repo)?;
                self.mode = AppMode::Confirm {
                    message: format!(
                        "Rewrite HEAD {}? {}. Published history will change.",
                        &snapshot.head.to_string()[..7],
                        if message_only {
                            "Edit message only; staged changes stay staged"
                        } else {
                            "Include staged changes and edit message"
                        }
                    ),
                    action: ConfirmAction::Amend(snapshot, message_only),
                };
            }
            Choice::Undo => {
                let snapshot = Snapshot::capture(&self.repo.repo)?;
                let commit = self.repo.repo.find_commit(snapshot.head)?;
                if commit.parent_count() == 0 {
                    bail!("The initial commit has no parent; use amend instead");
                }
                let target = commit.parent_id(0)?;
                self.mode=AppMode::Confirm {message:format!("Undo HEAD {} to first parent {}? Keep index and working files; commit changes become staged. Published history will change.",&snapshot.head.to_string()[..7],&target.to_string()[..7]),action:ConfirmAction::Reset(Reset {snapshot,target,mode:ResetMode::Soft})};
            }
            Choice::ResetMenu(oid) => self.git_menu(
                format!("Reset to {} · choose what to keep", &oid.to_string()[..7]),
                [ResetMode::Soft, ResetMode::Mixed, ResetMode::Hard]
                    .iter()
                    .map(|mode| {
                        (
                            format!("{} · {}", mode.flag(), mode.description()),
                            Choice::Reset(oid, *mode),
                        )
                    })
                    .collect(),
            ),
            Choice::Reset(target, mode) => {
                let snapshot = Snapshot::capture(&self.repo.repo)?;
                self.mode = AppMode::Confirm {
                    message: format!(
                        "Reset {} from {} to {} {}? {}. Published history will change.",
                        snapshot.reference,
                        &snapshot.head.to_string()[..7],
                        &target.to_string()[..7],
                        mode.flag(),
                        mode.description()
                    ),
                    action: ConfirmAction::Reset(Reset {
                        snapshot,
                        target,
                        mode,
                    }),
                };
            }
            Choice::Reflog => {
                let log = self.repo.repo.reflog("HEAD")?;
                let rows = log
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| !e.id_new().is_zero())
                    .take(200)
                    .map(|(i, e)| {
                        (
                            format!(
                                "HEAD@{{{i}}} {} {}",
                                &e.id_new().to_string()[..7],
                                e.message().unwrap_or("")
                            ),
                            Choice::ResetMenu(e.id_new()),
                        )
                    })
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    bail!("No HEAD reflog entries");
                }
                drop(log);
                self.git_menu("HEAD reflog · Enter choose reset · Esc close".into(), rows);
            }
            Choice::Tag(oid) => {
                self.mode = AppMode::Input {
                    title: format!("Create lightweight tag at {}", &oid.to_string()[..7]),
                    input: String::new(),
                    action: InputAction::CreateTag(oid),
                }
            }
            Choice::Rename(target) => {
                self.mode = AppMode::Input {
                    title: format!("Rename branch {}", target.old),
                    input: target.old.clone(),
                    action: InputAction::RenameBranch(target),
                }
            }
        }
        Ok(())
    }
    pub(crate) fn open_amend(&mut self, message_only: bool) -> Result<()> {
        let snapshot = Snapshot::capture(&self.repo.repo)?;
        let message = self
            .repo
            .repo
            .find_commit(snapshot.head)?
            .message()
            .context("Commit message is not UTF-8")?
            .trim_end()
            .to_string();
        let (subject, body) = message.split_once('\n').unwrap_or((&message, ""));
        let c = &mut self.workspace.composer;
        c.subject = Default::default();
        c.body = Default::default();
        c.subject.insert(subject);
        c.body.insert(body.trim_start_matches('\n'));
        c.amend = Some((snapshot, message_only));
        self.open_composer()
    }
}
pub fn draw(frame: &mut Frame, app: &mut App) {
    let AppMode::GitMenu(menu) = &mut app.mode else {
        return;
    };
    let area = frame.area();
    let width = area.width.saturating_sub(4).min(90);
    let height = (menu.rows.len() as u16 + 4).min(area.height.saturating_sub(4));
    let rect = Rect::new(
        (area.width - width) / 2,
        (area.height - height) / 2,
        width,
        height,
    );
    menu.rect = rect;
    frame.render_widget(Clear, rect);
    frame.render_widget(crate::ui::pane_block(&menu.title, true), rect);
    let inner = rect.inner(Margin::new(1, 1));
    let list = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(1),
    );
    let mut state = ListState::default()
        .with_selected(Some(menu.selected))
        .with_offset(menu.offset);
    frame.render_stateful_widget(
        List::new(
            menu.rows
                .iter()
                .map(|(text, _)| ListItem::new(text.clone()))
                .collect::<Vec<_>>(),
        )
        .highlight_symbol("› ")
        .highlight_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray)),
        list,
        &mut state,
    );
    menu.offset = state.offset();
    frame.render_widget(
        Paragraph::new(Line::from("↑/↓ select · Enter open · Esc close"))
            .style(Style::default().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}
