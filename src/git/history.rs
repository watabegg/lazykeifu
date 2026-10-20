//! Explicit history edits: capture the ref/index being reviewed and verify before writing.
use super::command::git_output;
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository, RepositoryState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub head: Oid,
    pub reference: String,
    pub tree: Oid,
}
impl Snapshot {
    pub fn capture(repo: &Repository) -> Result<Self> {
        if repo.state() != RepositoryState::Clean {
            bail!("Finish or abort the current Git operation first");
        }
        let head = repo.head().context("No commit yet")?;
        let mut index = repo.index()?;
        index.read(true)?;
        if index.has_conflicts() {
            bail!("Resolve conflicts first");
        }
        Ok(Self {
            head: head.peel_to_commit()?.id(),
            reference: head.name().context("Invalid HEAD")?.into(),
            tree: index.write_tree()?,
        })
    }
    pub fn verify(&self, repo: &Repository) -> Result<()> {
        if Self::capture(repo)? != *self {
            bail!("HEAD, branch or index changed. Review and try again");
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}
impl ResetMode {
    pub fn flag(self) -> &'static str {
        match self {
            Self::Soft => "--soft",
            Self::Mixed => "--mixed",
            Self::Hard => "--hard",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Soft => "keep index and working files",
            Self::Mixed => "unstage changes; keep working files",
            Self::Hard => "replace index and working files (requires clean worktree)",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Reset {
    pub snapshot: Snapshot,
    pub target: Oid,
    pub mode: ResetMode,
}
impl Reset {
    pub fn execute(&self, repo: &Repository) -> Result<String> {
        self.snapshot.verify(repo)?;
        repo.find_commit(self.target)?;
        if self.mode == ResetMode::Hard
            && !repo
                .statuses(Some(
                    git2::StatusOptions::new()
                        .include_untracked(true)
                        .recurse_untracked_dirs(true),
                ))?
                .is_empty()
        {
            bail!("Hard reset requires a clean worktree, including untracked files. Commit or stash changes first");
        }
        git_output(
            repo,
            &[
                "reset".as_ref(),
                self.mode.flag().as_ref(),
                self.target.to_string().as_ref(),
                "--".as_ref(),
            ],
        )?;
        Ok(format!(
            "Reset {} to {}",
            self.mode.flag(),
            &self.target.to_string()[..7]
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rename {
    pub old: String,
    pub oid: Oid,
}
impl Rename {
    pub fn execute(&self, repo: &Repository, name: &str) -> Result<String> {
        let branch = repo.find_branch(&self.old, git2::BranchType::Local)?;
        if branch.get().target() != Some(self.oid) {
            bail!("Branch changed; select it again");
        }
        if !git2::Reference::is_valid_name(&format!("refs/heads/{name}")) || name.starts_with('-') {
            bail!("Invalid branch name");
        }
        git_output(
            repo,
            &[
                "branch".as_ref(),
                "-m".as_ref(),
                self.old.as_ref(),
                name.as_ref(),
            ],
        )?;
        Ok(format!("Renamed {} to {name}", self.old))
    }
}
pub fn create_tag(repo: &Repository, name: &str, target: Oid) -> Result<String> {
    if !git2::Reference::is_valid_name(&format!("refs/tags/{name}")) || name.starts_with('-') {
        bail!("Invalid tag name");
    }
    repo.find_commit(target)?;
    git_output(
        repo,
        &[
            "-c".as_ref(),
            "tag.gpgSign=false".as_ref(),
            "tag".as_ref(),
            "--".as_ref(),
            name.as_ref(),
            target.to_string().as_ref(),
        ],
    )?;
    Ok(format!(
        "Created lightweight tag {name} at {}",
        &target.to_string()[..7]
    ))
}
pub fn amend(
    repo: &Repository,
    snapshot: &Snapshot,
    message: &str,
    message_only: bool,
) -> Result<String> {
    snapshot.verify(repo)?;
    let mut args: Vec<&std::ffi::OsStr> = vec!["commit".as_ref(), "--amend".as_ref()];
    if message_only {
        args.push("--only".as_ref());
    }
    args.push("-m".as_ref());
    args.push(message.as_ref());
    git_output(repo, &args)
}
