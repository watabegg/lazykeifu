//! Workspace shortcuts, using keifu's cyan keys and yellow section headings.
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

pub(super) fn lines(width: usize) -> Vec<Line<'static>> {
    let sections: &[(&str, &[(&str, &str)])] = &[
        (
            "Navigation",
            &[
                (
                    "1 / 2 / 3 / 4 / 5",
                    "Status / Files / Branches / Graph / Stashes",
                ),
                ("6", "Commit detail; j / k: scroll, Enter: inspect diff"),
                ("Tab / Shift-Tab", "Next / previous pane (also h / l)"),
                ("j / k · ↑ / ↓", "Move selection; Ctrl-d / Ctrl-u: page"),
                ("Home / End · < / >", "First / last loaded row"),
                ("[ / ]", "Files or Branches: switch tabs"),
                ("Enter / 0", "Open diff; Esc: close diff"),
                ("/", "Search branches; Enter: select, Esc: cancel"),
                ("q", "Return to graph; q again: quit"),
            ],
        ),
        (
            "History & refs",
            &[
                (
                    "@",
                    "Extras: undo last commit, reflog, and additional operations",
                ),
                ("A / r", "Files: amend HEAD; Graph: reword HEAD"),
                ("R / T", "Local: rename branch; Local / Graph: create tag"),
                ("g", "Graph / refs: reset to selected commit/ref"),
                (
                    "Amend / Reword",
                    "Include staged changes / change message only",
                ),
                ("Undo", "Move HEAD to first parent; keep changes staged"),
                (
                    "Reset",
                    "Soft: keep index; mixed: unstage; hard: clean worktree only",
                ),
                ("Reflog", "Last 200 HEAD entries; Enter chooses reset mode"),
            ],
        ),
        (
            "Files & staging",
            &[
                ("Space", "Stage / unstage selected file"),
                ("Enter", "Folder: expand / collapse; Space: stage directory"),
                ("- / =", "Files: collapse / expand all directories"),
                ("a", "Toggle stage all / unstage all working files"),
                ("Tab in Diff", "Switch staged / unstaged diff"),
                (
                    "c",
                    "Commit dialog; if nothing staged, confirm stage all first",
                ),
                (
                    "d",
                    "Discard unstaged changes after confirmation; index kept",
                ),
                ("e", "Edit working file with the configured Git editor"),
                (
                    "XY",
                    "Left: index; right: worktree. MM means partially staged",
                ),
                (
                    "M A D R ? U",
                    "Modified / added / deleted / renamed / untracked / conflict",
                ),
            ],
        ),
        (
            "Diff & history",
            &[
                (
                    "h / l · Space",
                    "Diff: select hunk; stage / unstage selected hunk",
                ),
                ("H / L", "Scroll diff horizontally"),
                (
                    "Enter",
                    "Graph: inspect commit files (read-only); 2: working files",
                ),
                (
                    "C then V / t",
                    "Graph: copy then cherry-pick / revert selected commit",
                ),
                ("Space", "Graph: checkout selected commit (detached HEAD)"),
                ("Ctrl-r", "Graph: clear copied cherry-pick commit"),
                (
                    "Ctrl-l",
                    "Graph: log options, load 500 more commits, reflog",
                ),
                ("n / N in Diff", "Next / previous file"),
                (
                    "Hunks",
                    "Modified UTF-8 regular files; other changes use file stage",
                ),
            ],
        ),
        (
            "Branches & repositories",
            &[
                (
                    "Space / Enter",
                    "Local / Remote: checkout / inspect history",
                ),
                ("n / d", "Local: create / delete branch"),
                (
                    "M / r",
                    "Branches: merge into current / rebase onto selected",
                ),
                (
                    "Enter",
                    "Tags: inspect commit; Worktrees / Submodules: open repository",
                ),
                ("f / p / P", "Fetch / pull / push using Git configuration"),
                ("R", "Refresh repository"),
            ],
        ),
        (
            "Stashes",
            &[
                (
                    "s in Files",
                    "Save working changes including untracked files",
                ),
                ("Space / g / d", "Stashes: apply / pop / drop"),
                ("Enter", "Stashes: inspect diff"),
            ],
        ),
        (
            "Recovery & selection",
            &[
                ("Drag", "Select text; release to copy"),
                (
                    "m",
                    "Continue / abort current operation; Graph: merge / rebase menu",
                ),
                (
                    "Space",
                    "Conflicted file: mark current contents resolved after confirmation",
                ),
            ],
        ),
    ];
    let keys = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let heading = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc = Style::default().fg(Color::White);
    let mut lines = Vec::new();
    // Stack key and explanation on narrow terminals; never truncate a shortcut.
    let key_width = if width >= 55 { 21 } else { 0 };
    for (title, entries) in sections {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        for line in wrap(title, width.saturating_sub(1)) {
            lines.push(Line::styled(format!(" {line}"), heading));
        }
        for (key, text) in *entries {
            if key_width == 0 {
                for line in wrap(key, width.saturating_sub(1)) {
                    lines.push(Line::styled(format!(" {line}"), keys));
                }
            }
            let indent = if key_width == 0 { 3 } else { key_width };
            let wrapped = wrap(text, width.saturating_sub(indent));
            for (i, text) in wrapped.into_iter().enumerate() {
                let prefix = if i == 0 && key_width > 0 {
                    format!(" {key:<20}")
                } else {
                    " ".repeat(indent)
                };
                lines.push(Line::from(vec![
                    Span::styled(prefix, keys),
                    Span::styled(text, desc),
                ]));
            }
        }
    }
    lines
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.width() + 1 + word.width() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    lines.push(current);
    lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn help_fits_minimum_compact_and_wide_viewports() {
        for width in [16, 46, 106] {
            for line in super::lines(width) {
                assert!(line.width() <= width, "width {width}: {line}");
            }
        }
    }
}
