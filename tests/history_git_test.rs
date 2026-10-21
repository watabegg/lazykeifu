use git2::{Oid, Repository};
use lazykeifu::git::history::{self, Rename, Reset, ResetMode, Snapshot};
use std::{fs, process::Command};
fn git(repo: &Repository, args: &[&str]) -> String {
    let o = Command::new("git")
        .args(args)
        .current_dir(repo.workdir().unwrap())
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
fn fixture() -> (tempfile::TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    git(&repo, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(&repo, &["config", "user.name", "Test"]);
    git(&repo, &["config", "user.email", "test@example.invalid"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    git(&repo, &["config", "core.hooksPath", ".git/test-hooks"]);
    git(&repo, &["config", "core.autocrlf", "false"]);
    fs::write(temp.path().join("file"), "base\n").unwrap();
    git(&repo, &["add", "file"]);
    git(&repo, &["commit", "-m", "base"]);
    fs::write(temp.path().join("file"), "second\n").unwrap();
    git(&repo, &["add", "file"]);
    git(&repo, &["commit", "-m", "second"]);
    (temp, repo)
}
#[test]
fn amend_and_reword_preserve_unstaged_and_staged_content() {
    let (t, r) = fixture();
    let parent = r
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .parent_id(0)
        .unwrap();
    fs::write(t.path().join("file"), "staged\n").unwrap();
    git(&r, &["add", "file"]);
    fs::write(t.path().join("file"), "unstaged\n").unwrap();
    history::amend(&r, &Snapshot::capture(&r).unwrap(), "amended", false).unwrap();
    assert_eq!(git(&r, &["show", "HEAD:file"]), "staged\n");
    assert_eq!(
        r.head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .parent_id(0)
            .unwrap(),
        parent
    );
    assert_eq!(
        fs::read_to_string(t.path().join("file")).unwrap(),
        "unstaged\n"
    );
    git(&r, &["add", "file"]);
    let index = git(&r, &["write-tree"]);
    history::amend(&r, &Snapshot::capture(&r).unwrap(), "reworded", true).unwrap();
    assert_eq!(git(&r, &["show", "HEAD:file"]), "staged\n");
    assert_eq!(git(&r, &["write-tree"]), index);
    assert_eq!(git(&r, &["log", "-1", "--format=%s"]).trim(), "reworded");
}
#[test]
fn reset_soft_mixed_and_clean_hard_have_distinct_effects() {
    let (t, r) = fixture();
    let current = Snapshot::capture(&r).unwrap();
    let parent = r.find_commit(current.head).unwrap().parent_id(0).unwrap();
    Reset {
        snapshot: current.clone(),
        target: parent,
        mode: ResetMode::Soft,
    }
    .execute(&r)
    .unwrap();
    assert_eq!(git(&r, &["show", ":file"]), "second\n");
    assert_eq!(
        fs::read_to_string(t.path().join("file")).unwrap(),
        "second\n"
    );
    Reset {
        snapshot: Snapshot::capture(&r).unwrap(),
        target: parent,
        mode: ResetMode::Mixed,
    }
    .execute(&r)
    .unwrap();
    assert_eq!(git(&r, &["show", ":file"]), "base\n");
    assert_eq!(
        fs::read_to_string(t.path().join("file")).unwrap(),
        "second\n"
    );
    assert!(Reset {
        snapshot: Snapshot::capture(&r).unwrap(),
        target: current.head,
        mode: ResetMode::Hard
    }
    .execute(&r)
    .is_err());
    git(&r, &["restore", "file"]);
    Reset {
        snapshot: Snapshot::capture(&r).unwrap(),
        target: current.head,
        mode: ResetMode::Hard,
    }
    .execute(&r)
    .unwrap();
    assert_eq!(
        fs::read_to_string(t.path().join("file")).unwrap(),
        "second\n"
    );
}
#[test]
fn stale_ref_or_index_refuses_history_rewrite() {
    let (t, r) = fixture();
    let snapshot = Snapshot::capture(&r).unwrap();
    git(&r, &["switch", "-c", "other"]);
    assert!(snapshot.verify(&r).is_err());
    git(&r, &["switch", "main"]);
    fs::write(t.path().join("file"), "external\n").unwrap();
    git(&r, &["add", "file"]);
    assert!(Reset {
        snapshot: snapshot.clone(),
        target: snapshot.head,
        mode: ResetMode::Soft
    }
    .execute(&r)
    .is_err());
    assert!(history::amend(&r, &snapshot, "stale", false).is_err());
}
#[test]
fn tags_and_renames_do_not_overwrite_existing_refs() {
    let (_t, r) = fixture();
    let head = Snapshot::capture(&r).unwrap().head;
    history::create_tag(&r, "v1", head).unwrap();
    assert_eq!(r.refname_to_id("refs/tags/v1").unwrap(), head);
    assert!(history::create_tag(&r, "v1", head).is_err());
    assert!(history::create_tag(&r, "--bad", head).is_err());
    git(&r, &["branch", "existing"]);
    let rename = Rename {
        old: "main".into(),
        oid: head,
    };
    assert!(rename.execute(&r, "existing").is_err());
    rename.execute(&r, "renamed").unwrap();
    assert_eq!(r.head().unwrap().shorthand(), Some("renamed"));
    assert!(r.find_reference("refs/heads/main").is_err());
}
#[test]
fn initial_commit_can_be_reworded_without_staging() {
    let (t, r) = fixture();
    git(&r, &["reset", "--hard", "HEAD^"]);
    let original = Snapshot::capture(&r).unwrap();
    history::amend(&r, &original, "root reworded", true).unwrap();
    let commit = r.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(commit.parent_count(), 0);
    assert_ne!(commit.id(), Oid::zero());
    assert_eq!(fs::read_to_string(t.path().join("file")).unwrap(), "base\n");
}
