//! Git operations (checkout, merge, rebase, branch operations)

use std::path::Path;

use anyhow::{bail, Context, Result};
use git2::{BranchType, IndexAddOption, Oid, Repository};

/// Checkout a branch
pub fn checkout_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    super::command::git_output(
        repo,
        &["switch".as_ref(), "--".as_ref(), branch_name.as_ref()],
    )?;
    Ok(())
}

/// Checkout a commit without changing any branch ref.
pub fn checkout_commit(repo: &Repository, oid: Oid) -> Result<()> {
    super::command::git_output(
        repo,
        &[
            "switch".as_ref(),
            "--detach".as_ref(),
            "--".as_ref(),
            oid.to_string().as_ref(),
        ],
    )?;
    Ok(())
}

/// Track a remote branch without ever resetting an existing local branch.
pub fn checkout_remote_branch(repo: &Repository, remote_branch: &str) -> Result<()> {
    let remote_ref = repo.find_branch(remote_branch, BranchType::Remote)?;
    let refname = remote_ref
        .get()
        .name()
        .context("Invalid remote reference")?;
    let remote = repo.branch_remote_name(refname)?;
    let remote = remote.as_str().context("Invalid remote name")?;
    let local_name = remote_branch
        .strip_prefix(&format!("{remote}/"))
        .context("Invalid remote branch")?;
    if let Ok(local) = repo.find_branch(local_name, BranchType::Local) {
        if local.get().peel_to_commit()?.id() != remote_ref.get().peel_to_commit()?.id() {
            bail!("Local branch '{local_name}' differs from '{remote_branch}'. Select the local branch or reconcile it with Git; it was not reset.");
        }
        return checkout_branch(repo, local_name);
    }
    super::command::git_output(
        repo,
        &[
            "switch".as_ref(),
            "--track".as_ref(),
            "-c".as_ref(),
            local_name.as_ref(),
            "--".as_ref(),
            remote_branch.as_ref(),
        ],
    )?;
    Ok(())
}

/// Create a new branch
pub fn create_branch(repo: &Repository, branch_name: &str, from_oid: Oid) -> Result<()> {
    let commit = repo.find_commit(from_oid).context("Commit not found")?;

    repo.branch(branch_name, &commit, false)
        .context(format!("Failed to create branch '{}'", branch_name))?;

    Ok(())
}

/// Delete a branch
pub fn delete_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    super::command::git_output(
        repo,
        &[
            "branch".as_ref(),
            "-d".as_ref(),
            "--".as_ref(),
            branch_name.as_ref(),
        ],
    )?;
    Ok(())
}

/// Stage a single path (add to the index, or remove for deleted files)
pub fn stage_path(repo: &Repository, path: &Path) -> Result<()> {
    let workdir = repo
        .workdir()
        .context("Repository has no working directory")?;
    let mut index = repo.index()?;
    if workdir.join(path).exists() {
        index.add_path(path)?;
    } else {
        index.remove_path(path)?;
    }
    index.write()?;
    Ok(())
}

/// Unstage a single path (reset the index entry to HEAD)
pub fn unstage_path(repo: &Repository, path: &Path) -> Result<()> {
    match repo.head() {
        Ok(head) => {
            let commit = head.peel(git2::ObjectType::Commit)?;
            repo.reset_default(Some(&commit), [path])?;
        }
        Err(_) => {
            // Unborn HEAD: unstaging means removing the entry from the index
            let mut index = repo.index()?;
            index.remove_path(path)?;
            index.write()?;
        }
    }
    Ok(())
}

/// Stage all changes (tracked modifications, deletions, and untracked files)
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.update_all(["*"].iter(), None)?;
    index.write()?;
    Ok(())
}

/// Unstage all changes (reset the index to HEAD)
pub fn unstage_all(repo: &Repository) -> Result<()> {
    match repo.head() {
        Ok(head) => {
            let commit = head.peel(git2::ObjectType::Commit)?;
            repo.reset_default(Some(&commit), ["*"])?;
        }
        Err(_) => {
            let mut index = repo.index()?;
            index.clear()?;
            index.write()?;
        }
    }
    Ok(())
}

/// Create a commit from the current index
pub fn create_commit(repo: &Repository, message: &str) -> Result<Oid> {
    let signature = repo
        .signature()
        .context("Cannot determine author (set user.name and user.email)")?;

    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    match &parent {
        Some(parent_commit) if parent_commit.tree_id() == tree_oid => {
            bail!("No staged changes to commit")
        }
        None if index.is_empty() => bail!("No staged changes to commit"),
        _ => {}
    }

    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    )?;
    Ok(oid)
}

/// Push using Git's configured remote, refspecs and push policy.
/// Refuse a branch switch that happened while the confirmation was open.
pub fn push_branch(repo_path: &str, branch: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    ensure_current_branch(&repo, branch)?;
    super::command::git_output(&repo, &["push".as_ref()])?;
    Ok(())
}

pub fn ensure_current_branch(repo: &Repository, branch: &str) -> Result<()> {
    let head = repo.head().context("No current branch")?;
    if !head.is_branch() || head.shorthand() != Some(branch) {
        bail!("Current branch changed; close this dialog and try again");
    }
    Ok(())
}

/// Let Git choose the fetch remote from branch configuration and its defaults.
pub fn fetch_configured(repo_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    super::command::git_output(&repo, &["fetch".as_ref()])?;
    Ok(())
}

/// Respect pull.rebase / pull.ff and branch configuration without overriding policy.
pub fn pull_branch(repo: &Repository, branch: &str) -> Result<String> {
    ensure_current_branch(repo, branch)?;
    super::command::git_output(repo, &["pull".as_ref()])
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::Signature;
    use tempfile::TempDir;

    use super::*;
    use crate::git::{GitRepository, StageState};

    fn init_repo_with_commit() -> (TempDir, Repository) {
        let tempdir = tempfile::tempdir().unwrap();
        let repo = Repository::init(tempdir.path()).unwrap();
        fs::write(tempdir.path().join("base.txt"), "base\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("base.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
        drop(tree);
        (tempdir, repo)
    }

    fn state_of(tempdir: &TempDir, path: &str) -> Option<StageState> {
        let repo = GitRepository::open(tempdir.path()).unwrap();
        repo.stage_states().unwrap().get(Path::new(path)).copied()
    }

    #[test]
    fn stage_and_unstage_untracked_file() {
        let (tempdir, repo) = init_repo_with_commit();
        fs::write(tempdir.path().join("new.txt"), "hello\n").unwrap();
        assert_eq!(state_of(&tempdir, "new.txt"), Some(StageState::Unstaged));

        stage_path(&repo, Path::new("new.txt")).unwrap();
        assert_eq!(state_of(&tempdir, "new.txt"), Some(StageState::Staged));

        unstage_path(&repo, Path::new("new.txt")).unwrap();
        assert_eq!(state_of(&tempdir, "new.txt"), Some(StageState::Unstaged));
    }

    #[test]
    fn stage_deleted_file() {
        let (tempdir, repo) = init_repo_with_commit();
        fs::remove_file(tempdir.path().join("base.txt")).unwrap();
        assert_eq!(state_of(&tempdir, "base.txt"), Some(StageState::Unstaged));

        stage_path(&repo, Path::new("base.txt")).unwrap();
        assert_eq!(state_of(&tempdir, "base.txt"), Some(StageState::Staged));
    }

    #[test]
    fn stage_all_and_unstage_all() {
        let (tempdir, repo) = init_repo_with_commit();
        fs::write(tempdir.path().join("a.txt"), "a\n").unwrap();
        fs::write(tempdir.path().join("base.txt"), "modified\n").unwrap();

        stage_all(&repo).unwrap();
        assert_eq!(state_of(&tempdir, "a.txt"), Some(StageState::Staged));
        assert_eq!(state_of(&tempdir, "base.txt"), Some(StageState::Staged));

        unstage_all(&repo).unwrap();
        assert_eq!(state_of(&tempdir, "a.txt"), Some(StageState::Unstaged));
        assert_eq!(state_of(&tempdir, "base.txt"), Some(StageState::Unstaged));
    }

    #[test]
    fn partially_staged_file_is_reported_as_partial() {
        let (tempdir, repo) = init_repo_with_commit();
        fs::write(tempdir.path().join("base.txt"), "staged change\n").unwrap();
        stage_path(&repo, Path::new("base.txt")).unwrap();
        fs::write(tempdir.path().join("base.txt"), "staged + unstaged\n").unwrap();

        assert_eq!(state_of(&tempdir, "base.txt"), Some(StageState::Partial));
    }

    #[test]
    fn create_commit_commits_staged_changes_only() {
        let (tempdir, repo) = init_repo_with_commit();
        repo.config().unwrap().set_str("user.name", "Test").unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();

        fs::write(tempdir.path().join("staged.txt"), "s\n").unwrap();
        fs::write(tempdir.path().join("unstaged.txt"), "u\n").unwrap();
        stage_path(&repo, Path::new("staged.txt")).unwrap();

        let oid = create_commit(&repo, "add staged.txt").unwrap();
        let commit = repo.find_commit(oid).unwrap();
        assert_eq!(commit.message(), Some("add staged.txt"));
        let tree = commit.tree().unwrap();
        assert!(tree.get_name("staged.txt").is_some());
        assert!(tree.get_name("unstaged.txt").is_none());

        // Unstaged file remains in the working tree
        assert_eq!(
            state_of(&tempdir, "unstaged.txt"),
            Some(StageState::Unstaged)
        );
    }

    #[test]
    fn create_commit_rejects_empty_index() {
        let (_tempdir, repo) = init_repo_with_commit();
        repo.config().unwrap().set_str("user.name", "Test").unwrap();
        repo.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();

        let err = create_commit(&repo, "empty").unwrap_err();
        assert!(err.to_string().contains("No staged changes"));
    }
}
