//! Native Git operation recovery targets and explicit conflict staging.
use super::command::git_output;
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository, RepositoryState};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}
impl Kind {
    pub fn command(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Rebase => "rebase",
            Self::CherryPick => "cherry-pick",
            Self::Revert => "revert",
        }
    }
}
pub fn kind(repo: &Repository) -> Option<Kind> {
    match repo.state() {
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Some(Kind::CherryPick),
        RepositoryState::Revert | RepositoryState::RevertSequence => Some(Kind::Revert),
        RepositoryState::Merge => Some(Kind::Merge),
        RepositoryState::Rebase
        | RepositoryState::RebaseInteractive
        | RepositoryState::RebaseMerge => Some(Kind::Rebase),
        _ => None,
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub kind: Kind,
    head: Option<Oid>,
    markers: Vec<(String, Vec<u8>)>,
}
impl Operation {
    pub fn capture(repo: &Repository) -> Result<Self> {
        let kind = kind(repo).context("No merge, rebase, cherry-pick or revert in progress")?;
        let mut markers = Vec::new();
        for name in [
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "sequencer/head",
            "sequencer/todo",
            "sequencer/opts",
            "MERGE_HEAD",
            "MERGE_MSG",
            "rebase-merge/orig-head",
            "rebase-merge/onto",
            "rebase-merge/head-name",
            "rebase-merge/msgnum",
            "rebase-merge/git-rebase-todo",
            "rebase-apply/orig-head",
            "rebase-apply/onto",
            "rebase-apply/head-name",
            "rebase-apply/next",
        ] {
            match fs::read(repo.path().join(name)) {
                Ok(bytes) => markers.push((name.into(), bytes)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(Self {
            kind,
            head: repo.head().ok().and_then(|h| h.target()),
            markers,
        })
    }
    pub fn verify(&self, repo: &Repository) -> Result<()> {
        if Self::capture(repo).ok().as_ref() != Some(self) {
            bail!("Git operation changed; review the current state and try again");
        }
        Ok(())
    }
    pub fn abort(&self, repo: &Repository) -> Result<String> {
        self.verify(repo)?;
        git_output(repo, &[self.kind.command().as_ref(), "--abort".as_ref()])?;
        Ok(format!("{} aborted", self.kind.command()))
    }
    pub fn continue_command(&self, repo: &Repository) -> Result<Command> {
        self.verify(repo)?;
        let mut index = repo.index()?;
        index.read(true)?;
        if index.has_conflicts() {
            bail!("Unresolved conflicts remain; edit and stage the resolved files first");
        }
        let mut command = Command::new("git");
        command
            .args([self.kind.command(), "--continue"])
            .current_dir(repo.workdir().context("No working tree")?);
        Ok(command)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub path: PathBuf,
    stages: Vec<(u16, Oid, u32)>,
    content: Option<Oid>,
}
impl Resolution {
    pub fn capture(repo: &Repository, path: &Path) -> Result<Self> {
        let mut index = repo.index()?;
        index.read(true)?;
        let stages: Vec<_> = (1..=3)
            .filter_map(|stage| {
                index
                    .get_path(path, stage)
                    .map(|e| (stage as u16, e.id, e.mode))
            })
            .collect();
        if stages.is_empty() {
            bail!("Conflict state changed; refresh the file list");
        }
        let full = repo.workdir().context("No working tree")?.join(path);
        let content = match fs::symlink_metadata(&full) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
            Ok(meta) if meta.is_file() => {
                Some(Oid::hash_object(git2::ObjectType::Blob, &fs::read(full)?)?)
            }
            _ => bail!("Resolve symlink and submodule conflicts with Git directly"),
        };
        Ok(Self {
            path: path.into(),
            stages,
            content,
        })
    }
    pub fn stage(&self, repo: &Repository) -> Result<String> {
        if Self::capture(repo, &self.path).ok().as_ref() != Some(self) {
            bail!("File or conflict state changed; review and confirm resolution again");
        }
        git_output(
            repo,
            &[
                "add".as_ref(),
                "-A".as_ref(),
                "--".as_ref(),
                self.path.as_os_str(),
            ],
        )?;
        Ok(format!("Resolution staged: {}", self.path.display()))
    }
}
