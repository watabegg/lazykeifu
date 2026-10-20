//! Run Git with repository-local configuration and literal pathspecs.
use anyhow::{bail, Context, Result};
use git2::Repository;
use std::{
    io::Write,
    process::{Command, Stdio},
};

pub fn git_output(repo: &Repository, args: &[&std::ffi::OsStr]) -> Result<String> {
    Ok(String::from_utf8_lossy(&git_bytes(repo, args, None)?).into_owned())
}

pub fn git_bytes(
    repo: &Repository,
    args: &[&std::ffi::OsStr],
    input: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let root = repo.workdir().context("A working tree is required")?;
    let mut child = Command::new("git")
        .arg("--no-pager")
        .args(args)
        .current_dir(root)
        // Stash constructs its own magic pathspecs when cleaning saved untracked
        // files. Literal mode breaks that cleanup; stash calls accept no user paths.
        .env(
            "GIT_LITERAL_PATHSPECS",
            if args
                .first()
                .is_some_and(|arg| *arg == std::ffi::OsStr::new("stash"))
            {
                "0"
            } else {
                "1"
            },
        )
        .env("GIT_MERGE_AUTOEDIT", "no")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Could not run git")?;
    let writer = input.map(|bytes| {
        let bytes = bytes.to_vec();
        let mut stdin = child.stdin.take().expect("piped stdin");
        std::thread::spawn(move || stdin.write_all(&bytes))
    });
    let output = child.wait_with_output()?;
    let write_result = writer
        .map(|handle| {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("Git input worker stopped"))
        })
        .transpose()?;
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
    if let Some(result) = write_result {
        result?;
    }
    Ok(output.stdout)
}
