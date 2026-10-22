//! Partial index updates and confirmed working-tree discard.
use super::command::{git_bytes, git_output};
use anyhow::{bail, Context, Result};
use git2::{Oid, Repository};
use std::{
    ffi::OsStr,
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

pub fn diff(repo: &Repository, path: &Path, staged: bool) -> Result<String> {
    let mut args: Vec<&OsStr> = [
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--no-renames",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "--unified=3",
        "--inter-hunk-context=0",
    ]
    .iter()
    .map(OsStr::new)
    .collect();
    if staged {
        args.push(OsStr::new("--cached"));
    }
    args.extend([OsStr::new("--"), path.as_os_str()]);
    String::from_utf8(git_bytes(repo, &args, None)?)
        .context(crate::tr!("Non-UTF-8 diff; use whole-file staging"))
}

pub fn hunks(text: &str) -> Vec<Range<usize>> {
    if !text.starts_with("diff --git ") {
        return Vec::new();
    }
    let lines: Vec<_> = text.lines().collect();
    let starts: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| line.starts_with("@@ ").then_some(i))
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| start..starts.get(i + 1).copied().unwrap_or(lines.len()))
        .collect()
}

pub fn apply_hunk(
    repo: &Repository,
    path: &Path,
    staged: bool,
    expected: &str,
    selected: usize,
) -> Result<String> {
    let status = repo.status_file(path)?;
    if status.intersects(
        git2::Status::WT_NEW
            | git2::Status::INDEX_NEW
            | git2::Status::WT_DELETED
            | git2::Status::INDEX_DELETED
            | git2::Status::INDEX_RENAMED
            | git2::Status::WT_RENAMED,
    ) {
        bail!(crate::tr!(
            "Hunk staging supports modified tracked files; use whole-file staging for this change"
        ));
    }
    let current = diff(repo, path, staged)?;
    if current != expected {
        bail!(crate::tr!(
            "Diff changed; refresh and select the hunk again"
        ));
    }
    let status = repo.status_file(path)?;
    if status.is_conflicted() {
        bail!(crate::tr!("Resolve conflicts before staging hunks"));
    }
    let ranges = hunks(expected);
    let range = ranges
        .get(selected)
        .context(crate::tr!("No text hunk selected; use whole-file staging"))?;
    let lines: Vec<_> = expected.split_inclusive('\n').collect();
    let header = lines[..ranges[0].start].concat();
    // Structural changes are whole-file operations in this first implementation.
    if !header.starts_with("diff --git ")
        || !header.lines().any(|line| {
            line.starts_with("index ") && (line.ends_with(" 100644") || line.ends_with(" 100755"))
        })
        || header.lines().any(|l| {
            l.starts_with("new file")
                || l.starts_with("deleted file")
                || l.starts_with("old mode")
                || l.starts_with("new mode")
                || l.starts_with("rename ")
        })
        || header.contains(" 120000")
        || header.contains(" 160000")
    {
        bail!(crate::tr!("Hunk staging supports modified regular text files; use whole-file staging for this change"));
    }
    let patch = header + &lines[range.clone()].concat();
    let mut args: Vec<&OsStr> = ["apply", "--cached", "--whitespace=nowarn"]
        .iter()
        .map(OsStr::new)
        .collect();
    if staged {
        args.push(OsStr::new("--reverse"));
    }
    git_bytes(repo, &args, Some(patch.as_bytes()))?;
    Ok(if staged {
        crate::tr!("Hunk unstaged")
    } else {
        crate::tr!("Hunk staged")
    }
    .into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Content {
    Missing,
    File(Oid, bool),
    Symlink(PathBuf),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discard {
    pub path: PathBuf,
    pub untracked: bool,
    index: Option<(Oid, u32)>,
    content: Content,
}

pub(crate) fn snapshot(repo: &Repository, path: &Path) -> Result<Discard> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!(crate::tr!(
            "Discard requires a repository-relative file path"
        ));
    }
    let root = repo.workdir().context(crate::tr!("No working tree"))?;
    let mut parent = path.parent();
    while let Some(dir) = parent.filter(|p| !p.as_os_str().is_empty()) {
        if root.join(dir).is_symlink() {
            bail!(crate::tr!("Cannot discard through a symlink directory"));
        }
        parent = dir.parent();
    }
    let full = root.join(path);
    let content = match fs::symlink_metadata(&full) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Content::Missing,
        Err(e) => return Err(e.into()),
        Ok(meta) if meta.file_type().is_symlink() => Content::Symlink(fs::read_link(full)?),
        Ok(meta) if meta.is_file() => {
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode() & 0o111 != 0
            };
            #[cfg(not(unix))]
            let executable = false;
            Content::File(
                Oid::hash_object(git2::ObjectType::Blob, &fs::read(full)?)?,
                executable,
            )
        }
        _ => bail!(crate::tr!(
            "Directory and submodule discard is not supported"
        )),
    };
    let mut index = repo.index()?;
    index.read(true)?;
    let entry = index.get_path(path, 0).map(|e| (e.id, e.mode));
    if repo.status_file(path)?.is_conflicted() {
        bail!(crate::tr!("Resolve conflicts with Git before discarding"));
    }
    Ok(Discard {
        path: path.to_owned(),
        untracked: entry.is_none(),
        index: entry,
        content,
    })
}

pub fn prepare_discard(repo: &Repository, path: &Path) -> Result<Discard> {
    let status = repo.status_file(path)?;
    if !status.intersects(
        git2::Status::WT_MODIFIED
            | git2::Status::WT_DELETED
            | git2::Status::WT_TYPECHANGE
            | git2::Status::WT_NEW,
    ) {
        bail!(crate::tr!(
            "No unstaged changes; unstage first if you want to discard staged content"
        ));
    }
    snapshot(repo, path)
}

pub fn discard(repo: &Repository, expected: &Discard) -> Result<String> {
    if snapshot(repo, &expected.path)? != *expected {
        bail!(crate::tr!(
            "File or index changed; review the diff and confirm discard again"
        ));
    }
    if expected.untracked {
        fs::remove_file(
            repo.workdir()
                .context(crate::tr!("No working tree"))?
                .join(&expected.path),
        )?;
    } else {
        git_output(
            repo,
            &[
                "restore".as_ref(),
                "--worktree".as_ref(),
                "--".as_ref(),
                expected.path.as_os_str(),
            ],
        )?;
    }
    Ok(crate::trf!(
        "Discarded unstaged changes: {path}",
        path = expected.path.display()
    ))
}
