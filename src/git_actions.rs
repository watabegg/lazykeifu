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
    Stay,
    Checkout(
        Box<crate::git::checkout::Checkout>,
        crate::git::checkout::Changes,
    ),
    Action(Action),
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
    pub(crate) fn request_checkout(&mut self, target: crate::git::checkout::Target) -> Result<()> {
        use crate::git::checkout::{Changes, Checkout};
        let checkout = Checkout::prepare(&self.repo.repo, target)?;
        if checkout.dirty() {
            self.git_menu(
                crate::trf!(
                    "Uncommitted changes · switch to {target}",
                    target = checkout.target.label()
                ),
                vec![
                    (crate::tr!("Cancel").into(), Choice::Stay),
                    (
                        crate::tr!("Stash changes and switch").into(),
                        Choice::Checkout(Box::new(checkout.clone()), Changes::Stash),
                    ),
                    (
                        crate::tr!("Discard all changes and switch").into(),
                        Choice::Checkout(Box::new(checkout), Changes::Discard),
                    ),
                ],
            );
        } else {
            self.workspace_git_job(move |repo| checkout.execute(repo, None));
        }
        Ok(())
    }

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
                crate::tr!("Amend HEAD · staged changes + message").into(),
                Choice::Amend(false),
            ),
            (
                crate::tr!("Reword HEAD · message only; keep staged changes").into(),
                Choice::Amend(true),
            ),
            (
                crate::tr!("Undo last commit · keep changes staged").into(),
                Choice::Undo,
            ),
            (
                crate::tr!("Reflog · find and restore previous HEAD").into(),
                Choice::Reflog,
            ),
        ];
        if let Some(oid) = target {
            rows.push((
                crate::trf!(
                    "Reset current branch to {oid}…",
                    oid = &oid.to_string()[..7]
                ),
                Choice::ResetMenu(oid),
            ));
            rows.push((
                crate::trf!("Create tag at {oid}…", oid = &oid.to_string()[..7]),
                Choice::Tag(oid),
            ));
        }
        if self.workspace.focus == Panel::Branches {
            if let Some(branch) = self.workspace_selected_branch().filter(|b| !b.is_remote) {
                rows.push((
                    crate::trf!("Rename branch {branch}…", branch = branch.name),
                    Choice::Rename(Rename {
                        old: branch.name.clone(),
                        oid: branch.tip_oid,
                    }),
                ));
            }
        }
        if self.workspace.composer.amend.is_some() {
            rows.push((
                crate::tr!("New commit · leave amend draft").into(),
                Choice::NewCommit,
            ));
        }
        self.git_menu(crate::tr!("Extras · j/k select · Enter open").into(), rows);
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
    pub(crate) fn choose_git_action(&mut self, choice: Choice) -> Result<()> {
        match choice {
            Choice::Stay => self.mode = AppMode::Normal,
            Choice::Checkout(checkout, changes) => {
                self.mode = AppMode::Normal;
                self.workspace_git_job(move |repo| checkout.execute(repo, Some(changes)));
            }
            Choice::Action(action) => {
                self.mode = AppMode::Normal;
                self.handle_action(action)?;
            }
            Choice::NewCommit => {
                self.workspace.composer = Default::default();
                self.mode = AppMode::Normal;
                self.open_composer()?;
            }
            Choice::Amend(message_only) => {
                let snapshot = Snapshot::capture(&self.repo.repo)?;
                self.mode = AppMode::Confirm {
                    message: crate::trf!(
                        "Rewrite HEAD {head}? {detail}. Published history will change.",
                        head = &snapshot.head.to_string()[..7],
                        detail = if message_only {
                            crate::tr!("Edit message only; staged changes stay staged")
                        } else {
                            crate::tr!("Include staged changes and edit message")
                        }
                    ),
                    action: ConfirmAction::Amend(snapshot, message_only),
                };
            }
            Choice::Undo => {
                let snapshot = Snapshot::capture(&self.repo.repo)?;
                let commit = self.repo.repo.find_commit(snapshot.head)?;
                if commit.parent_count() == 0 {
                    bail!(
                        "{}",
                        crate::tr!("The initial commit has no parent; use amend instead")
                    );
                }
                let target = commit.parent_id(0)?;
                self.mode = AppMode::Confirm {
                    message: crate::trf!(
                        "Undo HEAD {head} to first parent {parent}? Keep index and working files; commit changes become staged. Published history will change.",
                        head = &snapshot.head.to_string()[..7],
                        parent = &target.to_string()[..7]
                    ),
                    action: ConfirmAction::Reset(Reset {
                        snapshot,
                        target,
                        mode: ResetMode::Soft,
                    }),
                };
            }
            Choice::ResetMenu(oid) => self.git_menu(
                crate::trf!(
                    "Reset to {oid} · choose what to keep",
                    oid = &oid.to_string()[..7]
                ),
                [ResetMode::Soft, ResetMode::Mixed, ResetMode::Hard]
                    .iter()
                    .map(|mode| {
                        (
                            crate::trf!(
                                "{flag} · {description}",
                                flag = mode.flag(),
                                description = mode.description()
                            ),
                            Choice::Reset(oid, *mode),
                        )
                    })
                    .collect(),
            ),
            Choice::Reset(target, mode) => {
                let snapshot = Snapshot::capture(&self.repo.repo)?;
                self.mode = AppMode::Confirm {
                    message: crate::trf!(
                        "Reset {reference} from {head} to {target} {flag}? {description}. Published history will change.",
                        reference = snapshot.reference,
                        head = &snapshot.head.to_string()[..7],
                        target = &target.to_string()[..7],
                        flag = mode.flag(),
                        description = mode.description()
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
                            crate::trf!(
                                "HEAD@{selector} {oid} {message}",
                                selector = format!("{{{i}}}"),
                                oid = &e.id_new().to_string()[..7],
                                message = e.message().unwrap_or("")
                            ),
                            Choice::ResetMenu(e.id_new()),
                        )
                    })
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    bail!("{}", crate::tr!("No HEAD reflog entries"));
                }
                drop(log);
                self.git_menu(
                    crate::tr!("HEAD reflog · Enter choose reset · Esc close").into(),
                    rows,
                );
            }
            Choice::Tag(oid) => {
                self.mode = AppMode::Input {
                    title: crate::trf!(
                        "Create lightweight tag at {oid}",
                        oid = &oid.to_string()[..7]
                    ),
                    input: String::new(),
                    action: InputAction::CreateTag(oid),
                }
            }
            Choice::Rename(target) => {
                self.mode = AppMode::Input {
                    title: crate::trf!("Rename branch {branch}", branch = target.old),
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
            .context(crate::tr!("Commit message is not UTF-8"))?
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
    app.layout.overlay = rect;
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
        Paragraph::new(Line::from(crate::tr!(
            "↑/↓ select · Enter open · Esc close"
        )))
        .style(Style::default().fg(Color::DarkGray)),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}

impl App {
    fn selected_git_target(&self) -> Result<Oid> {
        if matches!(self.workspace.focus, Panel::Graph | Panel::CommitDetail) {
            self.graph_list_state
                .selected()
                .and_then(|i| self.graph_layout.nodes.get(i))
                .and_then(|n| n.commit.as_ref())
                .map(|c| c.oid)
                .context(crate::tr!("Select a commit in Graph first"))
        } else if self.workspace.focus == Panel::Branches {
            self.workspace_selected_branch()
                .map(|b| b.tip_oid)
                .or(self.workspace.commit)
                .context(crate::tr!("Select a branch or tag first"))
        } else {
            self.repo
                .repo
                .head()?
                .target()
                .context(crate::tr!("No HEAD commit"))
        }
    }
    pub(crate) fn context_git_action(&mut self, action: &Action) -> Result<bool> {
        match action {
            Action::StageToggle
                if self.workspace.focus == Panel::Files
                    && self.workspace.tabs.files == crate::workspace_tabs::FilesTab::Files
                    && self.workspace.commit.is_none()
                    && self
                        .workspace
                        .tree
                        .selected()
                        .is_some_and(|n| n.file.is_none()) =>
            {
                let path = self.workspace.tree.selected().unwrap().path.clone();
                self.workspace.load_files(&self.repo.repo)?;
                let rows = self
                    .workspace
                    .files
                    .iter()
                    .filter(|f| f.path.starts_with(&path))
                    .cloned()
                    .collect::<Vec<_>>();
                if rows.iter().any(|f| f.conflict) {
                    bail!(
                        "{}",
                        crate::tr!("Resolve conflicts individually before staging this directory")
                    );
                }
                let unstage = !rows.iter().any(|f| f.unstaged);
                for row in rows
                    .iter()
                    .filter(|f| if unstage { f.staged } else { f.unstaged })
                {
                    crate::workspace::toggle_stage(&self.repo.repo, row, unstage)?;
                }
                self.workspace.invalidate();
                self.set_message(if unstage {
                    crate::tr!("Directory unstaged")
                } else {
                    crate::tr!("Directory staged")
                });
            }
            Action::AmendHead | Action::RewordHead => {
                if self.workspace.focus == Panel::Graph
                    && self.selected_git_target()? != Snapshot::capture(&self.repo.repo)?.head
                {
                    bail!("{}", crate::tr!("Only HEAD amend/reword is supported. Select HEAD; older commits require interactive rebase"));
                }
                self.choose_git_action(Choice::Amend(*action == Action::RewordHead))?;
            }
            Action::RenameBranch => {
                let branch = self
                    .workspace_selected_branch()
                    .filter(|b| !b.is_remote)
                    .context(crate::tr!("Select a local branch first"))?;
                self.choose_git_action(Choice::Rename(Rename {
                    old: branch.name.clone(),
                    oid: branch.tip_oid,
                }))?;
            }
            Action::CreateTag => {
                self.choose_git_action(Choice::Tag(self.selected_git_target()?))?
            }
            Action::CreateBranch if self.workspace.focus == Panel::Branches => {
                self.mode = AppMode::Input {
                    title: crate::tr!("New branch from selected branch").into(),
                    input: String::new(),
                    action: InputAction::CreateBranchAt(self.selected_git_target()?),
                };
            }
            Action::ResetMenu => {
                self.choose_git_action(Choice::ResetMenu(self.selected_git_target()?))?
            }
            Action::InspectBranch => {
                let target = self.selected_git_target()?;
                let i = self
                    .graph_layout
                    .nodes
                    .iter()
                    .position(|n| n.commit.as_ref().is_some_and(|c| c.oid == target))
                    .ok_or_else(|| anyhow::anyhow!(
                        "{}",
                        crate::tr!("Branch tip is outside the loaded graph; load more history or show remote branches first")
                    ))?;
                self.select_node(i);
                self.workspace_focus(Panel::Graph);
                self.workspace_commit_target(target);
            }
            Action::CheckoutRevision => {
                let oid = self.selected_git_target()?;
                self.request_checkout(crate::git::checkout::Target::Commit(oid))?;
            }
            Action::CopyCommit => {
                let oid = self.selected_git_target()?;
                self.workspace.copied_commit = Some(oid);
                self.set_message(crate::trf!(
                    "Copied {oid} for cherry-pick · V paste · Ctrl-r clear",
                    oid = &oid.to_string()[..7]
                ));
            }
            Action::ClearCopiedCommit => {
                self.workspace.copied_commit = None;
                self.set_message(crate::tr!("Cherry-pick selection cleared"));
            }
            Action::PasteCommit => {
                let oid = self
                    .workspace
                    .copied_commit
                    .context(crate::tr!("Copy a commit with C first"))?;
                let target = crate::git::commit_operation::CommitOperation::capture(
                    &self.repo.repo,
                    oid,
                    crate::git::recovery::Kind::CherryPick,
                )?;
                self.mode = AppMode::Confirm {
                    message: target.message(),
                    action: ConfirmAction::CommitOperation(target),
                };
            }
            Action::RecoveryMenu => {
                let mut rows = Vec::new();
                if crate::git::recovery::kind(&self.repo.repo).is_some() {
                    rows.push((
                        crate::tr!("Continue current Git operation").into(),
                        Choice::Action(Action::ContinueOperation),
                    ));
                    rows.push((
                        crate::tr!("Abort current Git operation").into(),
                        Choice::Action(Action::AbortOperation),
                    ));
                } else if self.workspace.focus == Panel::Graph {
                    rows.push((
                        crate::tr!("Merge selected branch into current").into(),
                        Choice::Action(Action::Merge),
                    ));
                    rows.push((
                        crate::tr!("Rebase current onto selected branch").into(),
                        Choice::Action(Action::Rebase),
                    ));
                } else {
                    bail!(
                        "{}",
                        crate::tr!("No merge/rebase/cherry-pick/revert in progress")
                    );
                }
                self.git_menu(crate::tr!("Merge / rebase options").into(), rows);
            }
            Action::LogMenu => self.git_menu(
                crate::tr!("Log options").into(),
                vec![
                    (
                        crate::tr!("Load 500 more commits").into(),
                        Choice::Action(Action::LoadMoreHistory),
                    ),
                    (crate::tr!("HEAD reflog").into(), Choice::Reflog),
                    (
                        crate::tr!("Toggle remote branches").into(),
                        Choice::Action(Action::ToggleRemoteBranches),
                    ),
                    (
                        crate::tr!("Toggle tags").into(),
                        Choice::Action(Action::ToggleTags),
                    ),
                ],
            ),
            Action::ToggleStageAll => {
                self.workspace.load_files(&self.repo.repo)?;
                if self.workspace.files.iter().any(|f| f.conflict) {
                    bail!(
                        "{}",
                        crate::tr!("Resolve conflicts individually with Space before staging all")
                    );
                }
                let action = if self.workspace.files.iter().any(|f| f.unstaged) {
                    Action::StageAll
                } else {
                    Action::UnstageAll
                };
                self.workspace_action(&action)?;
            }
            Action::NextFile | Action::PrevFile if self.workspace.focus == Panel::Diff => {
                let len = self.workspace.files.len();
                if len > 0 {
                    let old = self.workspace.file_state.selected().unwrap_or(0);
                    let next = if *action == Action::NextFile {
                        (old + 1).min(len - 1)
                    } else {
                        old.saturating_sub(1)
                    };
                    let path = self.workspace.files[next].path.clone();
                    self.workspace.tree.closed.retain(|p| !path.starts_with(p));
                    self.workspace.tree.state.select(None);
                    self.workspace.file_state.select(Some(next));
                    self.tree_sync();
                    self.workspace.staged =
                        self.workspace.files[next].staged && !self.workspace.files[next].unstaged;
                }
            }
            Action::CollapseAll | Action::ExpandAll => {
                if *action == Action::ExpandAll {
                    self.workspace.tree.closed.clear();
                } else {
                    for file in &self.workspace.files {
                        for parent in file
                            .path
                            .ancestors()
                            .skip(1)
                            .filter(|p| !p.as_os_str().is_empty())
                        {
                            self.workspace.tree.closed.insert(parent.to_path_buf());
                        }
                    }
                }
                self.tree_sync();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
