//! Confirmed merge/rebase starts, using Git's editor, hooks and conflict state.
use super::{recovery::Kind, BranchInfo};
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository, RepositoryState};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Integration {
    pub kind: Kind,
    current_ref: String,
    current_oid: Oid,
    target_ref: String,
    target_oid: Oid,
    target_name: String,
}
impl Integration {
    pub fn capture(repo: &Repository, branch: &BranchInfo, kind: Kind) -> Result<Self> {
        if repo.state() != RepositoryState::Clean {
            bail!("Finish or abort the current Git operation first");
        }
        let head = repo.head().context("Create an initial commit first")?;
        if !head.is_branch() {
            bail!("Check out a local branch first; HEAD is detached");
        }
        let current_ref = head.name().context("Branch name is not UTF-8")?.to_owned();
        let target_ref = format!(
            "refs/{}/{}",
            if branch.is_remote { "remotes" } else { "heads" },
            branch.name
        );
        if current_ref == target_ref {
            bail!("Select a different branch");
        }
        let target_oid = repo.find_reference(&target_ref)?.peel_to_commit()?.id();
        if target_oid != branch.tip_oid {
            bail!("Selected branch changed; refresh and select it again");
        }
        Ok(Self {
            kind,
            current_ref,
            current_oid: head.peel_to_commit()?.id(),
            target_ref,
            target_oid,
            target_name: branch.name.clone(),
        })
    }
    pub fn message(&self) -> String {
        let current = self
            .current_ref
            .strip_prefix("refs/heads/")
            .unwrap_or(&self.current_ref);
        let source = format!("'{}' ({:.7})", self.target_name, self.target_oid);
        match self.kind {
            Kind::Merge => format!("Merge {source} into current branch '{current}'?"),
            Kind::Rebase => format!("Rebase current branch '{current}' onto {source}? Current branch commits may be rewritten."),
        }
    }
    pub fn command(&self, repo: &Repository) -> Result<Command> {
        let head = repo.head()?;
        let target = repo
            .find_reference(&self.target_ref)
            .ok()
            .and_then(|r| r.peel_to_commit().ok());
        if repo.state() != RepositoryState::Clean
            || head.name() != Some(self.current_ref.as_str())
            || head.target() != Some(self.current_oid)
            || target.map(|c| c.id()) != Some(self.target_oid)
        {
            bail!("Branch or operation changed; review and confirm again");
        }
        let mut command = Command::new("git");
        command
            .args([self.kind.command(), "--no-autostash", &self.target_ref])
            .current_dir(repo.workdir().context("No working tree")?);
        Ok(command)
    }
}
