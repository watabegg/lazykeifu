use lazykeifu::git::checkout::{Changes, Checkout, Target};
use std::{fs, process::Command};
use tempfile::TempDir;

fn git(root: &TempDir, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn fixture() -> (TempDir, git2::Repository, git2::Oid) {
    let root = tempfile::tempdir().unwrap();
    git(&root, &["init", "-b", "main"]);
    git(&root, &["config", "user.name", "Test"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    fs::write(root.path().join("tracked"), "old\n").unwrap();
    fs::write(root.path().join(".gitignore"), "ignored\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "first"]);
    let first = git2::Oid::from_str(&git(&root, &["rev-parse", "HEAD"])).unwrap();
    fs::write(root.path().join("tracked"), "new\n").unwrap();
    git(&root, &["commit", "-am", "second"]);
    git(&root, &["branch", "topic", &first.to_string()]);
    let repo = git2::Repository::open(root.path()).unwrap();
    (root, repo, first)
}
fn dirty(root: &TempDir) {
    fs::write(root.path().join("tracked"), "staged\n").unwrap();
    fs::write(root.path().join("added"), "added\n").unwrap();
    git(root, &["add", "."]);
    fs::write(root.path().join("tracked"), "unstaged\n").unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    fs::write(root.path().join("nested/untracked"), "untracked\n").unwrap();
    fs::write(root.path().join("ignored"), "keep\n").unwrap();
}
#[test]
fn stash_preserves_staged_unstaged_and_untracked_without_applying() {
    let (root, repo, first) = fixture();
    dirty(&root);
    let checkout = Checkout::prepare(&repo, Target::Commit(first)).unwrap();
    assert!(checkout.dirty());
    checkout.execute(&repo, Some(Changes::Stash)).unwrap();
    assert_eq!(repo.head().unwrap().target(), Some(first));
    assert_eq!(
        fs::read_to_string(root.path().join("tracked")).unwrap(),
        "old\n"
    );
    assert!(!root.path().join("nested/untracked").exists());
    assert!(!root.path().join("added").exists());
    git(&root, &["switch", "main"]);
    git(&root, &["stash", "apply", "--index"]);
    assert_eq!(
        fs::read_to_string(root.path().join("tracked")).unwrap(),
        "unstaged\n"
    );
    assert_eq!(git(&root, &["show", ":tracked"]), "staged");
    assert_eq!(
        fs::read_to_string(root.path().join("nested/untracked")).unwrap(),
        "untracked\n"
    );
    assert_eq!(git(&root, &["stash", "list"]).lines().count(), 1);
}
#[test]
fn discard_removes_all_reviewed_changes_and_keeps_ignored_files() {
    let (root, repo, first) = fixture();
    dirty(&root);
    Checkout::prepare(&repo, Target::Branch("topic".into()))
        .unwrap()
        .execute(&repo, Some(Changes::Discard))
        .unwrap();
    assert_eq!(repo.head().unwrap().target(), Some(first));
    assert!(!repo.head_detached().unwrap());
    assert_eq!(git(&root, &["status", "--porcelain"]), "");
    assert_eq!(
        fs::read_to_string(root.path().join("ignored")).unwrap(),
        "keep\n"
    );
    assert!(!root.path().join("added").exists());
    assert!(!root.path().join("nested/untracked").exists());
}
#[test]
fn changed_working_files_invalidate_discard_choice() {
    let (root, repo, first) = fixture();
    dirty(&root);
    let checkout = Checkout::prepare(&repo, Target::Commit(first)).unwrap();
    fs::write(root.path().join("tracked"), "new edit\n").unwrap();
    assert!(checkout.execute(&repo, Some(Changes::Discard)).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("tracked")).unwrap(),
        "new edit\n"
    );
    assert!(root.path().join("nested/untracked").exists());
}
#[test]
fn dirty_checkout_without_a_choice_does_nothing() {
    let (root, repo, first) = fixture();
    dirty(&root);
    let before = git(&root, &["status", "--porcelain"]);
    let checkout = Checkout::prepare(&repo, Target::Commit(first)).unwrap();
    assert!(checkout.execute(&repo, None).is_err());
    assert_eq!(git(&root, &["status", "--porcelain"]), before);
    assert!(!repo.head_detached().unwrap());
}
#[test]
fn remote_checkout_uses_the_same_stash_flow() {
    let (root, repo, first) = fixture();
    git(&root, &["remote", "add", "team", "/unused"]);
    git(
        &root,
        &[
            "update-ref",
            "refs/remotes/team/remote-topic",
            &first.to_string(),
        ],
    );
    dirty(&root);
    Checkout::prepare(&repo, Target::Remote("team/remote-topic".into()))
        .unwrap()
        .execute(&repo, Some(Changes::Stash))
        .unwrap();
    assert_eq!(repo.head().unwrap().shorthand(), Some("remote-topic"));
    assert_eq!(git(&root, &["stash", "list"]).lines().count(), 1);
}

#[test]
fn discard_restores_a_file_removed_from_index_but_left_on_disk() {
    let (root, repo, _) = fixture();
    git(&root, &["rm", "--cached", "tracked"]);
    fs::write(root.path().join("tracked"), "replacement\n").unwrap();
    Checkout::prepare(&repo, Target::Branch("main".into()))
        .unwrap()
        .execute(&repo, Some(Changes::Discard))
        .unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("tracked")).unwrap(),
        "new\n"
    );
    assert_eq!(git(&root, &["status", "--porcelain"]), "");
}
