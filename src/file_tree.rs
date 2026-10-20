//! Visible file rows are separate from Git file indices, so folders never target a file action.
use crate::{
    action::Action,
    app::App,
    workspace::{FileRow, Panel},
};
use ratatui::widgets::ListState;
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

#[derive(Clone)]
pub struct Node {
    pub path: PathBuf,
    pub file: Option<usize>,
    pub depth: usize,
}
#[derive(Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub state: ListState,
    pub closed: HashSet<PathBuf>,
    context: Option<git2::Oid>,
}
impl Tree {
    pub fn selected(&self) -> Option<&Node> {
        self.state.selected().and_then(|i| self.nodes.get(i))
    }
    pub fn rebuild(
        &mut self,
        files: &[FileRow],
        selected: Option<usize>,
        context: Option<git2::Oid>,
    ) {
        let old = if self.context == context {
            self.selected().map(|n| n.path.clone())
        } else {
            None
        };
        self.context = context;
        let target = old.or_else(|| selected.and_then(|i| files.get(i)).map(|f| f.path.clone()));
        let mut all = BTreeMap::new();
        for (i, file) in files.iter().enumerate() {
            for parent in file
                .path
                .ancestors()
                .skip(1)
                .filter(|p| !p.as_os_str().is_empty())
            {
                all.entry(parent.to_path_buf()).or_insert(None);
            }
            all.insert(file.path.clone(), Some(i));
        }
        self.nodes = all
            .into_iter()
            .filter(|(path, _)| !path.ancestors().skip(1).any(|p| self.closed.contains(p)))
            .map(|(path, file)| Node {
                depth: path.components().count().saturating_sub(1),
                path,
                file,
            })
            .collect();
        let index = target.and_then(|path| self.nodes.iter().position(|n| n.path == path));
        self.state.select(if self.nodes.is_empty() {
            None
        } else {
            Some(index.unwrap_or(0))
        });
    }
}
impl App {
    pub(crate) fn tree_sync(&mut self) {
        self.workspace.tree.rebuild(
            &self.workspace.files,
            self.workspace.file_state.selected(),
            self.workspace.commit,
        );
        self.tree_select();
    }
    pub(crate) fn tree_select(&mut self) {
        let file = self.workspace.tree.selected().and_then(|n| n.file);
        if self.workspace.file_state.selected() != file {
            if let Some(row) = file.and_then(|i| self.workspace.files.get(i)) {
                self.workspace.staged = row.staged && !row.unstaged;
            }
        }
        self.workspace.file_state.select(file);
    }
    pub(crate) fn tree_action(&mut self, action: &Action) -> bool {
        if self.workspace.focus != Panel::Files
            || self.workspace.tabs.files != crate::workspace_tabs::FilesTab::Files
        {
            return false;
        }
        let Some(node) = self.workspace.tree.selected().cloned() else {
            return matches!(action, Action::TreeLeft | Action::TreeRight);
        };
        match action {
            Action::TreeLeft => {
                if node.file.is_none() && !self.workspace.tree.closed.contains(&node.path) {
                    self.workspace.tree.closed.insert(node.path);
                    self.tree_sync();
                } else if let Some(parent) = node.path.parent() {
                    if let Some(index) = self
                        .workspace
                        .tree
                        .nodes
                        .iter()
                        .position(|n| n.path == parent)
                    {
                        self.workspace.tree.state.select(Some(index));
                        self.tree_select();
                    }
                }
            }
            Action::TreeRight if node.file.is_none() => {
                if self.workspace.tree.closed.remove(&node.path) {
                    self.tree_sync();
                } else {
                    let next = self.workspace.tree.state.selected().unwrap_or(0) + 1;
                    if next < self.workspace.tree.nodes.len() {
                        self.workspace.tree.state.select(Some(next));
                        self.tree_select();
                    }
                }
            }
            Action::Checkout | Action::StageToggle if node.file.is_none() => {
                if !self.workspace.tree.closed.remove(&node.path) {
                    self.workspace.tree.closed.insert(node.path);
                }
                self.tree_sync();
            }
            Action::TreeRight => {}
            _ => return false,
        }
        true
    }
}
