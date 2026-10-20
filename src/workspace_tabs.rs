//! Secondary views inside the workspace's two operation panes.
use crate::{
    action::Action,
    app::App,
    git::{BranchInfo, GitRepository},
    workspace::Panel,
};
use anyhow::{bail, Context, Result};
use git2::Oid;
use ratatui::{layout::Rect, widgets::ListState};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilesTab {
    #[default]
    Files,
    Worktrees,
    Submodules,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefsTab {
    #[default]
    Local,
    Remotes,
    Tags,
}
#[derive(Clone, Debug)]
pub struct Row {
    pub label: String,
    pub detail: String,
    pub path: Option<PathBuf>,
    pub oid: Option<Oid>,
    pub branch: Option<BranchInfo>,
}
#[derive(Default)]
pub struct Tabs {
    pub files: FilesTab,
    pub refs: RefsTab,
    pub rows: [Vec<Row>; 4],
    pub states: [ListState; 4],
    pub file_hits: Vec<(Rect, usize)>,
    pub ref_hits: Vec<(Rect, usize)>,
    loaded: bool,
}
impl Tabs {
    pub fn invalidate(&mut self) {
        self.loaded = false;
    }
    pub fn slot(&self, panel: Panel) -> Option<usize> {
        match panel {
            Panel::Files => match self.files {
                FilesTab::Files => None,
                FilesTab::Worktrees => Some(0),
                FilesTab::Submodules => Some(1),
            },
            Panel::Branches => match self.refs {
                RefsTab::Local => None,
                RefsTab::Remotes => Some(2),
                RefsTab::Tags => Some(3),
            },
            _ => None,
        }
    }
}
impl App {
    pub fn workspace_selected_branch(&self) -> Option<&BranchInfo> {
        if self.workspace.tabs.refs == RefsTab::Remotes {
            self.workspace.tabs.states[2]
                .selected()
                .and_then(|i| self.workspace.tabs.rows[2].get(i))
                .and_then(|r| r.branch.as_ref())
        } else if self.workspace.tabs.refs == RefsTab::Local {
            self.workspace
                .branch_state
                .selected()
                .and_then(|i| self.branches.iter().filter(|b| !b.is_remote).nth(i))
        } else {
            None
        }
    }
    pub fn workspace_load_tabs(&mut self) -> Result<()> {
        if self.workspace.tabs.loaded {
            return Ok(());
        }
        let repo = &self.repo.repo;
        let mut rows: [Vec<Row>; 4] = Default::default();
        // --porcelain -z preserves paths containing spaces, tabs and newlines.
        let bytes = crate::git::command::git_bytes(
            repo,
            &[
                "worktree".as_ref(),
                "list".as_ref(),
                "--porcelain".as_ref(),
                "-z".as_ref(),
            ],
            None,
        )?;
        let mut current: Option<Row> = None;
        for field in bytes.split(|b| *b == 0) {
            if let Some(path) = field.strip_prefix(b"worktree ") {
                if let Some(row) = current.take() {
                    rows[0].push(row);
                }
                #[cfg(unix)]
                let path = {
                    use std::os::unix::ffi::OsStringExt;
                    PathBuf::from(std::ffi::OsString::from_vec(path.to_vec()))
                };
                #[cfg(not(unix))]
                let path = PathBuf::from(String::from_utf8_lossy(path).into_owned());
                current = Some(Row {
                    label: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    detail: path.display().to_string(),
                    path: Some(path),
                    oid: None,
                    branch: None,
                });
            } else if let Some(row) = current.as_mut() {
                if let Some(name) = field.strip_prefix(b"branch refs/heads/") {
                    row.label = String::from_utf8_lossy(name).into_owned();
                } else if field == b"detached" {
                    row.label.push_str(" (detached)");
                } else if field.starts_with(b"locked") {
                    row.label.push_str(" (locked)");
                } else if let Some(oid) = field.strip_prefix(b"HEAD ") {
                    row.oid = Oid::from_str(&String::from_utf8_lossy(oid)).ok();
                }
            }
        }
        if let Some(row) = current {
            rows[0].push(row);
        }
        for module in repo.submodules()? {
            let path = repo
                .workdir()
                .context("No working tree")?
                .join(module.path());
            let available = module.open().is_ok();
            rows[1].push(Row {
                label: format!(
                    "{}{}",
                    module.path().display(),
                    if available { "" } else { " (not initialized)" }
                ),
                detail: path.display().to_string(),
                path: available.then_some(path),
                oid: module.head_id(),
                branch: None,
            });
        }
        for branch in BranchInfo::list_all(repo, true)?
            .into_iter()
            .filter(|b| b.is_remote)
        {
            rows[2].push(Row {
                label: branch.name.clone(),
                detail: format!("{} · {}", branch.name, branch.tip_oid),
                oid: Some(branch.tip_oid),
                path: None,
                branch: Some(branch),
            });
        }
        for tag in &self.tags {
            rows[3].push(Row {
                label: tag.name.clone(),
                detail: format!("{} · {}", tag.name, tag.tip_oid),
                oid: Some(tag.tip_oid),
                path: None,
                branch: None,
            });
        }
        for (i, list) in rows.iter().enumerate() {
            let old = self.workspace.tabs.states[i]
                .selected()
                .and_then(|n| self.workspace.tabs.rows[i].get(n))
                .map(|r| (&r.label, &r.path));
            let selection = old
                .and_then(|key| list.iter().position(|r| (&r.label, &r.path) == key))
                .unwrap_or(0);
            self.workspace.tabs.states[i].select((!list.is_empty()).then_some(selection));
        }
        self.workspace.tabs.rows = rows;
        self.workspace.tabs.loaded = true;
        Ok(())
    }
    pub fn workspace_switch_tab(&mut self, panel: Panel, index: usize) -> Result<()> {
        self.workspace.focus = panel;
        match panel {
            Panel::Files => {
                self.workspace.tabs.files =
                    [FilesTab::Files, FilesTab::Worktrees, FilesTab::Submodules][index % 3];
                if self.workspace.tabs.files == FilesTab::Files {
                    self.workspace.commit = None;
                    self.workspace.invalidate();
                }
            }
            Panel::Branches => {
                self.workspace.tabs.refs =
                    [RefsTab::Local, RefsTab::Remotes, RefsTab::Tags][index % 3]
            }
            _ => return Ok(()),
        }
        self.workspace_load_tabs()?;
        if panel == Panel::Branches && self.workspace.tabs.refs == RefsTab::Local {
            self.workspace_select_branch();
        } else {
            self.workspace_tab_target();
        }
        Ok(())
    }
    pub fn workspace_tab_target(&mut self) {
        let Some(slot) = self.workspace.tabs.slot(self.workspace.focus) else {
            return;
        };
        let Some(row) = self.workspace.tabs.states[slot]
            .selected()
            .and_then(|i| self.workspace.tabs.rows[slot].get(i))
            .cloned()
        else {
            return;
        };
        self.set_message(&row.detail);
        if let Some(oid) = row.oid.filter(|_| slot >= 2) {
            if let Some(index) = self
                .graph_layout
                .nodes
                .iter()
                .position(|n| n.commit.as_ref().is_some_and(|c| c.oid == oid))
            {
                self.select_node(index);
            }
            self.workspace_commit_target(oid);
        }
    }
    pub fn workspace_tabs_click(
        &mut self,
        panel: Panel,
        x: u16,
        y: u16,
        double: bool,
    ) -> Result<bool> {
        let hits = if panel == Panel::Files {
            &self.workspace.tabs.file_hits
        } else {
            &self.workspace.tabs.ref_hits
        };
        if let Some((_, index)) = hits
            .iter()
            .find(|(rect, _)| rect.contains(ratatui::layout::Position::new(x, y)))
        {
            let index = *index;
            self.workspace_switch_tab(panel, index)?;
            self.last_click = None;
            return Ok(true);
        }
        let Some(slot) = self.workspace.tabs.slot(panel) else {
            return Ok(false);
        };
        self.workspace.focus = panel;
        self.workspace_load_tabs()?;
        let rect = if panel == Panel::Files {
            self.layout.files
        } else {
            self.workspace.branches_rect
        };
        if y > rect.y && y < rect.bottom().saturating_sub(1) {
            let index = self.workspace.tabs.states[slot].offset() + (y - rect.y - 1) as usize;
            if index < self.workspace.tabs.rows[slot].len() {
                self.workspace.tabs.states[slot].select(Some(index));
                self.workspace_tab_target();
                if double {
                    self.workspace_tabs_action(&Action::Checkout)?;
                }
            }
        }
        Ok(true)
    }
    pub fn workspace_tabs_action(&mut self, action: &Action) -> Result<bool> {
        let focus = self.workspace.focus;
        if matches!(focus, Panel::Files | Panel::Branches)
            && matches!(action, Action::NextHunk | Action::PrevHunk)
        {
            let index = if focus == Panel::Files {
                self.workspace.tabs.files as usize
            } else {
                self.workspace.tabs.refs as usize
            };
            self.workspace_switch_tab(
                focus,
                (index + if *action == Action::NextHunk { 1 } else { 2 }) % 3,
            )?;
            return Ok(true);
        }
        let Some(slot) = self.workspace.tabs.slot(focus) else {
            return Ok(false);
        };
        self.workspace_load_tabs()?;
        let len = self.workspace.tabs.rows[slot].len();
        let state = &mut self.workspace.tabs.states[slot];
        let delta = match action {
            Action::MoveUp => Some(-1),
            Action::MoveDown => Some(1),
            Action::PageUp => Some(-10),
            Action::PageDown => Some(10),
            Action::GoToTop => Some(-i32::MAX),
            Action::GoToBottom => Some(i32::MAX),
            _ => None,
        };
        if let Some(delta) = delta {
            state.select((len > 0).then(|| {
                (state.selected().unwrap_or(0) as i64 + i64::from(delta))
                    .clamp(0, len.saturating_sub(1) as i64) as usize
            }));
            self.workspace_tab_target();
            return Ok(true);
        }
        match action {
            Action::FocusPanel(Panel::Diff) if slot <= 1 => {
                self.set_message("Enter opens the selected repository; choose Files for a diff");
                Ok(true)
            }
            Action::Checkout | Action::StageToggle => {
                let Some(row) = self.workspace.tabs.states[slot]
                    .selected()
                    .and_then(|i| self.workspace.tabs.rows[slot].get(i))
                    .cloned()
                else {
                    return Ok(true);
                };
                if slot <= 1 {
                    if self.is_fetching() {
                        bail!("Wait for fetch to finish before opening another repository");
                    }
                    let path = row
                        .path
                        .context("Submodule is not initialized; initialize it with Git first")?;
                    let mut next = App::from_repository(GitRepository::open(&path)?)?;
                    next.set_message(format!("Opened {}", path.display()));
                    *self = next;
                } else if let Some(branch) = row.branch {
                    crate::git::operations::checkout_remote_branch(&self.repo.repo, &branch.name)?;
                    self.refresh(true)?;
                } else if let Some(oid) = row.oid {
                    self.workspace.commit = Some(oid);
                    self.workspace.tabs.files = FilesTab::Files;
                    self.workspace.diff_visible = true;
                    self.workspace.diff_return = Panel::Branches;
                    self.workspace.focus = Panel::Files;
                    self.workspace.invalidate();
                }
                Ok(true)
            }
            Action::Merge | Action::Rebase if slot == 2 => Ok(false),
            Action::Merge
            | Action::Rebase
            | Action::CreateBranch
            | Action::DeleteBranch
            | Action::StageAll
            | Action::UnstageAll
            | Action::CommitDialog
            | Action::EditFile
            | Action::ToggleDiffSide => {
                self.set_message("Select the Files or Local tab for this operation");
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
