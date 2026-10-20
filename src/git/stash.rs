//! Stash operations keep the selected object stable across list refreshes.
use super::command::git_output;
use anyhow::{bail, Result};
use git2::{ErrorCode, Oid, Repository};

#[derive(Clone, Debug)]
pub struct StashEntry {
    pub selector: String,
    pub oid: Oid,
    pub message: String,
}

#[derive(Clone, Copy, Debug)]
pub enum StashOperation {
    Apply,
    Pop,
    Drop,
}
impl StashOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Apply => crate::tr!("Apply"),
            Self::Pop => crate::tr!("Pop"),
            Self::Drop => crate::tr!("Drop"),
        }
    }
}

pub fn list(repo: &Repository) -> Result<Vec<StashEntry>> {
    let log = match repo.reflog("refs/stash") {
        Ok(log) => log,
        Err(e) if e.code() == ErrorCode::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(log
        .iter()
        .enumerate()
        .map(|(index, entry)| StashEntry {
            selector: format!("stash@{{{index}}}"),
            oid: entry.id_new(),
            message: entry.message().unwrap_or("(stash)").to_owned(),
        })
        .collect())
}

pub fn save(repo: &Repository, message: &str) -> Result<String> {
    git_output(
        repo,
        &[
            "stash".as_ref(),
            "push".as_ref(),
            "--include-untracked".as_ref(),
            "-m".as_ref(),
            message.as_ref(),
        ],
    )
}

fn verify(repo: &Repository, entry: &StashEntry) -> Result<()> {
    if repo.revparse_single(&entry.selector).ok().map(|o| o.id()) != Some(entry.oid) {
        bail!(crate::tr!(
            "Stash list changed; refresh and select the stash again"
        ));
    }
    Ok(())
}

pub fn execute(repo: &Repository, entry: &StashEntry, operation: StashOperation) -> Result<String> {
    verify(repo, entry)?;
    if matches!(operation, StashOperation::Apply | StashOperation::Pop) {
        // Apply the captured object, never a selector that can shift beneath us.
        git_output(
            repo,
            &[
                "stash".as_ref(),
                "apply".as_ref(),
                entry.oid.to_string().as_ref(),
            ],
        )?;
    }
    if matches!(operation, StashOperation::Drop | StashOperation::Pop) {
        // Failed/conflicting apply returns above and retains the stash.
        verify(repo, entry)?;
        git_output(
            repo,
            &["stash".as_ref(), "drop".as_ref(), entry.selector.as_ref()],
        )?;
    }
    Ok(crate::trf!(
        "{operation} completed: {stash}",
        operation = operation.label(),
        stash = entry.selector
    ))
}

pub fn preview(repo: &Repository, oid: Oid) -> Result<String> {
    git_output(
        repo,
        &[
            "stash".as_ref(),
            "show".as_ref(),
            "--patch".as_ref(),
            "--include-untracked".as_ref(),
            "--no-ext-diff".as_ref(),
            "--no-textconv".as_ref(),
            "--color=never".as_ref(),
            oid.to_string().as_ref(),
        ],
    )
}
