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
    repo.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
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

#[test]
fn file_status_columns_match_git_porcelain_and_preserve_change_kind_colors() {
    use lazykeifu::{app::App, git::GitRepository};
    use ratatui::{backend::TestBackend, style::Color, Terminal};
    let (temp, repo) = repo();
    for name in ["modified", "partial", "deleted", "renamed"] {
        fs::write(temp.path().join(name), "base\n").unwrap();
    }
    git(temp.path(), &["add", "."]);
    commit(&repo);
    fs::write(temp.path().join("modified"), "edit\n").unwrap();
    fs::write(temp.path().join("partial"), "staged\n").unwrap();
    git(temp.path(), &["add", "partial"]);
    fs::write(temp.path().join("partial"), "worktree\n").unwrap();
    fs::remove_file(temp.path().join("deleted")).unwrap();
    git(temp.path(), &["mv", "renamed", "new-name"]);
    fs::write(temp.path().join("added"), "added\n").unwrap();
    git(temp.path(), &["add", "added"]);
    fs::write(temp.path().join("unknown"), "unknown\n").unwrap();
    let rows = working_files(&repo).unwrap();
    for (name, expected) in [
        ("added", "A "),
        ("deleted", " D"),
        ("modified", " M"),
        ("new-name", "R "),
        ("partial", "MM"),
        ("unknown", "??"),
    ] {
        let row = rows.iter().find(|row| row.path == Path::new(name)).unwrap();
        assert_eq!(
            format!("{}{}", row.index_status, row.worktree_status),
            expected
        );
    }
    let mut app = App::from_repository(GitRepository::open(temp.path()).unwrap()).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
    terminal
        .draw(|frame| lazykeifu::ui::draw(frame, &mut app))
        .unwrap();
    let x = app.layout.files.x + 3; // border + selection gutter
    let buffer = terminal.backend().buffer();
    for (name, column, symbol, color) in [
        ("added", 0, "A", Color::Green),
        ("deleted", 1, "D", Color::Red),
        ("modified", 1, "M", Color::Yellow),
        ("new-name", 0, "R", Color::Cyan),
        ("partial", 0, "M", Color::Yellow),
        ("partial", 1, "M", Color::Yellow),
        ("unknown", 0, "?", Color::Magenta),
    ] {
        let y = app.layout.files.y
            + 1
            + rows.iter().position(|r| r.path == Path::new(name)).unwrap() as u16;
        assert_eq!(buffer[(x + column, y)].symbol(), symbol);
        assert_eq!(buffer[(x + column, y)].fg, color);
    }
    app.workspace.help = true;
    terminal
        .draw(|frame| lazykeifu::ui::draw(frame, &mut app))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert!(buffer
        .content
        .iter()
        .any(|c| c.symbol() == "N" && c.fg == Color::Yellow));
    assert!(buffer
        .content
        .iter()
        .any(|c| c.symbol() == "T" && c.fg == Color::Cyan));
}

#[test]
fn conflict_columns_match_git_including_delete_modify_and_add_add() {
    for case in ["modify", "delete-us", "delete-them", "add-add"] {
        let (temp, repo) = repo();
        fs::write(temp.path().join("base"), "base\n").unwrap();
        if case != "add-add" {
            fs::write(temp.path().join("file"), "base\n").unwrap();
        }
        git(temp.path(), &["add", "."]);
        commit(&repo);
        git(temp.path(), &["switch", "-c", "topic"]);
        if case == "delete-them" {
            fs::remove_file(temp.path().join("file")).unwrap();
        } else {
            fs::write(temp.path().join("file"), "topic\n").unwrap();
        }
        git(temp.path(), &["add", "-A"]);
        commit(&repo);
        git(temp.path(), &["switch", "main"]);
        if case == "delete-us" {
            fs::remove_file(temp.path().join("file")).unwrap();
        } else {
            fs::write(temp.path().join("file"), "main\n").unwrap();
        }
        git(temp.path(), &["add", "-A"]);
        commit(&repo);
        assert!(!Command::new("git")
            .args(["merge", "topic"])
            .current_dir(temp.path())
            .output()
            .unwrap()
            .status
            .success());
        let rows = working_files(&repo).unwrap();
        let row = rows.iter().find(|r| r.path == Path::new("file")).unwrap();
        let actual = Command::new("git")
            .args(["status", "--porcelain", "--", "file"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert_eq!(
            format!("{}{}", row.index_status, row.worktree_status),
            String::from_utf8(actual.stdout).unwrap()[..2],
            "{case}"
        );
    }
}
