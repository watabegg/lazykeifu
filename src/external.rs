//! Commands that must own the user's terminal (editors, hooks, signing).
use crate::git::{command::git_output, recovery::Operation};
use anyhow::{bail, Context, Result};
use git2::Repository;
use std::{path::PathBuf, process::Command};

#[derive(Debug)]
pub enum Request {
    Edit(PathBuf),
    Continue(Operation),
    Commit(crate::git::commit_operation::CommitOperation),
    Integrate(crate::git::integrate::Integration),
}
impl Request {
    pub fn command(&self, repo: &Repository) -> Result<Command> {
        match self {
            Self::Integrate(target) => target.command(repo),
            Self::Commit(target) => target.command(repo),
            Self::Continue(operation) => operation.continue_command(repo),
            Self::Edit(path) => {
                let root = repo.workdir().context("No working tree")?;
                let full = root.join(path);
                if full.exists() && !full.is_file() {
                    bail!("Selected path is not a regular file");
                }
                let editor = git_output(repo, &["var".as_ref(), "GIT_EDITOR".as_ref()])?;
                if editor.trim().is_empty() {
                    bail!("Git editor is empty; set core.editor or GIT_EDITOR");
                }
                // Git editor values are shell commands. The filename is a separate
                // positional argument, never interpolated into that command.
                let mut command = Command::new("sh");
                command
                    .arg("-c")
                    .arg(format!("{} \"$@\"", editor.trim()))
                    .arg("lazykeifu-editor")
                    .arg(full)
                    .current_dir(root);
                Ok(command)
            }
        }
    }
}
