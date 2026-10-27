use crate::{app::App, workspace::Panel};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
pub fn line(app: &App) -> Line<'static> {
    let focus = app.workspace.focus;
    let keys: &[(&str, &str)] = if let Some(slot) = app.workspace.tabs.slot(focus) {
        match slot {
            0 | 1 => &[
                ("Enter", crate::tr!("open repo")),
                ("[ / ]", crate::tr!("tabs")),
                ("R", crate::tr!("refresh")),
            ],
            2 => &[
                ("Space", crate::tr!("checkout")),
                ("Enter", crate::tr!("history")),
                ("M/r", crate::tr!("merge/rebase")),
                ("T", crate::tr!("tag")),
            ],
            _ => &[
                ("Enter", crate::tr!("inspect tag")),
                ("[ / ]", crate::tr!("tabs")),
                ("R", crate::tr!("refresh")),
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
                        ("Enter", crate::tr!("fold")),
                        ("- / =", crate::tr!("fold/unfold all")),
                        ("2", crate::tr!("working files")),
                    ]
                } else {
                    &[
                        ("Enter", crate::tr!("fold")),
                        ("Space", crate::tr!("stage directory")),
                        ("a", crate::tr!("stage/unstage all")),
                        ("c", crate::tr!("commit")),
                        ("A", crate::tr!("amend")),
                    ]
                }
            }
            Panel::Files if app.workspace.commit.is_some() => &[
                ("Enter", crate::tr!("diff")),
                ("2", crate::tr!("working files")),
            ],
            Panel::Files => &[
                (
                    "Space",
                    if app.workspace.staged {
                        crate::tr!("unstage")
                    } else {
                        crate::tr!("stage")
                    },
                ),
                ("Enter", crate::tr!("diff")),
                ("c", crate::tr!("commit")),
                ("A", crate::tr!("amend")),
                ("d", crate::tr!("discard")),
                ("r", crate::tr!("refresh")),
            ],
            Panel::Branches => &[
                ("Space", crate::tr!("checkout")),
                ("Enter", crate::tr!("history")),
                ("n", crate::tr!("new")),
                ("R", crate::tr!("rename")),
                ("T", crate::tr!("tag")),
                ("d", crate::tr!("delete")),
                ("M/r", crate::tr!("merge/rebase")),
            ],
            Panel::Graph => &[
                ("Space", crate::tr!("branch")),
                ("o", crate::tr!("commit")),
                ("Enter", crate::tr!("diff")),
                ("r", crate::tr!("reword HEAD")),
                ("g", crate::tr!("reset")),
                ("T", crate::tr!("tag")),
                ("C/V", crate::tr!("copy/paste")),
                ("t", crate::tr!("revert")),
                ("Ctrl-l", crate::tr!("log")),
            ],
            Panel::Stashes => &[
                ("Space", crate::tr!("apply")),
                ("g", crate::tr!("pop")),
                ("d", crate::tr!("drop")),
                ("Enter", crate::tr!("diff")),
            ],
            Panel::CommitDetail => &[
                ("j/k", crate::tr!("scroll")),
                ("Enter", crate::tr!("diff")),
                ("4", crate::tr!("graph")),
            ],
            Panel::Diff
                if app.workspace.commit.is_none() && app.workspace.stash_preview.is_none() =>
            {
                &[
                    (
                        "Space",
                        if app.workspace.staged {
                            crate::tr!("unstage hunk")
                        } else {
                            crate::tr!("stage hunk")
                        },
                    ),
                    ("h/l", crate::tr!("hunk")),
                    ("Tab", crate::tr!("side")),
                    ("n/N", crate::tr!("file")),
                    ("Esc", crate::tr!("close")),
                ]
            }
            Panel::Diff => &[
                ("j/k", crate::tr!("scroll")),
                ("h/l", crate::tr!("hunk")),
                ("n/N", crate::tr!("file")),
                ("Esc", crate::tr!("close")),
            ],
            Panel::Status => &[
                ("f", crate::tr!("fetch")),
                ("p", crate::tr!("pull")),
                ("P", crate::tr!("push")),
                ("R", crate::tr!("refresh")),
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
