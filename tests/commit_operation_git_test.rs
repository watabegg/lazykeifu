use git2::{Oid, Repository};
use lazykeifu::git::{
    commit_operation::CommitOperation,
    recovery::{Kind, Operation, Resolution},
};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;
fn git(root: &Path, args: &[&str]) -> String {
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
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn fixture() -> (TempDir, Repository, Oid) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    git(root, &["init", "-b", "main"]);
    for (key, value) in [
        ("user.name", "Test"),
        ("user.email", "test@example.invalid"),
        ("commit.gpgsign", "false"),
        ("core.hooksPath", ".git/test-hooks"),
    ] {
        git(root, &["config", key, value]);
    }
    fs::write(root.join("file"), "base\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "base"]);
    git(root, &["switch", "-c", "topic"]);
    fs::write(root.join("file"), "topic\n").unwrap();
    git(root, &["commit", "-am", "topic"]);
    let oid = Oid::from_str(&git(root, &["rev-parse", "HEAD"])).unwrap();
    git(root, &["switch", "main"]);
    let repo = Repository::open(root).unwrap();
    (temp, repo, oid)
}
fn run(repo: &Repository, target: &CommitOperation) -> bool {
    target
        .command(repo)
        .unwrap()
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap()
        .status
        .success()
}
#[test]
fn cherry_pick_then_revert_preserves_source_branch() {
    let (temp, repo, oid) = fixture();
    let root = temp.path();
    assert!(run(
        &repo,
        &CommitOperation::capture(&repo, oid, Kind::CherryPick).unwrap()
    ));
    assert_eq!(fs::read_to_string(root.join("file")).unwrap(), "topic\n");
    assert!(run(
        &repo,
        &CommitOperation::capture(&repo, oid, Kind::Revert).unwrap()
    ));
    assert_eq!(fs::read_to_string(root.join("file")).unwrap(), "base\n");
    assert_eq!(git(root, &["rev-parse", "topic"]), oid.to_string());
    assert_eq!(git(root, &["rev-list", "--count", "main"]), "3");
}
#[test]
fn both_operations_support_conflict_continue_and_abort() {
    for kind in [Kind::CherryPick, Kind::Revert] {
        for abort in [false, true] {
            let (temp, repo, oid) = fixture();
            let root = temp.path();
            if kind == Kind::Revert {
                git(root, &["merge", "--ff-only", "topic"]);
            }
            fs::write(root.join("file"), "different\n").unwrap();
            git(root, &["commit", "-am", "different"]);
            let old = git(root, &["rev-parse", "HEAD"]);
            assert!(!run(
                &repo,
                &CommitOperation::capture(&repo, oid, kind).unwrap()
            ));
            let operation = Operation::capture(&repo).unwrap();
            assert_eq!(operation.kind, kind);
            assert!(operation.continue_command(&repo).is_err());
            assert!(CommitOperation::capture(&repo, oid, kind).is_err());
            if abort {
                operation.abort(&repo).unwrap();
                assert_eq!(git(root, &["rev-parse", "HEAD"]), old);
                assert_eq!(
                    fs::read_to_string(root.join("file")).unwrap(),
                    "different\n"
                );
            } else {
                fs::write(root.join("file"), "resolved\n").unwrap();
                Resolution::capture(&repo, Path::new("file"))
                    .unwrap()
                    .stage(&repo)
                    .unwrap();
                assert!(operation
                    .continue_command(&repo)
                    .unwrap()
                    .env("GIT_EDITOR", "true")
                    .output()
                    .unwrap()
                    .status
                    .success());
                assert_ne!(git(root, &["rev-parse", "HEAD"]), old);
            }
            assert!(Operation::capture(&repo).is_err());
        }
    }
}
#[test]
fn stale_branch_detached_head_and_merge_commit_are_rejected() {
    let (temp, repo, oid) = fixture();
    let root = temp.path();
    let target = CommitOperation::capture(&repo, oid, Kind::CherryPick).unwrap();
    git(root, &["switch", "-c", "other"]);
    assert!(target.command(&repo).is_err());
    git(root, &["switch", "--detach"]);
    assert!(CommitOperation::capture(&repo, oid, Kind::Revert).is_err());
    git(root, &["switch", "main"]);
    git(
        root,
        &[
            "-c",
            "core.editor=true",
            "merge",
            "--no-ff",
            "topic",
            "-m",
            "merge",
        ],
    );
    let merge = repo.head().unwrap().target().unwrap();
    for kind in [Kind::CherryPick, Kind::Revert] {
        assert!(CommitOperation::capture(&repo, merge, kind).is_err());
    }
}
#[test]
fn dirty_worktree_is_preserved_and_empty_pick_can_be_aborted() {
    let (temp, repo, oid) = fixture();
    let root = temp.path();
    fs::write(root.join("file"), "local\n").unwrap();
    assert!(!run(
        &repo,
        &CommitOperation::capture(&repo, oid, Kind::CherryPick).unwrap()
    ));
    assert_eq!(fs::read_to_string(root.join("file")).unwrap(), "local\n");
    git(root, &["restore", "file"]);
    git(root, &["merge", "--ff-only", "topic"]);
    assert!(!run(
        &repo,
        &CommitOperation::capture(&repo, oid, Kind::CherryPick).unwrap()
    ));
    Operation::capture(&repo).unwrap().abort(&repo).unwrap();
    assert_eq!(git(root, &["rev-parse", "HEAD"]), oid.to_string());
}
