//! Checkout with an explicit decision for uncommitted changes.
use super::{changes, command::git_output, history::Snapshot, operations};
use anyhow::{bail, Context, Result};
use git2::{BranchType, Oid, Repository, StatusOptions};

#[derive(Clone, Debug)]
pub enum Target {
    Commit(Oid),
    Branch(String),
    Remote(String),
}
impl Target {
    pub fn label(&self) -> String {
        match self {
            Self::Commit(oid) => oid.to_string()[..7].into(),
            Self::Branch(name) | Self::Remote(name) => name.clone(),
        }
    }
    fn oid(&self, repo: &Repository) -> Result<Oid> {
        match self {
            Self::Commit(oid) => Ok(repo.find_commit(*oid)?.id()),
            Self::Branch(name) => Ok(repo
                .find_branch(name, BranchType::Local)?
                .get()
                .peel_to_commit()?
                .id()),
            Self::Remote(name) => {
                let remote = repo.find_branch(name, BranchType::Remote)?;
                let refname = remote.get().name().context("Invalid remote reference")?;
                let remote_name = repo.branch_remote_name(refname)?;
                let local_name = name
                    .strip_prefix(&format!(
                        "{}/",
                        remote_name.as_str().context("Invalid remote name")?
                    ))
                    .context("Invalid remote branch")?;
                let oid = remote.get().peel_to_commit()?.id();
                if let Ok(local) = repo.find_branch(local_name, BranchType::Local) {
                    if local.get().peel_to_commit()?.id() != oid {
                        bail!(crate::trf!("Local branch '{local_name}' differs from '{remote_branch}'. Select the local branch or reconcile it with Git; it was not reset.", local_name = local_name, remote_branch = name));
                    }
                }
                Ok(oid)
            }
        }
    }
    fn execute(&self, repo: &Repository) -> Result<()> {
        match self {
            Self::Commit(oid) => operations::checkout_commit(repo, *oid),
            Self::Branch(name) => operations::checkout_branch(repo, name),
            Self::Remote(name) => operations::checkout_remote_branch(repo, name),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Changes {
    Stash,
    Discard,
}

#[derive(Clone, Debug)]
pub struct Checkout {
    pub target: Target,
    snapshot: Snapshot,
    target_oid: Oid,
    changes: Vec<changes::Discard>,
}
fn capture_changes(repo: &Repository) -> Result<Vec<changes::Discard>> {
    let statuses = repo.statuses(Some(
        StatusOptions::new()
            .include_untracked(true)
            .recurse_untracked_dirs(true),
    ))?;
    statuses
        .iter()
        .map(|entry| {
            let path = super::GitRepository::path_from_bytes(entry.path_bytes());
            changes::snapshot(repo, &path)
        })
        .collect()
}
impl Checkout {
    pub fn prepare(repo: &Repository, target: Target) -> Result<Self> {
        Ok(Self {
            snapshot: Snapshot::capture(repo)?,
            target_oid: target.oid(repo)?,
            changes: capture_changes(repo)?,
            target,
        })
    }
    pub fn dirty(&self) -> bool {
        !self.changes.is_empty()
    }
    pub fn execute(&self, repo: &Repository, choice: Option<Changes>) -> Result<String> {
        self.snapshot.verify(repo)?;
        if self.target.oid(repo)? != self.target_oid || capture_changes(repo)? != self.changes {
            bail!(crate::tr!(
                "Checkout state changed; select the destination again"
            ));
        }
        if self.dirty() {
            match choice.context("Choose how to handle uncommitted changes")? {
                Changes::Stash => {
                    super::stash::save(repo, "lazykeifu: before checkout")?;
                }
                Changes::Discard => {
                    for entry in self.changes.iter().filter(|entry| entry.untracked) {
                        // Only remove files included in the user's decision, never ignored files.
                        let path = repo.workdir().context("No working tree")?.join(&entry.path);
                        match std::fs::remove_file(path) {
                            Ok(()) => {}
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                            Err(e) => return Err(e.into()),
                        }
                    }
                    git_output(
                        repo,
                        &[
                            "restore".as_ref(),
                            "--source=HEAD".as_ref(),
                            "--staged".as_ref(),
                            "--worktree".as_ref(),
                            "--".as_ref(),
                            ".".as_ref(),
                        ],
                    )?;
                }
            }
        }
        self.target.execute(repo).with_context(|| {
            if matches!(choice, Some(Changes::Stash)) {
                crate::tr!("Checkout failed; your changes remain in Stashes").to_string()
            } else {
                crate::tr!("Checkout failed").to_string()
            }
        })?;
        let message = match &self.target {
            Target::Commit(oid) => crate::trf!(
                "Checked out {oid} (detached HEAD)",
                oid = &oid.to_string()[..7]
            ),
            _ => crate::trf!("Switched to {target}", target = self.target.label()),
        };
        Ok(if matches!(choice, Some(Changes::Stash)) {
            format!(
                "{message} · {}",
                crate::tr!("Changes saved in Stashes; not applied")
            )
        } else {
            message
        })
    }
}
