//! A single, explicitly selected commit applied or reverted by native Git.
use super::recovery::Kind;
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository, RepositoryState};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct CommitOperation {
    kind: Kind,
    oid: Oid,
    subject: String,
    branch: String,
    head: Oid,
}
impl CommitOperation {
    pub fn capture(repo: &Repository, oid: Oid, kind: Kind) -> Result<Self> {
        if !matches!(kind, Kind::CherryPick | Kind::Revert) {
            bail!(crate::tr!("Expected cherry-pick or revert"));
        }
        if repo.state() != RepositoryState::Clean {
            bail!(crate::tr!(
                "Finish or abort the current Git operation first"
            ));
        }
        let head = repo
            .head()
            .context(crate::tr!("Create an initial commit first"))?;
        if !head.is_branch() {
            bail!(crate::tr!(
                "Check out a local branch first; HEAD is detached"
            ));
        }
        let commit = repo.find_commit(oid)?;
        if commit.parent_count() > 1 {
            bail!(crate::tr!(
                "Merge commits require a mainline parent; use Git directly for this commit"
            ));
        }
        Ok(Self {
            kind,
            oid,
            subject: commit.summary().unwrap_or("").to_owned(),
            branch: head
                .name()
                .context(crate::tr!("Branch name is not UTF-8"))?
                .to_owned(),
            head: head.peel_to_commit()?.id(),
        })
    }
    pub fn message(&self) -> String {
        let oid = self.oid.to_string();
        let detail = if self.kind == Kind::Revert {
            crate::tr!("Create a new commit undoing this change.")
        } else {
            crate::tr!("Apply this change as a new commit.")
        };
        crate::trf!(
            "{operation} {oid} ({subject}) on current branch '{branch}'? {detail}",
            operation = self.kind.command(),
            oid = &oid[..7],
            subject = self.subject,
            branch = self.branch.trim_start_matches("refs/heads/"),
            detail = detail
        )
    }
    pub fn command(&self, repo: &Repository) -> Result<Command> {
        let head = repo.head()?;
        if repo.state() != RepositoryState::Clean
            || head.name() != Some(self.branch.as_str())
            || head.target() != Some(self.head)
        {
            bail!(crate::tr!(
                "Branch or operation changed; review and confirm again"
            ));
        }
        let mut command = Command::new("git");
        command
            .arg(self.kind.command())
            .arg(self.oid.to_string())
            .current_dir(repo.workdir().context(crate::tr!("No working tree"))?);
        Ok(command)
    }
}
