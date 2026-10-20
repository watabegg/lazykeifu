use crate::{app::App, workspace::Panel};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
pub fn line(app: &App) -> Line<'static> {
    let focus = app.workspace.focus;
    let keys: &[(&str, &str)] = if let Some(slot) = app.workspace.tabs.slot(focus) {
        match slot {
            0 | 1 => &[("Enter", "open repo"), ("[ / ]", "tabs"), ("R", "refresh")],
            2 => &[
                ("Space", "checkout"),
                ("Enter", "history"),
                ("M/r", "merge/rebase"),
                ("T", "tag"),
            ],
            _ => &[
                ("Enter", "inspect tag"),
                ("[ / ]", "tabs"),
                ("R", "refresh"),
            ],
        }
    } else {
        match focus {
            Panel::Files
                if app
                    .workspace
                    .tree
                    .selected()
                    .is_some_and(|n| n.file.is_none()) =>
            {
                if app.workspace.commit.is_some() {
                    &[
                        ("Enter", "fold"),
                        ("- / =", "fold/unfold all"),
                        ("2", "working files"),
                    ]
                } else {
                    &[
                        ("Enter", "fold"),
                        ("Space", "stage directory"),
                        ("a", "stage/unstage all"),
                        ("c", "commit"),
                        ("A", "amend"),
                    ]
                }
            }
            Panel::Files if app.workspace.commit.is_some() => {
                &[("Enter", "diff"), ("2", "working files")]
            }
            Panel::Files => &[
                (
                    "Space",
                    if app.workspace.staged {
                        "unstage"
                    } else {
                        "stage"
                    },
                ),
                ("Enter", "diff"),
                ("c", "commit"),
                ("A", "amend"),
                ("d", "discard"),
                ("r", "refresh"),
            ],
            Panel::Branches => &[
                ("Space", "checkout"),
                ("Enter", "history"),
                ("n", "new"),
                ("R", "rename"),
                ("T", "tag"),
                ("d", "delete"),
                ("M/r", "merge/rebase"),
            ],
            Panel::Graph => &[
                ("Enter", "diff"),
                ("r", "reword HEAD"),
                ("g", "reset"),
                ("T", "tag"),
                ("C/V", "copy/paste"),
                ("t", "revert"),
                ("Ctrl-l", "log"),
            ],
            Panel::Stashes => &[
                ("Space", "apply"),
                ("g", "pop"),
                ("d", "drop"),
                ("Enter", "diff"),
            ],
            Panel::CommitDetail => &[("j/k", "scroll"), ("Enter", "diff"), ("4", "graph")],
            Panel::Diff
                if app.workspace.commit.is_none() && app.workspace.stash_preview.is_none() =>
            {
                &[
                    (
                        "Space",
                        if app.workspace.staged {
                            "unstage hunk"
                        } else {
                            "stage hunk"
                        },
                    ),
                    ("h/l", "hunk"),
                    ("Tab", "side"),
                    ("n/N", "file"),
                    ("Esc", "close"),
                ]
            }
            Panel::Diff => &[
                ("j/k", "scroll"),
                ("h/l", "hunk"),
                ("n/N", "file"),
                ("Esc", "close"),
            ],
            Panel::Status => &[
                ("f", "fetch"),
                ("p", "pull"),
                ("P", "push"),
                ("R", "refresh"),
            ],
        }
    };
    let mut spans = Vec::new();
    for (i, (key, label)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(" {label}")));
    }
    Line::from(spans)
}
