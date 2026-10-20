use git2::{Repository, Signature};
use lazykeifu::{
    git::operations::{checkout_branch, checkout_remote_branch},
    workspace::{toggle_stage, working_files},
};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

fn repo() -> (TempDir, Repository) {
    let temp = TempDir::new().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    repo.config().unwrap().set_str("user.name", "Test").unwrap();
    repo.config()
        .unwrap()
        .set_str("user.email", "test@example.invalid")
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    (temp, repo)
}
fn commit(repo: &Repository) -> git2::Oid {
    let mut index = repo.index().unwrap();
    index.read(true).unwrap();
    let tree = index.write_tree().unwrap();
    let tree = repo.find_tree(tree).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let signature = Signature::now("Test", "test@example.invalid").unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}
fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn all_working_files_are_reachable_and_literal_pathspecs_are_safe() {
    let (temp, repo) = repo();
    for i in 0..60 {
        fs::write(temp.path().join(format!("{i}.txt")), "change").unwrap();
    }
    fs::write(temp.path().join("[x].txt"), "literal").unwrap();
    fs::write(temp.path().join("x.txt"), "other").unwrap();
    let rows = working_files(&repo).unwrap();
    assert_eq!(rows.len(), 62);
    toggle_stage(
        &repo,
        rows.iter()
            .find(|r| r.path == Path::new("[x].txt"))
            .unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(repo.index().unwrap().len(), 1);
    let row = working_files(&repo)
        .unwrap()
        .into_iter()
        .find(|r| r.path == Path::new("[x].txt"))
        .unwrap();
    toggle_stage(&repo, &row, true).unwrap();
    assert!(repo.index().unwrap().is_empty());
    assert!(temp.path().join("[x].txt").exists());
}
#[test]
fn partial_file_stage_and_unstage_preserve_working_content() {
    let (temp, repo) = repo();
    let file = temp.path().join("file.txt");
    fs::write(&file, "base\n").unwrap();
    git(temp.path(), &["add", "."]);
    commit(&repo);
    fs::write(&file, "staged\n").unwrap();
    git(temp.path(), &["add", "."]);
    fs::write(&file, "working\n").unwrap();
    let row = working_files(&repo).unwrap().remove(0);
    assert!(row.staged && row.unstaged);
    toggle_stage(&repo, &row, true).unwrap();
    let row = working_files(&repo).unwrap().remove(0);
    assert!(!row.staged && row.unstaged);
    assert_eq!(fs::read_to_string(&file).unwrap(), "working\n");
    toggle_stage(&repo, &row, false).unwrap();
    let entry = repo
        .index()
        .unwrap()
        .get_path(Path::new("file.txt"), 0)
        .unwrap();
    assert_eq!(repo.find_blob(entry.id).unwrap().content(), b"working\n");
}
#[test]
fn renamed_file_can_be_unstaged_without_deleting_working_copy() {
    let (temp, repo) = repo();
    fs::write(temp.path().join("old"), "content\n").unwrap();
    git(temp.path(), &["add", "."]);
    commit(&repo);
    git(temp.path(), &["mv", "old", "new"]);
    let row = working_files(&repo).unwrap().remove(0);
    toggle_stage(&repo, &row, true).unwrap();
    assert!(repo
        .index()
        .unwrap()
        .get_path(Path::new("old"), 0)
        .is_some());
    assert!(repo
        .index()
        .unwrap()
        .get_path(Path::new("new"), 0)
        .is_none());
    assert!(temp.path().join("new").exists());
}
#[test]
fn divergent_remote_checkout_never_moves_local_ref_or_head() {
    let (temp, repo) = repo();
    fs::write(temp.path().join("a"), "one").unwrap();
    git(temp.path(), &["add", "."]);
    let old = commit(&repo);
    fs::write(temp.path().join("a"), "two").unwrap();
    git(temp.path(), &["add", "."]);
    let new = commit(&repo);
    repo.remote("origin", "https://example.invalid/repo")
        .unwrap();
    repo.reference("refs/remotes/origin/main", old, true, "fixture")
        .unwrap();
    assert!(checkout_remote_branch(&repo, "origin/main").is_err());
    assert_eq!(repo.head().unwrap().target(), Some(new));
    assert_eq!(fs::read_to_string(temp.path().join("a")).unwrap(), "two");
}
#[test]
fn remote_tracking_checkout_supports_non_origin_and_dirty_changes_are_preserved() {
    let (temp, repo) = repo();
    fs::write(temp.path().join("a"), "one").unwrap();
    git(temp.path(), &["add", "."]);
    let old = commit(&repo);
    repo.remote("team", "https://example.invalid/repo").unwrap();
    repo.reference("refs/remotes/team/topic", old, true, "fixture")
        .unwrap();
    checkout_remote_branch(&repo, "team/topic").unwrap();
    assert_eq!(repo.head().unwrap().shorthand(), Some("topic"));
    assert_eq!(
        repo.find_branch("topic", git2::BranchType::Local)
            .unwrap()
            .upstream()
            .unwrap()
            .name()
            .unwrap(),
        Some("team/topic")
    );
    fs::write(temp.path().join("a"), "two").unwrap();
    git(temp.path(), &["add", "."]);
    commit(&repo);
    fs::write(temp.path().join("a"), "dirty").unwrap();
    assert!(checkout_branch(&repo, "main").is_err());
    assert_eq!(repo.head().unwrap().shorthand(), Some("topic"));
    assert_eq!(fs::read_to_string(temp.path().join("a")).unwrap(), "dirty");
}

#[test]
fn staged_rename_with_new_edits_can_be_staged_again() {
    let (temp, repo) = repo();
    fs::write(temp.path().join("old"), "original content\n").unwrap();
    git(temp.path(), &["add", "."]);
    commit(&repo);
    git(temp.path(), &["mv", "old", "new"]);
    fs::write(temp.path().join("new"), "updated content\n").unwrap();
    let row = working_files(&repo).unwrap().remove(0);
    assert!(row.staged && row.unstaged);
    toggle_stage(&repo, &row, false).unwrap();
    let entry = repo.index().unwrap().get_path(Path::new("new"), 0).unwrap();
    assert_eq!(
        repo.find_blob(entry.id).unwrap().content(),
        b"updated content\n"
    );
    assert!(repo
        .index()
        .unwrap()
        .get_path(Path::new("old"), 0)
        .is_none());
}

#[test]
fn detached_head_history_includes_unreferenced_commits() {
    let (temp, repo) = repo();
    fs::write(temp.path().join("a"), "one").unwrap();
    git(temp.path(), &["add", "."]);
    let first = commit(&repo);
    repo.set_head_detached(first).unwrap();
    fs::write(temp.path().join("a"), "two").unwrap();
    git(temp.path(), &["add", "."]);
    let detached = commit(&repo);
    let wrapper = lazykeifu::git::GitRepository::open(temp.path()).unwrap();
    let commits = wrapper.get_commits(500, false).unwrap();
    assert!(commits.iter().any(|c| c.oid == detached));
}
