//! Patch presentation, separate from the exact Git patch used for staging.
use crate::git::{
    DiffHunkContent, DiffLineContent, DiffLineOrigin, FileChangeKind, FileDiffContent,
};
use ratatui::{
    layout::{Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::path::{Path, PathBuf};
#[derive(Default)]
pub struct PatchView {
    pub lines: Vec<Line<'static>>,
    pub hunks: Vec<usize>,
    pub additions: usize,
    pub deletions: usize,
}
impl PatchView {
    pub fn build(text: &str, path: Option<&Path>) -> Self {
        let mut result = Self::default();
        let mut file = FileDiffContent {
            path: path.unwrap_or(Path::new("patch.txt")).into(),
            kind: FileChangeKind::Modified,
            is_binary: false,
            hunks: vec![],
            total_additions: 0,
            total_deletions: 0,
        };
        let mut old = 0;
        let mut new = 0;
        let mut meta = Vec::new();
        for line in text.lines() {
            if line.starts_with("diff --git ") {
                Self::flush(&mut result, &mut file, &mut meta, path.is_none());
                if path.is_none() {
                    file.path = PathBuf::from(line.split_once(" b/").map_or(line, |(_, s)| s));
                }
                continue;
            }
            if line.starts_with("@@ ") {
                let parts: Vec<_> = line.split_whitespace().collect();
                old = parts
                    .get(1)
                    .and_then(|s| s.trim_start_matches('-').split(',').next()?.parse().ok())
                    .unwrap_or(0);
                new = parts
                    .get(2)
                    .and_then(|s| s.trim_start_matches('+').split(',').next()?.parse().ok())
                    .unwrap_or(0);
                file.hunks.push(DiffHunkContent {
                    header: line.to_string(),
                    lines: vec![],
                });
            } else if let Some(hunk) = file.hunks.last_mut() {
                let (origin, old_lineno, new_lineno) = match line.as_bytes().first() {
                    Some(b'+') => {
                        result.additions += 1;
                        new += 1;
                        (DiffLineOrigin::Addition, None, Some(new - 1))
                    }
                    Some(b'-') => {
                        result.deletions += 1;
                        old += 1;
                        (DiffLineOrigin::Deletion, Some(old - 1), None)
                    }
                    Some(b' ') => {
                        old += 1;
                        new += 1;
                        (DiffLineOrigin::Context, Some(old - 1), Some(new - 1))
                    }
                    Some(b'\\') => (DiffLineOrigin::NoNewlineAtEof, None, None),
                    _ => continue,
                };
                hunk.lines.push(DiffLineContent {
                    origin,
                    old_lineno,
                    new_lineno,
                    content: format!("{}\n", line.get(1..).unwrap_or("")),
                });
            } else if !line.starts_with("index ")
                && !line.starts_with("--- ")
                && !line.starts_with("+++ ")
            {
                meta.push(line.to_string());
            }
        }
        Self::flush(&mut result, &mut file, &mut meta, path.is_none());
        result
    }
    fn flush(
        result: &mut Self,
        file: &mut FileDiffContent,
        meta: &mut Vec<String>,
        show_path: bool,
    ) {
        if file.hunks.is_empty() && meta.is_empty() {
            return;
        }
        if show_path && !file.hunks.is_empty() {
            result.lines.push(Line::styled(
                file.path.display().to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        result.lines.extend(
            meta.drain(..)
                .map(|s| Line::styled(s, Style::default().fg(Color::Gray))),
        );
        if !file.hunks.is_empty() {
            let (mut lines, positions) = super::file_diff_view::build_highlighted_lines(file);
            // Drop the renderer's leading spacer; keep separation between hunks.
            lines.remove(0);
            let base = result.lines.len();
            result
                .hunks
                .extend(positions.into_iter().map(|p| base + p.saturating_sub(1)));
            result.lines.extend(lines);
            file.hunks.clear();
        }
    }
}
pub fn totals(files: Option<usize>, add: usize, del: usize) -> Line<'static> {
    let mut spans = vec![];
    if let Some(files) = files {
        spans.push(Span::raw(format!("{files} files  ")));
    }
    spans.push(Span::styled(
        format!("+{add}"),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        format!("-{del}"),
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    ));
    Line::from(spans)
}
#[derive(Default)]
pub struct Viewport {
    pub scroll: u16,
    pub horizontal: u16,
    pub selected: Option<usize>,
    pub focused: bool,
}
pub fn draw(
    frame: &mut Frame,
    rect: Rect,
    title: &str,
    path: Option<&Path>,
    view: &PatchView,
    viewport: Viewport,
) {
    let Viewport {
        scroll,
        horizontal,
        selected,
        focused,
    } = viewport;
    frame.render_widget(super::pane_block(title, focused), rect);
    let inner = rect.inner(Margin::new(1, 1));
    if inner.height == 0 {
        return;
    }
    let mut header = totals(None, view.additions, view.deletions);
    if let Some(path) = path {
        header.spans.push(Span::styled(
            format!("  {}", path.display()),
            Style::default().fg(Color::Cyan),
        ));
    }
    frame.render_widget(
        Paragraph::new(header),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let lines = view
        .lines
        .iter()
        .enumerate()
        .skip(scroll as usize)
        .take(inner.height.saturating_sub(1) as usize)
        .map(|(i, line)| {
            let mut line = line.clone();
            if selected.is_some_and(|h| view.hunks.get(h) == Some(&i)) {
                line.style = Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
                line.spans
                    .insert(0, Span::styled("› ", Style::default().fg(Color::Yellow)));
            }
            line
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).scroll((0, horizontal)),
        Rect::new(
            inner.x,
            inner.y + 1,
            inner.width,
            inner.height.saturating_sub(1),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patch_counts_numbers_and_hunk_positions_ignore_metadata() {
        let text="diff --git a/code.rs b/code.rs\nindex abc..def 100644\n--- a/code.rs\n+++ b/code.rs\n@@ -2,2 +2,2 @@ function\n-old text\n+new text\n context\n@@ -20 +20 @@\n-last\n+next\n\\ No newline at end of file\n";
        let view = PatchView::build(text, Some(Path::new("code.rs")));
        assert_eq!((view.additions, view.deletions), (2, 2));
        assert_eq!(view.hunks.len(), 2);
        for &row in &view.hunks {
            assert!(view.lines[row].to_string().starts_with("@@"));
        }
        let lines: Vec<_> = view.lines.iter().map(ToString::to_string).collect();
        assert!(lines
            .iter()
            .any(|l| l.contains("   2") && l.contains("-old text")));
        assert!(lines
            .iter()
            .any(|l| l.contains("  20") && l.contains("+next")));
        assert!(!lines
            .iter()
            .any(|l| l.starts_with("diff --git") || l.starts_with("index ")));
        assert!(view
            .lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.style.bg.is_some())));
    }
    #[test]
    fn multiple_files_and_binary_metadata_remain_visible() {
        let text="diff --git a/a b/a\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/b b/b\nBinary files a/b and b/b differ\ndiff --git a/c b/c\n@@ -0,0 +1 @@\n+third\n";
        let view = PatchView::build(text, None);
        assert_eq!((view.additions, view.deletions), (2, 1));
        assert_eq!(view.hunks.len(), 2);
        assert!(view
            .lines
            .iter()
            .any(|l| l.to_string().contains("Binary files")));
        assert!(view.lines.iter().any(|l| l.to_string() == "c"));
    }
}
