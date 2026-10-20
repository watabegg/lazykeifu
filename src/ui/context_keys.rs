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
                ("Enter", "checkout"),
                ("M", "merge"),
                ("r", "rebase"),
                ("[ / ]", "tabs"),
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
                        ("Enter / Space", "fold"),
                        ("← / →", "tree"),
                        ("2", "working files"),
                    ]
                } else {
                    &[
                        ("Enter / Space", "fold"),
                        ("← / →", "tree"),
                        ("c", "commit"),
                        ("a / u", "stage all / none"),
                    ]
                }
            }
            Panel::Files if app.workspace.commit.is_some() => {
                &[("Enter", "diff"), ("← / →", "tree"), ("2", "working files")]
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
                ("d", "discard"),
                ("e", "edit"),
                ("← / →", "tree"),
                ("[ / ]", "tabs"),
            ],
            Panel::Branches => &[
                ("Enter", "checkout"),
                ("b", "new"),
                ("d", "delete"),
                ("M", "merge"),
                ("r", "rebase"),
                ("[ / ]", "tabs"),
            ],
            Panel::Graph => &[
                ("Enter", "diff"),
                ("y", "pick"),
                ("V", "revert"),
                ("M", "merge"),
                ("r", "rebase"),
                (
                    "+",
                    if app.history_has_more {
                        "load more"
                    } else {
                        "all loaded"
                    },
                ),
            ],
            Panel::Stashes => &[
                ("Space", "apply"),
                ("p", "pop"),
                ("d", "drop"),
                ("Enter", "diff"),
                ("s", "save"),
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
                    ("[ / ]", "hunk"),
                    ("v", "side"),
                    ("H/L", "pan"),
                    ("Esc", "close"),
                ]
            }
            Panel::Diff => &[("j/k", "scroll"), ("H/L", "pan"), ("Esc", "close")],
            Panel::Status => &[
                ("f", "fetch"),
                ("p", "pull"),
                ("P", "push"),
                ("R", "refresh"),
            ],
        }
    };
    let mut spans = vec![
        Span::styled(
            "x",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Git actions · "),
    ];
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
