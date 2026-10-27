//! Workspace shortcuts, using keifu's cyan keys and yellow section headings.
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

pub(super) fn lines(width: usize) -> Vec<Line<'static>> {
    let stash_key = format!("s ({})", crate::tr!("Files"));
    let sections: &[(&str, &[(&str, &str)])] = &[
        (
            crate::tr!("Navigation"),
            &[
                (
                    "1 / 2 / 3 / 4 / 5",
                    crate::tr!("Status / Files / Branches / Graph / Stashes"),
                ),
                (
                    "6",
                    crate::tr!("Commit detail; j / k: scroll, Enter: inspect diff"),
                ),
                (
                    "Tab / Shift-Tab",
                    crate::tr!("Next / previous pane (also h / l)"),
                ),
                (
                    "j / k · ↑ / ↓",
                    crate::tr!("Move selection; Ctrl-d / Ctrl-u: page"),
                ),
                ("Home / End · < / >", crate::tr!("First / last loaded row")),
                ("[ / ]", crate::tr!("Files or Branches: switch tabs")),
                ("Enter / 0", crate::tr!("Open diff; Esc: close diff")),
                (
                    "/",
                    crate::tr!("Search branches; Enter: select, Esc: cancel"),
                ),
                ("q", crate::tr!("Return to graph; q again: quit")),
            ],
        ),
        (
            crate::tr!("History & refs"),
            &[
                (
                    "@",
                    crate::tr!("Extras: undo last commit, reflog, and additional operations"),
                ),
                ("A / r", crate::tr!("Files: amend HEAD; Graph: reword HEAD")),
                (
                    "R / T",
                    crate::tr!("Local: rename branch; Local / Graph: create tag"),
                ),
                (
                    "g",
                    crate::tr!("Graph / refs: reset to selected commit/ref"),
                ),
                (
                    crate::tr!("Amend / Reword"),
                    crate::tr!("Include staged changes / change message only"),
                ),
                (
                    crate::tr!("Undo"),
                    crate::tr!("Move HEAD to first parent; keep changes staged"),
                ),
                (
                    crate::tr!("Reset"),
                    crate::tr!("Soft: keep index; mixed: unstage; hard: clean worktree only"),
                ),
                (
                    crate::tr!("Reflog"),
                    crate::tr!("Last 200 HEAD entries; Enter chooses reset mode"),
                ),
            ],
        ),
        (
            crate::tr!("Files & staging"),
            &[
                ("Space", crate::tr!("Stage / unstage selected file")),
                (
                    "Enter",
                    crate::tr!("Folder: expand / collapse; Space: stage directory"),
                ),
                (
                    "- / =",
                    crate::tr!("Files: collapse / expand all directories"),
                ),
                (
                    "a",
                    crate::tr!("Toggle stage all / unstage all working files"),
                ),
                (
                    crate::tr!("Tab in Diff"),
                    crate::tr!("Switch staged / unstaged diff"),
                ),
                (
                    "c",
                    crate::tr!("Commit dialog; if nothing staged, confirm stage all first"),
                ),
                (
                    "d",
                    crate::tr!("Discard unstaged changes after confirmation; index kept"),
                ),
                (
                    "e",
                    crate::tr!("Edit working file with the configured Git editor"),
                ),
                (
                    "XY",
                    crate::tr!("Left: index; right: worktree. MM means partially staged"),
                ),
                (
                    "M A D R ? U",
                    crate::tr!("Modified / added / deleted / renamed / untracked / conflict"),
                ),
            ],
        ),
        (
            crate::tr!("Diff & history"),
            &[
                (
                    "h / l · Space",
                    crate::tr!("Diff: select hunk; stage / unstage selected hunk"),
                ),
                ("H / L", crate::tr!("Scroll diff horizontally")),
                (
                    "Enter",
                    crate::tr!("Graph: inspect commit files (read-only); 2: working files"),
                ),
                (
                    "C → V / t",
                    crate::tr!("Graph: copy then cherry-pick / revert selected commit"),
                ),
                (
                    "Space",
                    crate::tr!("Graph: checkout branch at selected commit"),
                ),
                (
                    "Alt+Space",
                    crate::tr!("Graph: checkout selected commit (detached HEAD)"),
                ),
                (
                    "Ctrl-r",
                    crate::tr!("Graph: clear copied cherry-pick commit"),
                ),
                (
                    "Ctrl-l",
                    crate::tr!("Graph: log options, load 500 more commits, reflog"),
                ),
                (
                    crate::tr!("n / N in Diff"),
                    crate::tr!("Next / previous file"),
                ),
                (
                    crate::tr!("Hunks"),
                    crate::tr!("Modified UTF-8 regular files; other changes use file stage"),
                ),
            ],
        ),
        (
            crate::tr!("Branches & repositories"),
            &[
                (
                    "Space / Enter",
                    crate::tr!("Local / Remote: checkout / inspect history"),
                ),
                ("n / d", crate::tr!("Local: create / delete branch")),
                (
                    "M / r",
                    crate::tr!("Branches: merge into current / rebase onto selected"),
                ),
                (
                    "Enter",
                    crate::tr!("Tags: inspect commit; Worktrees / Submodules: open repository"),
                ),
                ("f / p / P", crate::tr!("Fetch / pull / push")),
                ("R", crate::tr!("Refresh repository")),
            ],
        ),
        (
            crate::tr!("Stashes"),
            &[
                (
                    stash_key.as_str(),
                    crate::tr!("Save working changes including untracked files"),
                ),
                ("Space / g / d", crate::tr!("Stashes: apply / pop / drop")),
                ("Enter", crate::tr!("Stashes: inspect diff")),
            ],
        ),
        (
            crate::tr!("Recovery & selection"),
            &[
                (
                    crate::tr!("Drag"),
                    crate::tr!("Select text; release to copy"),
                ),
                (
                    "m",
                    crate::tr!("Continue / abort current operation; Graph: merge / rebase menu"),
                ),
                (
                    "Space",
                    crate::tr!(
                        "Conflicted file: mark current contents resolved after confirmation"
                    ),
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
    let key_width: usize = if width >= 55 { 21 } else { 0 };
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
                for text in wrap(text, width.saturating_sub(3)) {
                    lines.push(Line::from(vec![Span::raw("   "), Span::styled(text, desc)]));
                }
            } else {
                let key_lines = wrap(key, key_width.saturating_sub(1));
                let desc_lines = wrap(text, width.saturating_sub(key_width));
                for i in 0..key_lines.len().max(desc_lines.len()) {
                    let prefix = if let Some(key_line) = key_lines.get(i) {
                        format!(" {}", pad_to_width(key_line, key_width - 1))
                    } else {
                        " ".repeat(key_width)
                    };
                    let text = desc_lines.get(i).map_or_else(String::new, Clone::clone);
                    lines.push(Line::from(vec![
                        Span::styled(prefix, keys),
                        Span::styled(text, desc),
                    ]));
                }
            }
        }
    }
    lines
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let width = width.max(1);
    for word in text.split_whitespace() {
        if !current.is_empty() && current.width() + 1 + word.width() <= width {
            current.push(' ');
            current.push_str(word);
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }

        if word.width() <= width {
            current.push_str(word);
            continue;
        }

        for ch in word.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if !current.is_empty() && current.width() + ch_width > width {
                lines.push(std::mem::take(&mut current));
            }
            current.push(ch);
        }
        if current.width() >= width {
            lines.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn pad_to_width(text: &str, width: usize) -> String {
    format!("{text}{}", " ".repeat(width.saturating_sub(text.width())))
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
