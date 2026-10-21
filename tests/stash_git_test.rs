use git2::{Repository, Signature};
use lazykeifu::git::stash::{self, StashOperation};
use std::{fs, path::Path};
use tempfile::TempDir;

fn fixture() -> (TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config
        .set_str("user.email", "test@example.invalid")
        .unwrap();
    config.set_bool("commit.gpgsign", false).unwrap();
    commit(&repo, "base\n");
    (temp, repo)
}
fn commit(repo: &Repository, contents: &str) {
    fs::write(repo.workdir().unwrap().join("file"), contents).unwrap();
    let mut index = repo.index().unwrap();
    index.read(true).unwrap();
    index.add_path(Path::new("file")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let sig = Signature::now("Test", "test@example.invalid").unwrap();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        "fixture",
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap();
}

#[test]
fn save_and_preview_include_untracked_but_not_ignored_files() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join("file"), "staged\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("file")).unwrap();
    index.write().unwrap();
    fs::write(temp.path().join("file"), "unstaged\n").unwrap();
    fs::write(temp.path().join("new file"), "untracked contents\n").unwrap();
    fs::write(repo.path().join("info/exclude"), "ignored\n").unwrap();
    fs::write(temp.path().join("ignored"), "keep\n").unwrap();
    stash::save(&repo, "snapshot").unwrap();
    let entries = stash::list(&repo).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].message.contains("snapshot"));
    assert_eq!(
        fs::read_to_string(temp.path().join("file")).unwrap(),
        "base\n"
    );
    assert!(!temp.path().join("new file").exists());
    assert!(temp.path().join("ignored").exists());
    let preview = stash::preview(&repo, entries[0].oid).unwrap();
    assert!(preview.contains("+unstaged"));
    assert!(preview.contains("+untracked contents"));
    let staged_tree = repo
        .find_commit(entries[0].oid)
        .unwrap()
        .parent(1)
        .unwrap()
        .tree()
        .unwrap();
    let blob = staged_tree
        .get_path(Path::new("file"))
        .unwrap()
        .to_object(&repo)
        .unwrap()
        .peel_to_blob()
        .unwrap();
    assert_eq!(blob.content(), b"staged\n");
    stash::execute(&repo, &entries[0], StashOperation::Apply).unwrap();
    assert_eq!(stash::list(&repo).unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(temp.path().join("file")).unwrap(),
        "unstaged\n"
    );
    assert!(temp.path().join("new file").exists());
}

#[test]
fn pop_removes_only_the_applied_entry() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join("file"), "older\n").unwrap();
    stash::save(&repo, "older").unwrap();
    fs::write(temp.path().join("file"), "newer\n").unwrap();
    stash::save(&repo, "newer").unwrap();
    let entries = stash::list(&repo).unwrap();
    stash::execute(&repo, &entries[1], StashOperation::Pop).unwrap();
    assert_eq!(stash::list(&repo).unwrap()[0].oid, entries[0].oid);
    assert_eq!(
        fs::read_to_string(temp.path().join("file")).unwrap(),
        "older\n"
    );
}

#[test]
fn conflicting_pop_keeps_stash_and_conflicts() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join("file"), "stash change\n").unwrap();
    stash::save(&repo, "conflict").unwrap();
    let entry = stash::list(&repo).unwrap().remove(0);
    commit(&repo, "new base\n");
    assert!(stash::execute(&repo, &entry, StashOperation::Pop).is_err());
    assert_eq!(stash::list(&repo).unwrap()[0].oid, entry.oid);
    assert!(repo.index().unwrap().has_conflicts());
}

#[test]
fn shifted_list_rejects_apply_pop_and_drop_without_changing_worktree() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join("file"), "older\n").unwrap();
    stash::save(&repo, "older").unwrap();
    let entry = stash::list(&repo).unwrap().remove(0);
    fs::write(temp.path().join("file"), "newer\n").unwrap();
    stash::save(&repo, "newer").unwrap();
    for op in [
        StashOperation::Apply,
        StashOperation::Pop,
        StashOperation::Drop,
    ] {
        assert!(stash::execute(&repo, &entry, op)
            .unwrap_err()
            .to_string()
            .contains("Stash list changed"));
    }
    assert_eq!(stash::list(&repo).unwrap().len(), 2);
    assert_eq!(
        fs::read_to_string(temp.path().join("file")).unwrap(),
        "base\n"
    );
}

#[test]
fn drop_does_not_change_worktree_or_head_and_empty_list_is_supported() {
    let (temp, repo) = fixture();
    assert!(stash::list(&repo).unwrap().is_empty());
    fs::write(temp.path().join("file"), "saved\n").unwrap();
    stash::save(&repo, "discard me").unwrap();
    let entry = stash::list(&repo).unwrap().remove(0);
    fs::write(temp.path().join("file"), "unsaved\n").unwrap();
    let head = repo.head().unwrap().target();
    stash::execute(&repo, &entry, StashOperation::Drop).unwrap();
    assert!(stash::list(&repo).unwrap().is_empty());
    assert_eq!(repo.head().unwrap().target(), head);
    assert_eq!(
        fs::read_to_string(temp.path().join("file")).unwrap(),
        "unsaved\n"
    );
}

#[test]
fn apply_refuses_to_overwrite_dirty_working_files() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join("file"), "saved\n").unwrap();
    stash::save(&repo, "saved").unwrap();
    let entry = stash::list(&repo).unwrap().remove(0);
    fs::write(temp.path().join("file"), "unsaved\n").unwrap();
    assert!(stash::execute(&repo, &entry, StashOperation::Pop).is_err());
    assert_eq!(stash::list(&repo).unwrap()[0].oid, entry.oid);
    assert_eq!(
        fs::read_to_string(temp.path().join("file")).unwrap(),
        "unsaved\n"
    );
}

#[test]
fn unborn_save_fails_without_removing_files() {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    fs::write(temp.path().join("new"), "unsaved\n").unwrap();
    assert!(stash::save(&repo, "initial").is_err());
    assert!(stash::list(&repo).unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(temp.path().join("new")).unwrap(),
        "unsaved\n"
    );
}
