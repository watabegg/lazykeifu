//! Help popup widget

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
};

pub struct HelpPopup {
    scroll: u16,
}

impl HelpPopup {
    pub fn new(scroll: u16) -> Self {
        Self { scroll }
    }

    pub fn line_count(width: u16) -> usize {
        let inner_width = width.saturating_sub(2).max(1);
        Paragraph::new(help_lines())
            .wrap(Wrap { trim: false })
            .line_count(inner_width)
    }
}

fn help_lines() -> Vec<Line<'static>> {
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    vec![
        Line::from(Span::styled(crate::tr!("Navigation"), header_style)),
        Line::from(vec![
            Span::styled("  j / ↓      ", key_style),
            Span::styled(crate::tr!("Move down"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑      ", key_style),
            Span::styled(crate::tr!("Move up"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Tab        ", key_style),
            Span::styled(crate::tr!("Switch pane focus (graph / detail)"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  ]          ", key_style),
            Span::styled(crate::tr!("Select next branch"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  [          ", key_style),
            Span::styled(crate::tr!("Select previous branch"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  h / ←      ", key_style),
            Span::styled(crate::tr!("Select left branch (same commit)"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  l / →      ", key_style),
            Span::styled(crate::tr!("Select right branch (same commit)"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d     ", key_style),
            Span::styled(crate::tr!("Page down"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u     ", key_style),
            Span::styled(crate::tr!("Page up"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  g / Home   ", key_style),
            Span::styled(crate::tr!("Go to top"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  G / End    ", key_style),
            Span::styled(crate::tr!("Go to bottom"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  @          ", key_style),
            Span::styled(crate::tr!("Jump to HEAD (current branch)"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  o          ", key_style),
            Span::styled(crate::tr!("Toggle remote branches"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  t          ", key_style),
            Span::styled(crate::tr!("Toggle tags"), desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(crate::tr!("Git Operations"), header_style)),
        Line::from(vec![
            Span::styled("  Enter      ", key_style),
            Span::styled(crate::tr!("Checkout selected branch/commit"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  b          ", key_style),
            Span::styled(crate::tr!("Create new branch"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  d          ", key_style),
            Span::styled(crate::tr!("Delete branch"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  f          ", key_style),
            Span::styled(
                crate::trf!("Fetch from {remote}", remote = "origin"),
                desc_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("  c          ", key_style),
            Span::styled(crate::tr!("Commit staged changes"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  p          ", key_style),
            Span::styled(
                crate::trf!("Push current branch to {remote}", remote = "origin"),
                desc_style,
            ),
        ]),
        // TODO: merge and rebase will be implemented in the future
        // Line::from(vec![
        //     Span::styled("  m          ", key_style),
        //     Span::styled("Merge branch", desc_style),
        // ]),
        // Line::from(vec![
        //     Span::styled("  r          ", key_style),
        //     Span::styled("Rebase onto branch", desc_style),
        // ]),
        Line::from(""),
        Line::from(Span::styled(crate::tr!("Search"), header_style)),
        Line::from(vec![
            Span::styled("  /          ", key_style),
            Span::styled(crate::tr!("Search branches"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / C-k    ", key_style),
            Span::styled(crate::tr!("Select previous result"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  ↓ / C-j    ", key_style),
            Span::styled(crate::tr!("Select next result"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Enter      ", key_style),
            Span::styled(crate::tr!("Jump to selected branch"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Esc        ", key_style),
            Span::styled(crate::tr!("Cancel search"), desc_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(crate::tr!("Diff / Staging"), header_style)),
        Line::from(vec![
            Span::styled("  Space      ", key_style),
            Span::styled(crate::tr!("View file diffs"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  s          ", key_style),
            Span::styled(
                crate::tr!("Stage/unstage selected file (file list)"),
                desc_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("  a / u      ", key_style),
            Span::styled(
                crate::tr!("Stage all / unstage all (file list)"),
                desc_style,
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(crate::tr!("Other"), header_style)),
        Line::from(vec![
            Span::styled("  Drag       ", key_style),
            Span::styled(crate::tr!("Select text; release to copy"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  y          ", key_style),
            Span::styled(crate::tr!("Copy commit hash to clipboard"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  Y          ", key_style),
            Span::styled(crate::tr!("Copy branch name to clipboard"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  R          ", key_style),
            Span::styled(crate::tr!("Refresh"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", key_style),
            Span::styled(crate::tr!("Toggle this help"), desc_style),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc    ", key_style),
            Span::styled(crate::tr!("Quit"), desc_style),
        ]),
    ]
}

impl Widget for HelpPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the background
        Clear.render(area, buf);

        let lines = help_lines();
        let block = Block::default()
            .title(crate::tr!(" Help "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));

        Widget::render(paragraph, area, buf);
    }
}
