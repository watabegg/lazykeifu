use git2::Repository;
use lazykeifu::git::{
    integrate::Integration,
    recovery::{Kind, Operation},
    BranchInfo,
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
fn fixture(diverge: bool, conflict: bool) -> (TempDir, Repository) {
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
    git(root, &["switch", "main"]);
    if diverge {
        fs::write(root.join(if conflict { "file" } else { "other" }), "main\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-m", "main"]);
    }
    let repo = Repository::open(root).unwrap();
    (temp, repo)
}
fn capture(repo: &Repository, name: &str, kind: Kind) -> Integration {
    let branch = BranchInfo::list_all(repo, true)
        .unwrap()
        .into_iter()
        .find(|b| b.name == name)
        .unwrap();
    Integration::capture(repo, &branch, kind).unwrap()
}
fn execute(repo: &Repository, target: &Integration) -> bool {
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
fn merge_fast_forward_and_divergent_merge_use_correct_direction() {
    for diverge in [false, true] {
        let (temp, repo) = fixture(diverge, false);
        let root = temp.path();
        let topic = git(root, &["rev-parse", "topic"]);
        let target = capture(&repo, "topic", Kind::Merge);
        assert!(target.message().contains("into current branch 'main'"));
        assert!(execute(&repo, &target));
        assert_eq!(git(root, &["branch", "--show-current"]), "main");
        assert_eq!(git(root, &["rev-parse", "topic"]), topic);
        if diverge {
            assert_eq!(
                repo.head()
                    .unwrap()
                    .peel_to_commit()
                    .unwrap()
                    .parent_count(),
                2
            );
        } else {
            assert_eq!(git(root, &["rev-parse", "HEAD"]), topic);
        }
    }
}
#[test]
fn rebase_replays_current_branch_and_keeps_target() {
    let (temp, repo) = fixture(true, false);
    let root = temp.path();
    let topic = git(root, &["rev-parse", "topic"]);
    let old = git(root, &["rev-parse", "HEAD"]);
    assert!(execute(&repo, &capture(&repo, "topic", Kind::Rebase)));
    assert_eq!(git(root, &["branch", "--show-current"]), "main");
    assert_eq!(git(root, &["rev-parse", "HEAD^"]), topic);
    assert_eq!(git(root, &["rev-parse", "topic"]), topic);
    assert_ne!(git(root, &["rev-parse", "HEAD"]), old);
    assert_eq!(fs::read_to_string(root.join("other")).unwrap(), "main\n");
}
#[test]
fn conflicts_leave_native_state_for_abort() {
    for kind in [Kind::Merge, Kind::Rebase] {
        let (temp, repo) = fixture(true, true);
        let old = git(temp.path(), &["rev-parse", "HEAD"]);
        assert!(!execute(&repo, &capture(&repo, "topic", kind)));
        let operation = Operation::capture(&repo).unwrap();
        assert_eq!(operation.kind, kind);
        assert!(repo.index().unwrap().has_conflicts());
        let branch = BranchInfo::list_all(&repo, false)
            .unwrap()
            .into_iter()
            .find(|b| b.name == "topic")
            .unwrap();
        assert!(Integration::capture(&repo, &branch, kind).is_err());
        operation.abort(&repo).unwrap();
        assert_eq!(git(temp.path(), &["rev-parse", "HEAD"]), old);
    }
}
#[test]
fn changed_head_or_target_and_detached_head_are_rejected() {
    let (temp, repo) = fixture(true, false);
    let root = temp.path();
    let target = capture(&repo, "topic", Kind::Merge);
    git(root, &["switch", "-c", "other"]); // Same OID, different branch.
    assert!(target.command(&repo).is_err());
    git(root, &["switch", "main"]);
    git(root, &["branch", "-f", "topic", "main"]);
    assert!(target.command(&repo).is_err());
    let branch = BranchInfo::list_all(&repo, false)
        .unwrap()
        .into_iter()
        .find(|b| b.name == "topic")
        .unwrap();
    git(root, &["switch", "--detach"]);
    assert!(Integration::capture(&repo, &branch, Kind::Rebase).is_err());
}
#[test]
fn remote_ref_is_unambiguous_and_dirty_changes_are_not_autostashed() {
    for kind in [Kind::Merge, Kind::Rebase] {
        let (temp, repo) = fixture(false, false);
        let root = temp.path();
        git(root, &["update-ref", "refs/remotes/team/topic", "topic"]);
        git(root, &["config", "merge.autoStash", "true"]);
        git(root, &["config", "rebase.autoStash", "true"]);
        let target = capture(&repo, "team/topic", kind);
        fs::write(root.join("file"), "local\n").unwrap();
        assert!(!execute(&repo, &target));
        assert_eq!(fs::read_to_string(root.join("file")).unwrap(), "local\n");
        assert!(git(root, &["stash", "list"]).is_empty());
        git(root, &["restore", "file"]);
        assert!(execute(&repo, &target));
    }
}
#[cfg(unix)]
#[test]
fn merge_respects_commit_hook_failure() {
    use std::os::unix::fs::PermissionsExt;
    let (temp, repo) = fixture(true, false);
    let root = temp.path();
    let hooks = root.join(".git/test-hooks");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-merge-commit");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let old = git(root, &["rev-parse", "HEAD"]);
    assert!(!execute(&repo, &capture(&repo, "topic", Kind::Merge)));
    assert_eq!(git(root, &["rev-parse", "HEAD"]), old);
    assert_eq!(Operation::capture(&repo).unwrap().kind, Kind::Merge);
}
