use git2::{Repository, Signature};
use lazykeifu::git::changes;
use std::{fs, path::Path};
use tempfile::TempDir;

const FILE: &str = "[x] file.txt";
fn original() -> String {
    (0..40).map(|i| format!("line {i}\n")).collect()
}
fn fixture() -> (TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    fs::write(temp.path().join(FILE), original()).unwrap();
    stage(&repo);
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = Signature::now("Test", "test@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    drop(tree);
    (temp, repo)
}
fn stage(repo: &Repository) {
    let mut index = repo.index().unwrap();
    index.read(true).unwrap();
    index.add_path(Path::new(FILE)).unwrap();
    index.write().unwrap();
}
fn indexed(repo: &Repository) -> String {
    let mut index = repo.index().unwrap();
    index.read(true).unwrap();
    String::from_utf8(
        repo.find_blob(index.get_path(Path::new(FILE), 0).unwrap().id)
            .unwrap()
            .content()
            .to_vec(),
    )
    .unwrap()
}

#[test]
fn stage_second_hunk_and_unstage_preserve_working_file() {
    let (temp, repo) = fixture();
    let changed = original()
        .replace("line 2\n", "inserted\nline 2 edited\n")
        .replace("line 30\n", "line 30 edited\n");
    fs::write(temp.path().join(FILE), &changed).unwrap();
    let diff = changes::diff(&repo, Path::new(FILE), false).unwrap();
    assert_eq!(changes::hunks(&diff).len(), 2);
    changes::apply_hunk(&repo, Path::new(FILE), false, &diff, 1).unwrap();
    assert_eq!(
        indexed(&repo),
        original().replace("line 30\n", "line 30 edited\n")
    );
    let staged = changes::diff(&repo, Path::new(FILE), true).unwrap();
    changes::apply_hunk(&repo, Path::new(FILE), true, &staged, 0).unwrap();
    assert_eq!(indexed(&repo), original());
    assert_eq!(fs::read_to_string(temp.path().join(FILE)).unwrap(), changed);
}

#[test]
fn reverse_second_hunk_handles_offsets_and_missing_final_newline() {
    let (temp, repo) = fixture();
    let changed = original()
        .replace("line 2\n", "extra\nextra2\nline 2\n")
        .replace("line 39\n", "last line");
    fs::write(temp.path().join(FILE), &changed).unwrap();
    stage(&repo);
    let diff = changes::diff(&repo, Path::new(FILE), true).unwrap();
    assert_eq!(changes::hunks(&diff).len(), 2);
    changes::apply_hunk(&repo, Path::new(FILE), true, &diff, 1).unwrap();
    assert_eq!(
        indexed(&repo),
        original().replace("line 2\n", "extra\nextra2\nline 2\n")
    );
    assert_eq!(fs::read_to_string(temp.path().join(FILE)).unwrap(), changed);
}

#[test]
fn stale_diff_does_not_stage_or_unstage_new_content() {
    let (temp, repo) = fixture();
    fs::write(
        temp.path().join(FILE),
        original().replace("line 2", "edited 2"),
    )
    .unwrap();
    let shown = changes::diff(&repo, Path::new(FILE), false).unwrap();
    fs::write(temp.path().join(FILE), "external edit\n").unwrap();
    assert!(changes::apply_hunk(&repo, Path::new(FILE), false, &shown, 0).is_err());
    assert_eq!(indexed(&repo), original());
    stage(&repo);
    let shown = changes::diff(&repo, Path::new(FILE), true).unwrap();
    fs::write(temp.path().join(FILE), "external stage\n").unwrap();
    stage(&repo);
    assert!(changes::apply_hunk(&repo, Path::new(FILE), true, &shown, 0).is_err());
    assert_eq!(indexed(&repo), "external stage\n");
}

#[test]
fn binary_new_and_deleted_files_are_rejected_without_mutation() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join(FILE), b"binary\0content").unwrap();
    let diff = changes::diff(&repo, Path::new(FILE), false).unwrap();
    assert!(changes::apply_hunk(&repo, Path::new(FILE), false, &diff, 0).is_err());
    fs::remove_file(temp.path().join(FILE)).unwrap();
    let diff = changes::diff(&repo, Path::new(FILE), false).unwrap();
    assert!(changes::apply_hunk(&repo, Path::new(FILE), false, &diff, 0).is_err());
    assert_eq!(indexed(&repo), original());
    fs::write(temp.path().join("new"), "new\n").unwrap();
    assert!(changes::apply_hunk(&repo, Path::new("new"), false, "", 0).is_err());
}

#[test]
fn discard_preserves_index_and_restores_deleted_files() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join(FILE), "staged\n").unwrap();
    stage(&repo);
    fs::write(temp.path().join(FILE), "unstaged\n").unwrap();
    let snapshot = changes::prepare_discard(&repo, Path::new(FILE)).unwrap();
    changes::discard(&repo, &snapshot).unwrap();
    assert_eq!(indexed(&repo), "staged\n");
    assert_eq!(
        fs::read_to_string(temp.path().join(FILE)).unwrap(),
        "staged\n"
    );
    assert!(changes::prepare_discard(&repo, Path::new(FILE)).is_err());
    fs::remove_file(temp.path().join(FILE)).unwrap();
    changes::discard(
        &repo,
        &changes::prepare_discard(&repo, Path::new(FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join(FILE)).unwrap(),
        "staged\n"
    );
}

#[test]
fn discard_checks_worktree_and_index_after_confirmation() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join(FILE), "first\n").unwrap();
    let snapshot = changes::prepare_discard(&repo, Path::new(FILE)).unwrap();
    fs::write(temp.path().join(FILE), "second\n").unwrap();
    assert!(changes::discard(&repo, &snapshot).is_err());
    stage(&repo);
    fs::write(temp.path().join(FILE), "first\n").unwrap();
    assert!(changes::discard(&repo, &snapshot).is_err());
    assert_eq!(indexed(&repo), "second\n");
    assert_eq!(
        fs::read_to_string(temp.path().join(FILE)).unwrap(),
        "first\n"
    );
}

#[test]
fn untracked_discard_deletes_only_the_literal_selected_file() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join("[a]"), "remove\n").unwrap();
    fs::write(temp.path().join("a"), "keep\n").unwrap();
    let snapshot = changes::prepare_discard(&repo, Path::new("[a]")).unwrap();
    assert!(snapshot.untracked);
    changes::discard(&repo, &snapshot).unwrap();
    assert!(!temp.path().join("[a]").exists());
    assert!(temp.path().join("a").exists());
    assert_eq!(indexed(&repo), original());
}

#[cfg(unix)]
#[test]
fn untracked_symlink_discard_keeps_its_target() {
    let (temp, repo) = fixture();
    std::os::unix::fs::symlink(FILE, temp.path().join("link")).unwrap();
    let snapshot = changes::prepare_discard(&repo, Path::new("link")).unwrap();
    changes::discard(&repo, &snapshot).unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join(FILE)).unwrap(),
        original()
    );
    assert!(!temp.path().join("link").exists());
}
