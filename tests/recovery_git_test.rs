use git2::Repository;
use lazykeifu::git::recovery::{Kind, Operation, Resolution};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;
const FILE: &str = "[x] file.txt";
fn git(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap()
}
fn ok(root: &Path, args: &[&str]) {
    let output = git(root, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn conflict(rebase: bool) -> (TempDir, Repository, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    ok(root, &["init", "-b", "main"]);
    ok(root, &["config", "user.name", "Test"]);
    ok(root, &["config", "user.email", "test@example.invalid"]);
    ok(root, &["config", "commit.gpgsign", "false"]);
    ok(
        root,
        &[
            "config",
            "core.hooksPath",
            root.join("no-hooks").to_str().unwrap(),
        ],
    );
    fs::write(root.join(FILE), "base\n").unwrap();
    ok(root, &["add", "."]);
    ok(root, &["commit", "-m", "base"]);
    ok(root, &["switch", "-c", "topic"]);
    fs::write(root.join(FILE), "topic\n").unwrap();
    ok(root, &["commit", "-am", "topic"]);
    ok(root, &["switch", "main"]);
    fs::write(root.join(FILE), "main\n").unwrap();
    ok(root, &["commit", "-am", "main"]);
    if rebase {
        ok(root, &["switch", "topic"]);
    }
    let original = String::from_utf8(git(root, &["rev-parse", "HEAD"]).stdout).unwrap();
    assert!(!git(
        root,
        if rebase {
            &["rebase", "main"]
        } else {
            &["merge", "topic"]
        }
    )
    .status
    .success());
    let repo = Repository::open(root).unwrap();
    (temp, repo, original.trim().to_owned())
}

#[test]
fn resolved_merge_and_rebase_continue_through_native_git() {
    for rebase in [false, true] {
        let (temp, repo, _) = conflict(rebase);
        let operation = Operation::capture(&repo).unwrap();
        assert_eq!(
            operation.kind,
            if rebase { Kind::Rebase } else { Kind::Merge }
        );
        assert!(operation.continue_command(&repo).is_err());
        fs::write(temp.path().join(FILE), "resolved\n").unwrap();
        Resolution::capture(&repo, Path::new(FILE))
            .unwrap()
            .stage(&repo)
            .unwrap();
        assert!(!repo.index().unwrap().has_conflicts());
        let result = operation
            .continue_command(&repo)
            .unwrap()
            .env("GIT_EDITOR", "true")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(repo.state(), git2::RepositoryState::Clean);
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.parent_count(), if rebase { 1 } else { 2 });
        assert_eq!(
            fs::read_to_string(temp.path().join(FILE)).unwrap(),
            "resolved\n"
        );
    }
}

#[test]
fn abort_restores_original_head_for_merge_and_rebase() {
    for rebase in [false, true] {
        let (temp, repo, original) = conflict(rebase);
        let operation = Operation::capture(&repo).unwrap();
        fs::write(temp.path().join(FILE), "in progress\n").unwrap();
        operation.abort(&repo).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap().to_string(), original);
        assert_eq!(repo.state(), git2::RepositoryState::Clean);
        assert_eq!(
            fs::read_to_string(temp.path().join(FILE)).unwrap(),
            if rebase { "topic\n" } else { "main\n" }
        );
        assert!(operation.abort(&repo).is_err());
        assert!(operation.continue_command(&repo).is_err());
    }
}

#[test]
fn resolution_checks_edits_after_confirmation_and_supports_deletion() {
    let (temp, repo, _) = conflict(false);
    let old = Resolution::capture(&repo, Path::new(FILE)).unwrap();
    fs::write(temp.path().join(FILE), "changed after confirmation\n").unwrap();
    assert!(old.stage(&repo).is_err());
    assert!(repo.index().unwrap().has_conflicts());
    fs::remove_file(temp.path().join(FILE)).unwrap();
    Resolution::capture(&repo, Path::new(FILE))
        .unwrap()
        .stage(&repo)
        .unwrap();
    assert!(!repo.index().unwrap().has_conflicts());
    assert!(repo.index().unwrap().get_path(Path::new(FILE), 0).is_none());
}

#[cfg(unix)]
#[test]
fn configured_editor_arguments_and_literal_filename_are_preserved() {
    let (temp, repo, _) = conflict(false);
    let editor = temp.path().join("fake editor.sh");
    fs::write(
        &editor,
        "#!/bin/sh\ntest \"$1\" = '--wait' || exit 4\nprintf edited > \"$2\"\n",
    )
    .unwrap();
    repo.config()
        .unwrap()
        .set_str("core.editor", &format!("sh '{}' --wait", editor.display()))
        .unwrap();
    let path = "$(touch SENTINEL).txt";
    fs::write(temp.path().join(path), "original").unwrap();
    let mut command = lazykeifu::external::Request::Edit(path.into())
        .command(&repo)
        .unwrap();
    assert!(command.status().unwrap().success());
    assert_eq!(
        fs::read_to_string(temp.path().join(path)).unwrap(),
        "edited"
    );
    assert!(!temp.path().join("SENTINEL").exists());
}

#[cfg(unix)]
#[test]
fn failed_merge_continue_keeps_resolution_and_can_be_retried() {
    use std::os::unix::fs::PermissionsExt;
    let (temp, repo, original) = conflict(false);
    let hooks = temp.path().join("hooks");
    fs::create_dir(&hooks).unwrap();
    repo.config()
        .unwrap()
        .set_str("core.hooksPath", hooks.to_str().unwrap())
        .unwrap();
    let hook = hooks.join("pre-commit");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(temp.path().join(FILE), "resolved\n").unwrap();
    Resolution::capture(&repo, Path::new(FILE))
        .unwrap()
        .stage(&repo)
        .unwrap();
    let operation = Operation::capture(&repo).unwrap();
    assert!(!operation
        .continue_command(&repo)
        .unwrap()
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(repo.head().unwrap().target().unwrap().to_string(), original);
    assert_eq!(repo.state(), git2::RepositoryState::Merge);
    assert!(!repo.index().unwrap().has_conflicts());
    fs::remove_file(hook).unwrap();
    assert!(operation
        .continue_command(&repo)
        .unwrap()
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
}

#[test]
fn empty_editor_configuration_does_not_execute_selected_file() {
    let (_temp, repo, _) = conflict(false);
    repo.config().unwrap().set_str("core.editor", "").unwrap();
    assert!(lazykeifu::external::Request::Edit(FILE.into())
        .command(&repo)
        .is_err());
}

#[test]
fn apply_backend_rebase_is_recognized_and_continues() {
    let (temp, repo, _) = conflict(true);
    Operation::capture(&repo).unwrap().abort(&repo).unwrap();
    assert!(!git(
        temp.path(),
        &["-c", "rebase.backend=apply", "rebase", "main"]
    )
    .status
    .success());
    let operation = Operation::capture(&repo).unwrap();
    assert_eq!(operation.kind, Kind::Rebase);
    fs::write(temp.path().join(FILE), "resolved apply backend\n").unwrap();
    Resolution::capture(&repo, Path::new(FILE))
        .unwrap()
        .stage(&repo)
        .unwrap();
    let output = operation
        .continue_command(&repo)
        .unwrap()
        .env("GIT_EDITOR", "true")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
}
