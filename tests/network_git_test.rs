use git2::{Repository, Signature};
use pedigit::git::operations::{fetch_configured, pull_branch, push_branch};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

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

fn commit(repo: &Repository, text: &str) -> git2::Oid {
    fs::write(repo.workdir().unwrap().join("file"), text).unwrap();
    let mut index = repo.index().unwrap();
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
    .unwrap()
}

struct Fixture {
    _temp: TempDir,
    local: Repository,
    peer: Repository,
    remote: Repository,
    push_remote: Repository,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let remote = Repository::init_bare(temp.path().join("remote.git")).unwrap();
        remote.set_head("refs/heads/main").unwrap();
        let push_remote = Repository::init_bare(temp.path().join("publish.git")).unwrap();
        let local = Repository::init(temp.path().join("local")).unwrap();
        local.set_head("refs/heads/main").unwrap();
        local
            .remote("team", remote.path().to_str().unwrap())
            .unwrap();
        local
            .remote("publish", push_remote.path().to_str().unwrap())
            .unwrap();
        commit(&local, "base\n");
        git(local.workdir().unwrap(), &["push", "-u", "team", "main"]);
        let peer =
            Repository::clone(remote.path().to_str().unwrap(), temp.path().join("peer")).unwrap();
        for repo in [&local, &peer] {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config
                .set_str("user.email", "test@example.invalid")
                .unwrap();
            config.set_bool("commit.gpgsign", false).unwrap();
            config
                .set_str(
                    "core.hooksPath",
                    temp.path().join("empty-hooks").to_str().unwrap(),
                )
                .unwrap();
            config.set_str("pull.ff", "only").unwrap();
            config.set_str("push.default", "simple").unwrap();
        }
        Self {
            _temp: temp,
            local,
            peer,
            remote,
            push_remote,
        }
    }
    fn path(&self) -> &str {
        self.local.workdir().unwrap().to_str().unwrap()
    }
}

#[test]
fn fetch_and_pull_use_non_origin_upstream_and_preserve_dirty_files() {
    let f = Fixture::new();
    let next = commit(&f.peer, "remote change\n");
    git(f.peer.workdir().unwrap(), &["push"]);
    let before = f.local.head().unwrap().target();
    fetch_configured(f.path()).unwrap();
    assert_eq!(
        f.local
            .find_reference("refs/remotes/team/main")
            .unwrap()
            .target(),
        Some(next)
    );
    assert_eq!(f.local.head().unwrap().target(), before);
    fs::write(f.local.workdir().unwrap().join("file"), "unsaved\n").unwrap();
    assert!(pull_branch(&f.local, "main").is_err());
    assert_eq!(
        fs::read_to_string(f.local.workdir().unwrap().join("file")).unwrap(),
        "unsaved\n"
    );
    git(f.local.workdir().unwrap(), &["restore", "file"]);
    pull_branch(&f.local, "main").unwrap();
    assert_eq!(f.local.head().unwrap().target(), Some(next));
}

#[test]
fn push_uses_pushremote_without_rewriting_pull_upstream() {
    let f = Fixture::new();
    let original = f.remote.head().unwrap().target();
    f.local
        .config()
        .unwrap()
        .set_str("branch.main.pushRemote", "publish")
        .unwrap();
    let next = commit(&f.local, "local change\n");
    push_branch(f.path(), "main").unwrap();
    assert_eq!(
        f.push_remote
            .find_reference("refs/heads/main")
            .unwrap()
            .target(),
        Some(next)
    );
    assert_eq!(f.remote.head().unwrap().target(), original);
    assert_eq!(
        f.local
            .config()
            .unwrap()
            .get_string("branch.main.remote")
            .unwrap(),
        "team"
    );
}

#[test]
fn divergent_pull_and_rejected_push_do_not_force_update() {
    let f = Fixture::new();
    let remote = commit(&f.peer, "remote change\n");
    git(f.peer.workdir().unwrap(), &["push"]);
    let local = commit(&f.local, "local change\n");
    assert!(pull_branch(&f.local, "main").is_err());
    assert!(push_branch(f.path(), "main").is_err());
    assert_eq!(f.remote.head().unwrap().target(), Some(remote));
    assert_eq!(f.local.head().unwrap().target(), Some(local));
}

#[test]
fn missing_upstream_and_changed_branch_are_reported() {
    let f = Fixture::new();
    git(f.local.workdir().unwrap(), &["switch", "-c", "topic"]);
    assert!(push_branch(f.path(), "main")
        .unwrap_err()
        .to_string()
        .contains("Current branch changed"));
    assert!(pull_branch(&f.local, "main").is_err());
    assert!(pull_branch(&f.local, "topic").is_err());
    f.local
        .config()
        .unwrap()
        .set_bool("push.autoSetupRemote", false)
        .unwrap();
    assert!(push_branch(f.path(), "topic").is_err());
}

#[test]
fn merge_conflict_is_left_visible_for_resolution() {
    let f = Fixture::new();
    commit(&f.peer, "remote change\n");
    git(f.peer.workdir().unwrap(), &["push"]);
    commit(&f.local, "local change\n");
    f.local
        .config()
        .unwrap()
        .set_str("pull.ff", "true")
        .unwrap();
    f.local
        .config()
        .unwrap()
        .set_bool("pull.rebase", false)
        .unwrap();
    assert!(pull_branch(&f.local, "main").is_err());
    assert_eq!(f.local.state(), git2::RepositoryState::Merge);
    assert!(f.local.index().unwrap().has_conflicts());
}

#[test]
fn pull_rebase_setting_replays_local_commit_onto_upstream() {
    let f = Fixture::new();
    fs::write(f.peer.workdir().unwrap().join("other"), "remote\n").unwrap();
    git(f.peer.workdir().unwrap(), &["add", "other"]);
    let remote = commit(&f.peer, "base\n");
    git(f.peer.workdir().unwrap(), &["push"]);
    let local = commit(&f.local, "local change\n");
    let mut config = f.local.config().unwrap();
    config.set_str("pull.ff", "true").unwrap();
    config.set_bool("pull.rebase", true).unwrap();
    pull_branch(&f.local, "main").unwrap();
    let head = f.local.head().unwrap().peel_to_commit().unwrap();
    assert_ne!(head.id(), local);
    assert_eq!(head.parent_id(0).unwrap(), remote);
    assert_eq!(
        fs::read_to_string(f.local.workdir().unwrap().join("file")).unwrap(),
        "local change\n"
    );
    assert_eq!(f.local.state(), git2::RepositoryState::Clean);
}
