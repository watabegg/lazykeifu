//! Run Git with repository-local configuration and literal pathspecs.
use anyhow::{bail, Context, Result};
use git2::Repository;
use std::process::{Command, Stdio};

pub fn git_output(repo: &Repository, args: &[&std::ffi::OsStr]) -> Result<String> {
    let root = repo.workdir().context("A working tree is required")?;
    let output = Command::new("git")
        .arg("--no-pager")
        .args(args)
        .current_dir(root)
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("GIT_MERGE_AUTOEDIT", "no")
        .stdin(Stdio::null())
        .output()
        .context("Could not run git")?;
    // CLI mutations bypass libgit2's cached index. Refresh before any later read.
    repo.index()?.read(true)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "Git exited with {}:\n{}\n{}",
            output.status,
            stdout.trim(),
            stderr.trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
