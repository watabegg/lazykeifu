//! Persistent panels and background Git jobs for the integrated workspace.
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

use crate::git::command::git_output;
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository, Status, StatusOptions};
use ratatui::{layout::Rect, widgets::ListState};

use crate::{
    action::Action,
    app::{App, AppMode, InputAction},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Panel {
    Status,
    #[default]
    Files,
    Branches,
    Graph,
    Diff,
    Stashes,
    CommitDetail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRow {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub conflict: bool,
    pub index_status: char,
    pub worktree_status: char,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreviewKey {
    stash: Option<Oid>,
    stash_view: bool,
    revision: u64,
    commit: Option<Oid>,
    path: Option<PathBuf>,
    staged: bool,
}

type PreviewResult = (
    PreviewKey,
    Result<(String, crate::ui::patch::PatchView), String>,
);

#[derive(Default)]
pub struct Workspace {
    pub focus: Panel,
    pub copied_commit: Option<Oid>,
    pub composer: crate::commit_composer::Composer,
    pub tabs: crate::workspace_tabs::Tabs,
    pub external: Option<crate::external::Request>,
    pub stashes: Vec<crate::git::stash::StashEntry>,
    pub stash_state: ListState,
    pub stash_rect: Rect,
    pub stash_preview: Option<Oid>,
    stashes_loaded: bool,
    pub files: Vec<FileRow>,
    pub file_state: ListState,
    pub tree: crate::file_tree::Tree,
    pub branch_state: ListState,
    pub commit: Option<Oid>,
    pub staged: bool,
    pub preview: String,
    pub patch: crate::ui::patch::PatchView,
    pub scroll: u16,
    pub horizontal: u16,
    pub diff_height: u16,
    pub diff_visible: bool,
    pub diff_return: Panel,
    pub hunk_index: usize,
    pub status_rect: Rect,
    pub branches_rect: Rect,
    pub help: bool,
    pub help_rect: Rect,
    pub detail_rect: Rect,
    pub commit_error: Option<String>,
    revision: u64,
    loaded: bool,
    files_commit: Option<Oid>,
    preview_key: Option<PreviewKey>,
    preview_rx: Option<Receiver<PreviewResult>>,
    job: Option<Receiver<Result<String, String>>>,
    commit_message: Option<String>,
}

pub fn working_files(repo: &Repository) -> Result<Vec<FileRow>> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true);
    let mut rows = Vec::new();
    for entry in repo.statuses(Some(&mut opts))?.iter() {
        let status = entry.status();
        if status.is_ignored() || status.is_empty() {
            continue;
        }
        let delta = entry.index_to_workdir().or_else(|| entry.head_to_index());
        let path = delta
            .as_ref()
            .and_then(|d| d.new_file().path().or(d.old_file().path()))
            .context(crate::tr!("Git status returned a pathless entry"))?
            .to_path_buf();
        let old_path = entry
            .head_to_index()
            .filter(|d| d.status() == git2::Delta::Renamed)
            .and_then(|d| d.old_file().path().map(PathBuf::from));
        let (index_status, worktree_status) = status_letters(repo, &path, status)?;
        rows.push(FileRow {
            index_status,
            worktree_status,
            path,
            old_path,
            staged: status.intersects(
                Status::INDEX_NEW
                    | Status::INDEX_MODIFIED
                    | Status::INDEX_DELETED
                    | Status::INDEX_RENAMED
                    | Status::INDEX_TYPECHANGE,
            ),
            unstaged: status.intersects(
                Status::WT_NEW
                    | Status::WT_MODIFIED
                    | Status::WT_DELETED
                    | Status::WT_RENAMED
                    | Status::WT_TYPECHANGE,
            ),
            untracked: status.is_wt_new(),
            conflict: status.is_conflicted(),
        });
    }
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(rows)
}

fn commit_files(repo: &Repository, oid: Oid) -> Result<Vec<FileRow>> {
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    let parent = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };
    let diff = repo.diff_tree_to_tree(parent.as_ref(), Some(&tree), None)?;
    Ok(diff
        .deltas()
        .filter_map(|d| {
            d.new_file()
                .path()
                .or(d.old_file().path())
                .map(|p| FileRow {
                    path: p.to_path_buf(),
                    old_path: None,
                    staged: false,
                    unstaged: false,
                    untracked: false,
                    conflict: false,
                    index_status: match d.status() {
                        git2::Delta::Added => 'A',
                        git2::Delta::Deleted => 'D',
                        git2::Delta::Renamed => 'R',
                        git2::Delta::Copied => 'C',
                        git2::Delta::Typechange => 'T',
                        _ => 'M',
                    },
                    worktree_status: ' ',
                })
        })
        .collect())
}

fn preview(repo: &Repository, key: &PreviewKey) -> Result<String> {
    use std::ffi::OsStr;
    if let Some(oid) = key.stash {
        return crate::git::stash::preview(repo, oid);
    }
    if key.stash_view {
        return Ok(crate::tr!("No stashes (s: save working changes)").into());
    }
    let Some(path) = &key.path else {
        return Ok(crate::tr!("No changed files").into());
    };
    if key.commit.is_none() && !key.staged && repo.status_file(path)?.is_wt_new() {
        let full = repo
            .workdir()
            .context(crate::tr!("No working tree"))?
            .join(path);
        if full.is_symlink() {
            return Ok(crate::trf!(
                "Symlink → {target}",
                target = std::fs::read_link(full)?.display()
            ));
        }
        if std::fs::metadata(&full)?.len() > 1024 * 1024 {
            return Ok(crate::tr!("Untracked file exceeds 1 MiB; preview omitted").into());
        }
        let bytes = std::fs::read(full)?;
        if bytes.contains(&0) {
            return Ok(crate::tr!("Binary untracked file").into());
        }
        let text = String::from_utf8_lossy(&bytes);
        let count = text.lines().count();
        return Ok(crate::trf!(
            "diff --git a/{path} b/{path}\nnew file (untracked)\n@@ -0,0 +1,{count} @@\n{body}",
            path = path.display(),
            count = count,
            body = text
                .lines()
                .map(|line| format!("+{line}\n"))
                .collect::<String>()
        ));
    }
    if key.commit.is_none() {
        let text = crate::git::changes::diff(repo, path, key.staged)?;
        return Ok(if text.is_empty() {
            crate::tr!("No changes on this side (Tab in Diff: staged / unstaged)").into()
        } else {
            text
        });
    }
    let mut args = vec![
        OsStr::new("diff"),
        OsStr::new("--no-ext-diff"),
        OsStr::new("--no-textconv"),
        OsStr::new("--color=never"),
    ];
    let mut revisions = Vec::new();
    if let Some(oid) = key.commit {
        let commit = repo.find_commit(oid)?;
        if commit.parent_count() > 0 {
            revisions.push(commit.parent_id(0)?.to_string());
            revisions.push(oid.to_string());
        } else {
            args = vec![
                OsStr::new("show"),
                OsStr::new("--format="),
                OsStr::new("--no-ext-diff"),
                OsStr::new("--no-textconv"),
                OsStr::new("--color=never"),
            ];
            revisions.push(oid.to_string());
        }
    } else if key.staged {
        args.push(OsStr::new("--cached"));
    }
    for revision in &revisions {
        args.push(OsStr::new(revision));
    }
    args.push(OsStr::new("--"));
    args.push(path.as_os_str());
    let text = git_output(repo, &args)?;
    Ok(if text.is_empty() {
        crate::tr!("No changes on this side (Tab in Diff: staged / unstaged)").into()
    } else {
        text
    })
}

pub fn toggle_stage(repo: &Repository, row: &FileRow, staged_side: bool) -> Result<()> {
    use std::ffi::OsStr;
    if row.conflict {
        bail!(
            "{}",
            crate::tr!("Resolve this conflict with Git before staging")
        );
    }
    let unstage = row.staged && (staged_side || !row.unstaged);
    let mut args: Vec<&OsStr> = if unstage {
        if repo.head().is_ok() {
            vec!["restore".as_ref(), "--staged".as_ref(), "--".as_ref()]
        } else {
            vec![
                "rm".as_ref(),
                "--cached".as_ref(),
                "--ignore-unmatch".as_ref(),
                "--".as_ref(),
            ]
        }
    } else {
        vec!["add".as_ref(), "-A".as_ref(), "--".as_ref()]
    };
    args.push(row.path.as_os_str());
    if let Some(old) = &row.old_path {
        if unstage || repo.index()?.get_path(old, 0).is_some() {
            args.push(old.as_os_str());
        }
    }
    git_output(repo, &args)?;
    Ok(())
}

impl Workspace {
    pub fn with_focus(focus: Panel) -> Self {
        Self {
            focus,
            ..Self::default()
        }
    }

    pub fn busy(&self) -> bool {
        self.job.is_some() || self.external.is_some()
    }
    pub fn preview_loading(&self) -> bool {
        let key = PreviewKey {
            stash: self.stash_preview,
            stash_view: self.focus == Panel::Stashes || self.stash_preview.is_some(),
            revision: self.revision,
            commit: self.commit,
            path: self
                .file_state
                .selected()
                .and_then(|i| self.files.get(i))
                .map(|r| r.path.clone()),
            staged: self.staged,
        };
        !self.loaded || self.preview_rx.is_some() || self.preview_key.as_ref() != Some(&key)
    }
    pub fn invalidate(&mut self) {
        self.tabs.invalidate();
        self.stashes_loaded = false;
        self.loaded = false;
        self.revision = self.revision.wrapping_add(1);
    }
    pub(crate) fn load_files(&mut self, repo: &Repository) -> Result<()> {
        if self.loaded && self.files_commit == self.commit {
            return Ok(());
        }
        let selected = self
            .file_state
            .selected()
            .and_then(|i| self.files.get(i))
            .map(|r| r.path.clone());
        let same = self.files_commit == self.commit;
        self.files = match self.commit {
            Some(oid) => commit_files(repo, oid)?,
            None => working_files(repo)?,
        };
        let index = if same {
            selected
                .and_then(|p| self.files.iter().position(|r| r.path == p))
                .unwrap_or(self.file_state.selected().unwrap_or(0))
        } else {
            0
        };
        self.file_state.select(
            (!self.files.is_empty()).then_some(index.min(self.files.len().saturating_sub(1))),
        );
        if let Some(row) = self.file_state.selected().and_then(|i| self.files.get(i)) {
            if !row.unstaged && row.staged {
                self.staged = true;
            } else if !row.staged {
                self.staged = false;
            }
        }
        self.files_commit = self.commit;
        self.loaded = true;
        Ok(())
    }
}

impl App {
    pub fn workspace_tick(&mut self) {
        if let Some(rx) = &self.workspace.job {
            match rx.try_recv() {
                Ok(result) => {
                    self.workspace.job = None;
                    let message = self.workspace.commit_message.take();
                    match result {
                        Ok(text) => {
                            if message.is_some() {
                                self.workspace.composer = Default::default();
                                self.workspace.commit_error = None;
                                self.workspace.focus = Panel::Files;
                                self.workspace.staged = false;
                            }
                            self.mode = AppMode::Normal;
                            self.set_message(text.trim());
                            if let Err(e) = self.refresh(true) {
                                self.show_error(e.to_string());
                            }
                            self.workspace.commit = None;
                            self.workspace.invalidate();
                        }
                        Err(error) => {
                            if let Some(input) = message {
                                self.mode = AppMode::Input {
                                    title: crate::tr!(
                                        "Commit failed — edit or retry (Esc cancels)"
                                    )
                                    .into(),
                                    input,
                                    action: InputAction::CommitMessage,
                                };
                                self.workspace.composer.field =
                                    crate::commit_composer::Field::Subject;
                                self.workspace.composer.error_scroll = 0;
                                self.workspace.commit_error = Some(error.clone());
                                self.set_message(error);
                            } else {
                                // A failed pull can still update refs or leave conflicts.
                                let _ = self.refresh(true);
                                self.workspace.commit = None;
                                self.workspace.invalidate();
                                self.set_message(crate::tr!(
                                    "Git operation failed; working files refreshed"
                                ));
                                self.show_error(error);
                            }
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.workspace.job = None;
                    self.show_error(crate::tr!("Git worker stopped unexpectedly").into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if !self.workspace.stashes_loaded {
            if let Err(e) = self.workspace_load_stashes() {
                self.set_message(e.to_string());
            }
        }
        if let Err(e) = self.workspace.load_files(&self.repo.repo) {
            self.workspace.files.clear();
            self.workspace.loaded = true;
            self.workspace.preview = e.to_string();
            self.workspace.patch =
                crate::ui::patch::PatchView::build(&self.workspace.preview, None);
            return;
        }
        if !self.composing() {
            self.tree_sync();
        }
        if self.composing() && self.workspace.composer.staged.is_empty() {
            self.workspace.file_state.select(None);
        }
        let key = PreviewKey {
            stash: self.workspace.stash_preview,
            stash_view: self.workspace.focus == Panel::Stashes
                || self.workspace.stash_preview.is_some(),
            revision: self.workspace.revision,
            commit: self.workspace.commit,
            path: self
                .workspace
                .file_state
                .selected()
                .and_then(|i| self.workspace.files.get(i))
                .map(|r| r.path.clone()),
            staged: self.workspace.staged,
        };
        if self.composing() && self.workspace.composer.staged.is_empty() {
            self.workspace.preview = if self
                .workspace
                .composer
                .amend
                .as_ref()
                .is_some_and(|(_, only)| *only)
            {
                crate::tr!("Message-only amend. Staged changes are kept for your next commit.")
                    .into()
            } else if self.workspace.composer.amend.is_some() {
                crate::tr!("No new staged changes. Amend keeps the current HEAD contents.").into()
            } else {
                crate::tr!("No staged changes. Esc returns to Files; the draft is kept.").into()
            };
            self.workspace.patch =
                crate::ui::patch::PatchView::build(&self.workspace.preview, None);
            self.workspace.preview_rx = None;
            self.workspace.preview_key = Some(key);
            return;
        }
        if self.workspace.preview_key.as_ref() != Some(&key) {
            // Once the visible text is cleared, the former key no longer describes it.
            // Returning to that file while another result is in flight must reload it.
            self.workspace.preview_key = None;
            self.workspace.preview = crate::tr!("Loading diff…").into();
            self.workspace.patch =
                crate::ui::patch::PatchView::build(&self.workspace.preview, None);
            self.workspace.hunk_index = 0;
            self.workspace.scroll = 0;
            self.workspace.horizontal = 0;
        }
        if let Some(rx) = &self.workspace.preview_rx {
            match rx.try_recv() {
                Ok((received, result)) => {
                    self.workspace.preview_rx = None;
                    if received == key {
                        let (text, patch) = result.unwrap_or_else(|e| {
                            let patch = crate::ui::patch::PatchView::build(&e, None);
                            (e, patch)
                        });
                        self.workspace.preview = text;
                        self.workspace.patch = patch;
                        self.workspace.preview_key = Some(received);
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.workspace.preview_rx = None;
                    self.workspace.preview = crate::tr!("Diff worker stopped").into();
                    self.workspace.patch =
                        crate::ui::patch::PatchView::build(&self.workspace.preview, None);
                    self.workspace.preview_key = Some(key.clone());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if self.workspace.preview_rx.is_none() && self.workspace.preview_key.as_ref() != Some(&key)
        {
            let (tx, rx) = mpsc::channel();
            self.workspace.preview_rx = Some(rx);
            let path = self.repo_path.clone();
            thread::spawn(move || {
                let result = Repository::open(path)
                    .map_err(anyhow::Error::from)
                    .and_then(|r| preview(&r, &key))
                    .map(|text| {
                        let patch = crate::ui::patch::PatchView::build(
                            &text,
                            if key.stash_view {
                                None
                            } else {
                                key.path.as_deref()
                            },
                        );
                        (text, patch)
                    });
                let _ = tx.send((key, result.map_err(|e| e.to_string())));
            });
        }
    }

    pub fn workspace_pull(&mut self, branch: String) {
        let (tx, rx) = mpsc::channel();
        self.workspace.job = Some(rx);
        let path = self.repo_path.clone();
        self.set_message(crate::tr!("Pulling using Git configuration…"));
        thread::spawn(move || {
            let result = Repository::open(path)
                .map_err(anyhow::Error::from)
                .and_then(|repo| crate::git::operations::pull_branch(&repo, &branch));
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
    }

    pub fn workspace_commit(&mut self, message: String) {
        self.workspace.commit_error = None;
        let (tx, rx) = mpsc::channel();
        self.workspace.job = Some(rx);
        self.workspace.commit_message = Some(message.clone());
        let path = self.repo_path.clone();
        let amend = self.workspace.composer.amend.clone();
        self.set_message(crate::tr!("Committing… (hooks and signing enabled)"));
        thread::spawn(move || {
            let result = Repository::open(path)
                .map_err(anyhow::Error::from)
                .and_then(|repo| {
                    if let Some((snapshot, message_only)) = amend {
                        crate::git::history::amend(&repo, &snapshot, &message, message_only)
                    } else {
                        git_output(&repo, &["commit".as_ref(), "-m".as_ref(), message.as_ref()])
                    }
                });
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
    }

    pub(crate) fn workspace_commit_target(&mut self, oid: Oid) {
        self.workspace.commit = Some(oid);
        self.workspace.stash_preview = None;
        self.workspace.loaded = false;
    }

    pub fn workspace_graph_target(&mut self) {
        self.workspace.stash_preview = None;
        self.workspace.commit = self
            .graph_list_state
            .selected()
            .and_then(|i| self.graph_layout.nodes.get(i))
            .and_then(|n| n.commit.as_ref())
            .map(|c| c.oid);
        self.workspace.loaded = false;
    }

    pub fn workspace_open_diff(&mut self) {
        if self.workspace.focus == Panel::CommitDetail {
            self.workspace_graph_target();
        }
        if self.workspace.focus != Panel::Diff {
            self.workspace.diff_return = self.workspace.focus;
        }
        self.workspace.diff_visible = true;
        self.workspace.focus = Panel::Diff;
    }

    pub fn workspace_focus(&mut self, panel: Panel) {
        if panel == Panel::Diff {
            self.workspace_open_diff();
            return;
        }
        if panel == Panel::CommitDetail {
            self.workspace.diff_visible = false;
            self.workspace_graph_target();
        }
        self.workspace.focus = panel;
        if panel != Panel::Diff {
            self.workspace.stash_preview = None;
        }
        if panel == Panel::Stashes {
            self.workspace_select_stash();
        }
        if panel == Panel::Files {
            self.workspace.commit = None;
            self.workspace.loaded = false;
        }
        if panel == Panel::Graph {
            self.workspace_graph_target();
        }
        if panel == Panel::Branches {
            if self.workspace.branch_state.selected().is_none() && !self.branches.is_empty() {
                self.workspace.branch_state.select(Some(0));
            }
            self.workspace_select_branch();
        }
        self.workspace_tab_target();
    }

    /// Returns true for actions owned by the workspace, false for shared dialogs/operations.
    pub fn workspace_action(&mut self, action: &Action) -> Result<bool> {
        let focus = self.workspace.focus;
        if self.context_git_action(action)? {
            return Ok(true);
        }
        if *action == Action::GitMenu {
            self.open_git_menu()?;
            return Ok(true);
        }
        if self.workspace_tabs_action(action)? {
            return Ok(true);
        }
        if focus == Panel::Files {
            self.workspace.load_files(&self.repo.repo)?;
            self.tree_sync();
        }
        if self.tree_action(action) {
            return Ok(true);
        }
        if matches!(action, Action::TreeLeft | Action::TreeRight) {
            return self.workspace_action(&if matches!(action, Action::TreeLeft) {
                Action::FocusPrevious
            } else {
                Action::FocusNext
            });
        }
        if self.workspace_recovery_action(action)? {
            return Ok(true);
        }
        if self.workspace_changes_action(action)? {
            return Ok(true);
        }
        if self.workspace_stash_action(action)? {
            return Ok(true);
        }
        if matches!(action, Action::CherryPick | Action::Revert) {
            if focus != Panel::Graph {
                self.set_message(crate::tr!("Select a commit in Graph (4) first"));
                return Ok(true);
            }
            let commit = self
                .graph_list_state
                .selected()
                .and_then(|i| self.graph_layout.nodes.get(i))
                .and_then(|n| n.commit.as_ref())
                .context(crate::tr!(
                    "Select a committed change; working files cannot be applied or reverted"
                ))?;
            let kind = if *action == Action::CherryPick {
                crate::git::recovery::Kind::CherryPick
            } else {
                crate::git::recovery::Kind::Revert
            };
            let target = crate::git::commit_operation::CommitOperation::capture(
                &self.repo.repo,
                commit.oid,
                kind,
            )?;
            self.mode = AppMode::Confirm {
                message: target.message(),
                action: crate::app::ConfirmAction::CommitOperation(target),
            };
            return Ok(true);
        }
        if *action == Action::LoadMoreHistory {
            if focus == Panel::Graph {
                self.load_more_history()?;
                self.workspace_graph_target();
            } else {
                self.set_message(crate::tr!("Use Ctrl-l in Graph (4) to load more history"));
            }
            return Ok(true);
        }
        if matches!(action, Action::Merge | Action::Rebase) {
            let branch = match focus {
                Panel::Branches => self.workspace_selected_branch(),
                Panel::Graph => self
                    .selected_branch_name()
                    .filter(|name| self.selected_node_branches().contains(name))
                    .and_then(|name| self.branches.iter().find(|b| b.name == name)),
                _ => None,
            };
            if let Some(branch) = branch {
                let kind = if *action == Action::Merge {
                    crate::git::recovery::Kind::Merge
                } else {
                    crate::git::recovery::Kind::Rebase
                };
                let target =
                    crate::git::integrate::Integration::capture(&self.repo.repo, branch, kind)?;
                self.mode = AppMode::Confirm {
                    message: target.message(),
                    action: crate::app::ConfirmAction::Integrate(target),
                };
            } else {
                self.set_message(crate::tr!(
                    "Select a branch in Branches (3) or Graph (4) first"
                ));
            }
            return Ok(true);
        }
        match action {
            Action::FocusPanel(panel) => self.workspace_focus(*panel),
            Action::FocusNext | Action::FocusPrevious => {
                let mut panels = vec![
                    Panel::Files,
                    Panel::Branches,
                    Panel::Stashes,
                    Panel::Graph,
                    if self.workspace.diff_visible {
                        Panel::Diff
                    } else {
                        Panel::CommitDetail
                    },
                ];
                if !self.workspace.diff_visible {
                    panels.retain(|panel| *panel != Panel::Diff);
                }
                let index = panels.iter().position(|p| *p == focus).unwrap_or(0);
                let next = if *action == Action::FocusNext {
                    (index + 1) % panels.len()
                } else {
                    (index + panels.len() - 1) % panels.len()
                };
                // Cycling into files preserves the commit context; 2 explicitly returns to worktree.
                self.workspace.focus = panels[next];
                if panels[next] != Panel::Diff {
                    self.workspace.stash_preview = None;
                }
                if panels[next] == Panel::Stashes {
                    self.workspace_select_stash();
                }
                if panels[next] == Panel::Graph {
                    self.workspace_graph_target();
                }
                if panels[next] == Panel::Branches {
                    self.workspace_select_branch();
                }
            }
            Action::ToggleHelp => self.workspace.help = !self.workspace.help,
            Action::Quit => self.should_quit = true,
            Action::Cancel if self.workspace.diff_visible => {
                self.workspace.diff_visible = false;
                if self.workspace.focus == Panel::Diff || self.workspace.focus == Panel::Files {
                    self.workspace.focus = self.workspace.diff_return;
                }
            }
            Action::Cancel => {
                self.workspace.stash_preview = None;
                self.workspace.focus = Panel::Files;
                self.workspace.commit = None;
                self.workspace.loaded = false;
            }
            Action::ToggleDiffSide => {
                if self.workspace.commit.is_none() {
                    self.workspace.staged = !self.workspace.staged;
                }
            }
            Action::MoveUp
            | Action::MoveDown
            | Action::PageUp
            | Action::PageDown
            | Action::GoToTop
            | Action::GoToBottom => {
                let delta = match action {
                    Action::MoveUp => -1,
                    Action::MoveDown => 1,
                    Action::PageUp => -10,
                    Action::PageDown => 10,
                    Action::GoToTop => -i32::MAX,
                    _ => i32::MAX,
                };
                match focus {
                    Panel::Files => {
                        move_list(
                            &mut self.workspace.tree.state,
                            self.workspace.tree.nodes.len(),
                            delta,
                        );
                        self.tree_select();
                        if let Some(row) = self
                            .workspace
                            .file_state
                            .selected()
                            .and_then(|i| self.workspace.files.get(i))
                        {
                            self.workspace.staged = row.staged && !row.unstaged;
                        }
                    }
                    Panel::Branches => {
                        move_list(
                            &mut self.workspace.branch_state,
                            self.branches.iter().filter(|b| !b.is_remote).count(),
                            delta,
                        );
                        self.workspace_select_branch();
                    }
                    Panel::Graph => {
                        if matches!(action, Action::GoToTop) {
                            self.select_node(0);
                        } else if matches!(action, Action::GoToBottom) {
                            self.select_node(self.graph_layout.nodes.len().saturating_sub(1));
                        } else {
                            self.move_selection(delta);
                        }
                        self.workspace_graph_target();
                    }
                    Panel::CommitDetail => {
                        let rect = self.workspace.detail_rect;
                        let max = crate::ui::commit_detail::CommitDetailWidget::new(self)
                            .estimated_height(rect.width.saturating_sub(2))
                            .saturating_sub(rect.height.saturating_sub(2));
                        self.detail_scroll = (i64::from(self.detail_scroll) + i64::from(delta))
                            .clamp(0, i64::from(max))
                            as u16;
                    }
                    Panel::Diff => {
                        let max = self
                            .workspace
                            .patch
                            .lines
                            .len()
                            .saturating_sub(self.workspace.diff_height as usize)
                            .min(u16::MAX as usize) as i64;
                        self.workspace.scroll = (i64::from(self.workspace.scroll)
                            + i64::from(delta))
                        .clamp(0, max) as u16;
                    }
                    Panel::Stashes => {
                        move_list(
                            &mut self.workspace.stash_state,
                            self.workspace.stashes.len(),
                            delta,
                        );
                        self.workspace_select_stash();
                    }
                    Panel::Status => {}
                }
            }
            Action::ScrollLeft => {
                self.workspace.horizontal = self.workspace.horizontal.saturating_sub(8)
            }
            Action::ScrollRight => {
                self.workspace.horizontal = self.workspace.horizontal.saturating_add(8)
            }
            Action::EnterFileSelect | Action::StageToggle | Action::Checkout => match focus {
                Panel::Files if *action != Action::Checkout => {
                    if self.workspace.commit.is_some() {
                        self.set_message(crate::tr!(
                            "Historical diff is read-only (2: working files)"
                        ));
                    } else if let Some(row) = self
                        .workspace
                        .file_state
                        .selected()
                        .and_then(|i| self.workspace.files.get(i))
                        .cloned()
                    {
                        toggle_stage(&self.repo.repo, &row, self.workspace.staged)?;
                        self.refresh(true)?;
                        self.workspace.invalidate();
                    }
                }
                Panel::Files => self.workspace_open_diff(),
                Panel::Graph | Panel::CommitDetail => {
                    self.workspace.tabs.files = crate::workspace_tabs::FilesTab::Files;
                    self.workspace_graph_target();
                    self.workspace.diff_visible = true;
                    self.workspace.diff_return = focus;
                    self.workspace.focus = Panel::Files;
                }
                Panel::Branches => {
                    if let Some(branch) = self.workspace_selected_branch().cloned() {
                        if branch.is_remote {
                            crate::git::operations::checkout_remote_branch(
                                &self.repo.repo,
                                &branch.name,
                            )?;
                        } else {
                            crate::git::operations::checkout_branch(&self.repo.repo, &branch.name)?;
                        }
                        self.refresh(true)?;
                        self.workspace.invalidate();
                    }
                }
                _ => {}
            },
            Action::StageAll | Action::UnstageAll => {
                if focus != Panel::Files || self.workspace.commit.is_some() {
                    self.set_message(crate::tr!("Select working files with 2 first"));
                } else {
                    if *action == Action::StageAll {
                        if self.repo.repo.index()?.has_conflicts() {
                            bail!(
                                "{}",
                                crate::tr!(
                                    "Resolve conflicts individually with Space before staging all"
                                )
                            );
                        }
                        git_output(&self.repo.repo, &["add".as_ref(), "--all".as_ref()])?;
                    } else {
                        if self.repo.repo.head().is_ok() {
                            git_output(
                                &self.repo.repo,
                                &[
                                    "restore".as_ref(),
                                    "--staged".as_ref(),
                                    "--".as_ref(),
                                    ".".as_ref(),
                                ],
                            )?;
                        } else {
                            git_output(
                                &self.repo.repo,
                                &[
                                    "rm".as_ref(),
                                    "-r".as_ref(),
                                    "--cached".as_ref(),
                                    "--ignore-unmatch".as_ref(),
                                    "--".as_ref(),
                                    ".".as_ref(),
                                ],
                            )?;
                        }
                    }
                    self.refresh(true)?;
                    self.workspace.invalidate();
                }
            }
            Action::Refresh => {
                self.refresh(true)?;
                self.workspace.invalidate();
            }
            Action::CommitDialog => {
                self.workspace.commit_error = None;
                return Ok(false);
            }
            Action::Search => {
                self.workspace.stash_preview = None;
                self.workspace.focus = Panel::Graph;
                return Ok(false);
            }
            Action::Push => return Ok(false),
            Action::Unavailable => {
                self.set_message(crate::tr!("Not implemented yet; see development plan"))
            }
            Action::CreateBranch | Action::DeleteBranch => {
                if focus == Panel::Branches {
                    if *action == Action::DeleteBranch {
                        if let Some(branch) = self
                            .workspace_selected_branch()
                            .filter(|b| !b.is_head && !b.is_remote)
                        {
                            self.mode = AppMode::Confirm {
                                message: crate::trf!(
                                    "Delete branch '{branch}' ?",
                                    branch = branch.name
                                ),
                                action: crate::app::ConfirmAction::DeleteBranch(
                                    branch.name.clone(),
                                ),
                            };
                        }
                        return Ok(true);
                    }
                    self.workspace_select_branch();
                    return Ok(false);
                }
                self.set_message(crate::tr!("Select branches with 3 first"));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(crate) fn workspace_select_branch(&mut self) {
        if let Some(branch) = self.workspace_selected_branch().cloned() {
            let oid = branch.tip_oid;
            let name = branch.name.clone();
            if let Some(index) = self
                .graph_layout
                .nodes
                .iter()
                .position(|n| n.commit.as_ref().is_some_and(|c| c.oid == oid))
            {
                self.select_node(index);
                self.selected_branch_position = self
                    .branch_positions
                    .iter()
                    .position(|(i, b)| *i == index && *b == name);
                self.workspace_graph_target();
            }
        }
    }
}

fn move_list(state: &mut ListState, len: usize, delta: i32) {
    if len == 0 {
        state.select(None);
        return;
    }
    let next = (state.selected().unwrap_or(0) as i64 + i64::from(delta))
        .clamp(0, len.saturating_sub(1) as i64);
    state.select(Some(next as usize));
}

impl App {
    pub fn workspace_panel_at(&self, x: u16, y: u16) -> Option<Panel> {
        let point = ratatui::layout::Position::new(x, y);
        [
            (self.layout.files, Panel::Files),
            (self.workspace.branches_rect, Panel::Branches),
            (self.workspace.stash_rect, Panel::Stashes),
            (self.layout.graph, Panel::Graph),
            (self.workspace.detail_rect, Panel::CommitDetail),
            (self.layout.commit_detail, Panel::Diff),
            (self.workspace.status_rect, Panel::Status),
        ]
        .into_iter()
        .find(|(rect, _)| rect.contains(point))
        .map(|(_, panel)| panel)
    }
    pub fn workspace_click(&mut self, x: u16, y: u16) {
        if self.workspace.busy() || self.is_pushing() || self.workspace.help {
            return;
        }
        let Some(panel) = self.workspace_panel_at(x, y) else {
            return;
        };
        let double_click = self.last_click.is_some_and(|(time, px, py)| {
            px == x && py == y && time.elapsed() < std::time::Duration::from_millis(400)
        });
        self.last_click = Some((std::time::Instant::now(), x, y));
        if matches!(panel, Panel::Files | Panel::Branches) {
            match self.workspace_tabs_click(panel, x, y, double_click) {
                Ok(true) => return,
                Err(error) => {
                    self.show_error(error.to_string());
                    return;
                }
                _ => {}
            }
        }
        self.workspace.focus = panel;
        if panel != Panel::Diff {
            self.workspace.stash_preview = None;
        }
        if panel == Panel::Stashes {
            self.workspace_select_stash();
        }
        let (rect, offset, len) = match panel {
            Panel::Files => (
                self.layout.files,
                self.workspace.tree.state.offset(),
                self.workspace.tree.nodes.len(),
            ),
            Panel::Stashes => (
                self.workspace.stash_rect,
                self.workspace.stash_state.offset(),
                self.workspace.stashes.len(),
            ),
            Panel::Branches => (
                self.workspace.branches_rect,
                self.workspace.branch_state.offset(),
                self.branches.iter().filter(|b| !b.is_remote).count(),
            ),
            Panel::Graph => (
                self.layout.graph,
                self.graph_list_state.offset(),
                self.graph_layout.nodes.len(),
            ),
            _ => return,
        };
        if y <= rect.y || y >= rect.bottom().saturating_sub(1) {
            return;
        }
        let index = offset + (y - rect.y - 1) as usize;
        if index >= len {
            return;
        }
        match panel {
            Panel::Stashes => {
                self.workspace.stash_state.select(Some(index));
                self.workspace_select_stash();
            }
            Panel::Files => {
                self.workspace.tree.state.select(Some(index));
                self.tree_select();
                if self
                    .workspace
                    .tree
                    .selected()
                    .is_some_and(|n| n.file.is_none())
                {
                    self.tree_action(&Action::Checkout);
                    return;
                }
                if let Some(file) = self.workspace.file_state.selected() {
                    let row = &self.workspace.files[file];
                    self.workspace.staged = row.staged && !row.unstaged;
                }
            }
            Panel::Branches => {
                self.workspace.branch_state.select(Some(index));
                self.workspace_select_branch();
            }
            Panel::Graph => {
                self.select_node(index);
                self.workspace_graph_target();
            }
            _ => {}
        }
        if double_click && matches!(panel, Panel::Files | Panel::Graph) {
            if let Err(error) = self.workspace_action(&Action::Checkout) {
                self.show_error(error.to_string());
            }
        }
    }
}

impl App {
    fn workspace_load_stashes(&mut self) -> Result<()> {
        self.workspace.stashes_loaded = true;
        let old = self
            .workspace
            .stash_state
            .selected()
            .and_then(|i| self.workspace.stashes.get(i))
            .map(|e| e.oid);
        self.workspace.stashes = crate::git::stash::list(&self.repo.repo)?;
        let index = old
            .and_then(|oid| self.workspace.stashes.iter().position(|e| e.oid == oid))
            .unwrap_or(0);
        self.workspace
            .stash_state
            .select((!self.workspace.stashes.is_empty()).then_some(index));
        if self.workspace.focus == Panel::Stashes || self.workspace.stash_preview.is_some() {
            self.workspace_select_stash();
        }
        Ok(())
    }
    fn workspace_select_stash(&mut self) {
        self.workspace.stash_preview = self
            .workspace
            .stash_state
            .selected()
            .and_then(|i| self.workspace.stashes.get(i))
            .map(|e| e.oid);
    }
    fn workspace_stash_action(&mut self, action: &Action) -> Result<bool> {
        use crate::{app::ConfirmAction, git::stash::StashOperation};
        if *action == Action::StashSave {
            self.workspace.commit_error = None;
            self.mode = AppMode::Input {
                title: crate::tr!("Save stash (includes untracked files) — message").into(),
                input: String::new(),
                action: InputAction::StashMessage,
            };
            return Ok(true);
        }
        if self.workspace.focus != Panel::Stashes {
            return Ok(false);
        }
        if !self.workspace.stashes_loaded {
            self.workspace_load_stashes()?;
        }
        let operation = match action {
            Action::StageToggle => StashOperation::Apply,
            Action::StashPop => StashOperation::Pop,
            Action::DeleteBranch => StashOperation::Drop,
            Action::Checkout | Action::EnterFileSelect => {
                if self.workspace.stash_preview.is_some() {
                    self.workspace_open_diff();
                }
                return Ok(true);
            }
            _ => return Ok(false),
        };
        if let Some(entry) = self
            .workspace
            .stash_state
            .selected()
            .and_then(|i| self.workspace.stashes.get(i))
            .cloned()
        {
            self.mode = AppMode::Confirm {
                message: crate::trf!(
                    "{operation} {selector} ({oid})? {message}",
                    operation = operation.label(),
                    selector = entry.selector,
                    oid = &entry.oid.to_string()[..7],
                    message = entry.message
                ),
                action: ConfirmAction::Stash(entry, operation),
            };
        } else {
            self.set_message(crate::tr!("No stashes"));
        }
        Ok(true)
    }
    pub fn workspace_stash_save(&mut self, message: String) {
        self.workspace_git_job(move |repo| crate::git::stash::save(repo, &message));
    }
    pub fn workspace_stash_execute(
        &mut self,
        entry: crate::git::stash::StashEntry,
        operation: crate::git::stash::StashOperation,
    ) {
        self.workspace_git_job(move |repo| crate::git::stash::execute(repo, &entry, operation));
    }
    pub(crate) fn workspace_git_job(
        &mut self,
        operation: impl FnOnce(&Repository) -> Result<String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.workspace.job = Some(rx);
        self.workspace.commit_message = None;
        let path = self.repo_path.clone();
        self.set_message(crate::tr!("Running Git operation…"));
        thread::spawn(move || {
            let result = Repository::open(path)
                .map_err(anyhow::Error::from)
                .and_then(|repo| operation(&repo));
            let _ = tx.send(result.map_err(|e| e.to_string()));
        });
    }
}

impl App {
    fn workspace_changes_action(&mut self, action: &Action) -> Result<bool> {
        use crate::git::changes;
        let focus = self.workspace.focus;
        if focus == Panel::Diff && matches!(action, Action::NextHunk | Action::PrevHunk) {
            let hunks = changes::hunks(&self.workspace.preview);
            if !hunks.is_empty() {
                self.workspace.hunk_index = if *action == Action::NextHunk {
                    (self.workspace.hunk_index + 1).min(hunks.len() - 1)
                } else {
                    self.workspace.hunk_index.saturating_sub(1)
                };
                self.workspace.scroll = self
                    .workspace
                    .patch
                    .hunks
                    .get(self.workspace.hunk_index)
                    .copied()
                    .unwrap_or(0)
                    .min(u16::MAX as usize) as u16;
            }
            return Ok(true);
        }
        let stage = focus == Panel::Diff && *action == Action::StageToggle;
        let discard = focus == Panel::Files && *action == Action::DeleteBranch;
        if !stage && !discard {
            return Ok(false);
        }
        if self.workspace.commit.is_some() || self.workspace.stash_preview.is_some() {
            self.set_message(crate::tr!(
                "Historical and stash diffs are read-only (2: working files)"
            ));
            return Ok(true);
        }
        if self.workspace.preview_loading() {
            self.set_message(crate::tr!("Wait for the current diff to load"));
            return Ok(true);
        }
        let Some(row) = self
            .workspace
            .file_state
            .selected()
            .and_then(|i| self.workspace.files.get(i))
            .cloned()
        else {
            return Ok(true);
        };
        if stage {
            let text = self.workspace.preview.clone();
            let index = self.workspace.hunk_index;
            let staged = self.workspace.staged;
            self.workspace_git_job(move |repo| {
                changes::apply_hunk(repo, &row.path, staged, &text, index)
            });
        } else {
            if self.workspace.staged {
                self.set_message(crate::tr!(
                    "Switch to the unstaged diff with Tab in Diff before discarding"
                ));
                return Ok(true);
            }
            if let Some(key) = &self.workspace.preview_key {
                if preview(&self.repo.repo, key)? != self.workspace.preview {
                    bail!(
                        "{}",
                        crate::tr!("Diff changed; refresh and review it before discarding")
                    );
                }
            }
            let target = changes::prepare_discard(&self.repo.repo, &row.path)?;
            self.mode = AppMode::Confirm {
                message: if target.untracked {
                    crate::trf!(
                        "Delete untracked file {path} permanently?",
                        path = row.path.display()
                    )
                } else {
                    crate::trf!(
                        "Discard ALL unstaged changes in {path}? Staged content is kept.",
                        path = row.path.display()
                    )
                },
                action: crate::app::ConfirmAction::Discard(target),
            };
        }
        Ok(true)
    }
    pub fn workspace_discard(&mut self, target: crate::git::changes::Discard) {
        self.workspace_git_job(move |repo| crate::git::changes::discard(repo, &target));
    }
}

impl App {
    fn workspace_recovery_action(&mut self, action: &Action) -> Result<bool> {
        use crate::{
            app::ConfirmAction,
            external::Request,
            git::recovery::{Operation, Resolution},
        };
        if matches!(action, Action::ContinueOperation | Action::AbortOperation) {
            let operation = Operation::capture(&self.repo.repo)?;
            let abort = *action == Action::AbortOperation;
            self.mode = AppMode::Confirm {
                message: crate::trf!(
                    "{action} {operation}? {detail}",
                    action = if abort {
                        crate::tr!("Abort")
                    } else {
                        crate::tr!("Continue")
                    },
                    operation = operation.kind.command(),
                    detail = if abort {
                        crate::tr!("Git may discard conflict-resolution edits.")
                    } else {
                        crate::tr!("Git will use the terminal for its editor and hooks.")
                    }
                ),
                action: ConfirmAction::Recovery(operation, abort),
            };
            return Ok(true);
        }
        let resolve = matches!(self.workspace.focus, Panel::Files | Panel::Diff)
            && *action == Action::StageToggle;
        if *action != Action::EditFile && !resolve {
            return Ok(false);
        }
        if self.workspace.commit.is_some()
            || self.workspace.stash_preview.is_some()
            || !matches!(self.workspace.focus, Panel::Files | Panel::Diff)
        {
            if *action == Action::EditFile {
                self.set_message(crate::tr!("Select a working file with 2 first"));
                return Ok(true);
            }
            return Ok(false);
        }
        let Some(row) = self
            .workspace
            .file_state
            .selected()
            .and_then(|i| self.workspace.files.get(i))
            .cloned()
        else {
            return Ok(false);
        };
        if *action == Action::EditFile {
            self.workspace.external = Some(Request::Edit(row.path));
            return Ok(true);
        }
        if row.conflict {
            if self.workspace.preview_loading() {
                self.set_message(crate::tr!("Wait for the current diff to load"));
                return Ok(true);
            }
            if let Some(key) = &self.workspace.preview_key {
                if preview(&self.repo.repo, key)? != self.workspace.preview {
                    bail!(
                        "{}",
                        crate::tr!("Diff changed; refresh and review the resolution first")
                    );
                }
            }
            let target = Resolution::capture(&self.repo.repo, &row.path)?;
            self.mode = AppMode::Confirm {
                message: crate::trf!(
                    "Mark {path} resolved and stage current contents? Review any conflict markers before confirming.",
                    path = row.path.display()
                ),
                action: ConfirmAction::Resolve(target),
            };
            return Ok(true);
        }
        Ok(false)
    }
    pub fn workspace_recovery(&mut self, operation: crate::git::recovery::Operation, abort: bool) {
        if abort {
            self.workspace_git_job(move |repo| operation.abort(repo));
        } else {
            self.workspace.external = Some(crate::external::Request::Continue(operation));
        }
    }
    pub fn workspace_resolve(&mut self, target: crate::git::recovery::Resolution) {
        self.workspace_git_job(move |repo| target.stage(repo));
    }
}

fn status_letters(
    repo: &Repository,
    path: &std::path::Path,
    status: Status,
) -> Result<(char, char)> {
    if status.is_conflicted() {
        let index = repo.index()?;
        let stages = (
            index.get_path(path, 1).is_some(),
            index.get_path(path, 2).is_some(),
            index.get_path(path, 3).is_some(),
        );
        return Ok(match stages {
            (true, true, true) => ('U', 'U'),
            (false, true, true) => ('A', 'A'),
            (true, true, false) => ('U', 'D'),
            (true, false, true) => ('D', 'U'),
            (false, true, false) => ('A', 'U'),
            (false, false, true) => ('U', 'A'),
            _ => ('D', 'D'),
        });
    }
    if status.is_wt_new() {
        return Ok(('?', '?'));
    }
    let code = |flags: [Status; 5]| {
        flags
            .into_iter()
            .zip(['A', 'D', 'R', 'T', 'M'])
            .find(|(flag, _)| status.intersects(*flag))
            .map_or(' ', |(_, code)| code)
    };
    Ok((
        code([
            Status::INDEX_NEW,
            Status::INDEX_DELETED,
            Status::INDEX_RENAMED,
            Status::INDEX_TYPECHANGE,
            Status::INDEX_MODIFIED,
        ]),
        code([
            Status::WT_NEW,
            Status::WT_DELETED,
            Status::WT_RENAMED,
            Status::WT_TYPECHANGE,
            Status::WT_MODIFIED,
        ]),
    ))
}
