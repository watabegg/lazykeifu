//! A persistent commit draft with staged-file review in the same workspace.
use crate::{
    action::Action,
    app::{App, AppMode, InputAction},
    workspace::Panel,
};
use anyhow::{bail, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use git2::Oid;
use ratatui::layout::Rect;

#[derive(Default)]
pub struct Editor {
    pub text: String,
    pub cursor: usize,
    pub top: usize,
    pub left: usize,
}
impl Editor {
    pub fn insert(&mut self, text: &str) {
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }
    fn previous(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }
    fn next(&self) -> usize {
        self.text[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |c| self.cursor + c.len_utf8())
    }
    pub fn row_col(&self) -> (usize, usize) {
        let before = &self.text[..self.cursor];
        (
            before.bytes().filter(|b| *b == b'\n').count(),
            before.rsplit('\n').next().unwrap_or("").chars().count(),
        )
    }
    pub fn place(&mut self, row: usize, column: usize) {
        let lines: Vec<_> = self.text.split('\n').collect();
        let row = row.min(lines.len() - 1);
        let start: usize = lines[..row].iter().map(|line| line.len() + 1).sum();
        self.cursor = start
            + lines[row]
                .char_indices()
                .nth(column)
                .map_or(lines[row].len(), |(i, _)| i);
    }
    fn key(&mut self, key: KeyEvent, multiline: bool) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('a')) | (_, KeyCode::Home) => {
                self.cursor = self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('e')) | (_, KeyCode::End) => {
                self.cursor += self.text[self.cursor..]
                    .find('\n')
                    .unwrap_or(self.text.len() - self.cursor)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                let start = self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1);
                self.text.drain(start..self.cursor);
                self.cursor = start;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                let end = self.cursor;
                while self.cursor > 0
                    && self.text[..self.cursor]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_whitespace)
                {
                    self.cursor = self.previous();
                }
                while self.cursor > 0
                    && self.text[..self.cursor]
                        .chars()
                        .next_back()
                        .is_some_and(|c| !c.is_whitespace())
                {
                    self.cursor = self.previous();
                }
                self.text.drain(self.cursor..end);
            }
            (_, KeyCode::Left) => self.cursor = self.previous(),
            (_, KeyCode::Right) => self.cursor = self.next(),
            (_, KeyCode::Up | KeyCode::Down) => {
                let (r, c) = self.row_col();
                self.place(
                    if key.code == KeyCode::Up {
                        r.saturating_sub(1)
                    } else {
                        r + 1
                    },
                    c,
                );
            }
            (_, KeyCode::Backspace) => {
                let start = self.previous();
                self.text.drain(start..self.cursor);
                self.cursor = start;
            }
            (_, KeyCode::Delete) => {
                let end = self.next();
                self.text.drain(self.cursor..end);
            }
            (_, KeyCode::Enter) if multiline => self.insert("\n"),
            (mods, KeyCode::Char(c))
                if !mods.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                ) =>
            {
                self.insert(&c.to_string())
            }
            _ => {}
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Field {
    #[default]
    Subject,
    Body,
    Review,
}
#[derive(Default)]
pub struct Composer {
    pub amend: Option<(crate::git::history::Snapshot, bool)>,
    pub subject: Editor,
    pub body: Editor,
    pub field: Field,
    pub staged: Vec<usize>,
    pub selected: usize,
    pub list_top: usize,
    pub rect: Rect,
    pub diff_rect: Rect,
    pub fields: [Rect; 3],
    pub error_rect: Rect,
    pub error_scroll: u16,
    tree: Option<Oid>,
    head: Option<Oid>,
}
impl Composer {
    pub fn message(&self) -> String {
        let subject = self.subject.text.trim();
        let body = self.body.text.trim_matches('\n');
        if body.trim().is_empty() {
            subject.into()
        } else {
            format!("{subject}\n\n{body}")
        }
    }
}
impl App {
    pub fn composing(&self) -> bool {
        matches!(
            self.mode,
            AppMode::Input {
                action: InputAction::CommitMessage,
                ..
            }
        )
    }
    fn composer_mode(&mut self) {
        self.mode = AppMode::Input {
            title: crate::tr!("Commit").into(),
            input: self.workspace.composer.message(),
            action: InputAction::CommitMessage,
        };
    }
    fn staged_snapshot(&self) -> Result<(Oid, Option<Oid>)> {
        let mut index = self.repo.repo.index()?;
        index.read(true)?;
        if index.has_conflicts() {
            bail!("{}", crate::tr!("Resolve conflicts before committing"));
        }
        Ok((
            index.write_tree()?,
            self.repo.repo.head().ok().and_then(|h| h.target()),
        ))
    }
    pub(crate) fn composer_refresh(&mut self) -> Result<()> {
        self.workspace.commit = None;
        self.workspace.stash_preview = None;
        self.workspace.invalidate();
        self.workspace.load_files(&self.repo.repo)?;
        let (tree, head) = self.staged_snapshot()?;
        let c = &mut self.workspace.composer;
        c.tree = Some(tree);
        c.head = head;
        c.staged = self
            .workspace
            .files
            .iter()
            .enumerate()
            .filter(|(_, r)| r.staged && !r.conflict)
            .map(|(i, _)| i)
            .collect();
        c.selected = c.selected.min(c.staged.len().saturating_sub(1));
        if c.amend
            .as_ref()
            .is_some_and(|(_, message_only)| *message_only)
        {
            c.staged.clear();
        }
        self.composer_select();
        Ok(())
    }
    pub(crate) fn composer_select(&mut self) {
        self.workspace.file_state.select(
            self.workspace
                .composer
                .staged
                .get(self.workspace.composer.selected)
                .copied(),
        );
        self.workspace.staged = true;
        self.workspace.scroll = 0;
        self.workspace.invalidate();
    }
    pub(crate) fn open_composer(&mut self) -> Result<()> {
        self.composer_refresh()?;
        if self.workspace.composer.staged.is_empty() && self.workspace.composer.amend.is_none() {
            if self.workspace.files.is_empty() {
                self.set_message(crate::tr!("No changes to commit"));
            } else {
                self.mode = AppMode::Confirm {
                    message: crate::tr!("Nothing staged. Stage all changes (including untracked files) and open commit?").into(),
                    action: crate::app::ConfirmAction::StageAllCommit,
                };
            }
            return Ok(());
        }
        self.workspace.tabs.files = crate::workspace_tabs::FilesTab::Files;

        self.workspace.focus = Panel::Files;
        self.workspace.composer.field = Field::Subject;
        self.workspace.commit_error = None;
        self.composer_mode();
        Ok(())
    }
    fn submit_composer(&mut self) -> Result<()> {
        self.workspace.composer.error_scroll = 0;
        if self.workspace.composer.subject.text.trim().is_empty() {
            self.workspace.commit_error = Some(crate::tr!("Write a commit subject first").into());
            self.workspace.composer.field = Field::Subject;
            return Ok(());
        }
        let snapshot = self.staged_snapshot()?;
        if Some(snapshot.0) != self.workspace.composer.tree
            || snapshot.1 != self.workspace.composer.head
        {
            self.composer_refresh()?;
            self.workspace.composer.field = Field::Review;
            self.workspace.commit_error = Some(
                crate::tr!(
                    "Staged changes or HEAD changed. Review the updated selection, then commit again."
                )
                .into(),
            );
            return Ok(());
        }
        if self.workspace.composer.staged.is_empty() && self.workspace.composer.amend.is_none() {
            bail!(
                "{}",
                crate::tr!("No staged changes; return to Files to stage a change")
            );
        }
        if let Some((snapshot, _)) = &mut self.workspace.composer.amend {
            let current = crate::git::history::Snapshot::capture(&self.repo.repo)?;
            if current.head != snapshot.head || current.reference != snapshot.reference {
                bail!(
                "{}",
                crate::tr!("HEAD or branch changed; close this draft and reopen amend from Git actions")
            );
            }
            snapshot.tree = current.tree;
        }
        self.workspace_commit(self.workspace.composer.message());
        Ok(())
    }
    pub(crate) fn composer_action(&mut self, action: Action) -> Result<()> {
        let result = self.composer_edit(action);
        if let Err(error) = result {
            self.workspace.composer.error_scroll = 0;
            self.workspace.commit_error = Some(error.to_string());
        }
        if self.composing() {
            self.composer_mode();
        }
        Ok(())
    }
    fn composer_edit(&mut self, action: Action) -> Result<()> {
        let key = match action {
            Action::CommitInput(key) => key,
            Action::Confirm => return self.submit_composer(),
            Action::Cancel => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            Action::InputChar(c) => KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            Action::InputBackspace => KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
            Action::Paste(text) => {
                let text = text
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
                    .replace('\t', "    ");
                let text: String = text
                    .chars()
                    .filter(|c| *c == '\n' || !c.is_control())
                    .collect();
                let c = &mut self.workspace.composer;
                match c.field {
                    Field::Subject => {
                        let (subject, body) = text.split_once('\n').unwrap_or((&text, ""));
                        c.subject.insert(subject);
                        if !body.is_empty() {
                            c.body.insert(body.trim_start_matches('\n'));
                            c.field = Field::Body;
                        }
                    }
                    Field::Body => c.body.insert(&text),
                    Field::Review => {}
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        match (key.modifiers, key.code) {
            (KeyModifiers::ALT, KeyCode::Char('j' | 'k')) => {
                self.composer_scroll_error(if key.code == KeyCode::Char('j') {
                    1
                } else {
                    -1
                });
            }
            (_, KeyCode::Esc) => {
                self.mode = AppMode::Normal;
                self.workspace.invalidate();
                self.workspace.focus = Panel::Files;
                self.set_message(crate::tr!(
                    "Commit draft kept · c resumes · Space changes staging"
                ));
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => return self.submit_composer(),
            (KeyModifiers::CONTROL, KeyCode::Char('d' | 'u')) => {
                let delta = if key.code == KeyCode::Char('d') {
                    10
                } else {
                    -10
                };
                let max = self
                    .workspace
                    .patch
                    .lines
                    .len()
                    .saturating_sub(self.workspace.diff_height as usize);
                self.workspace.scroll = (i64::from(self.workspace.scroll) + delta)
                    .clamp(0, max.min(u16::MAX as usize) as i64)
                    as u16;
            }
            (_, KeyCode::Tab | KeyCode::BackTab) => {
                let c = &mut self.workspace.composer;
                c.field = match (c.field, key.code == KeyCode::BackTab) {
                    (Field::Subject, false) | (Field::Review, true) => Field::Body,
                    (Field::Body, false) | (Field::Subject, true) => Field::Review,
                    _ => Field::Subject,
                };
            }
            (_, KeyCode::Enter) if self.workspace.composer.field == Field::Subject => {
                return self.submit_composer()
            }
            _ if self.workspace.composer.field == Field::Review => {
                let c = &mut self.workspace.composer;
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => c.selected = c.selected.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => {
                        c.selected = (c.selected + 1).min(c.staged.len().saturating_sub(1))
                    }
                    KeyCode::Enter => c.field = Field::Subject,
                    KeyCode::Char(' ') => {
                        if let Some(row) = c
                            .staged
                            .get(c.selected)
                            .and_then(|i| self.workspace.files.get(*i))
                            .cloned()
                        {
                            crate::workspace::toggle_stage(&self.repo.repo, &row, true)?;
                            self.composer_refresh()?;
                            self.workspace.commit_error = None;
                            return Ok(());
                        }
                    }
                    _ => return Ok(()),
                }
                self.composer_select();
            }
            _ => {
                let c = &mut self.workspace.composer;
                if c.field == Field::Subject {
                    c.subject.key(key, false)
                } else {
                    c.body.key(key, true)
                }
            }
        }
        Ok(())
    }
}

impl App {
    pub(crate) fn composer_click(&mut self, x: u16, y: u16) {
        use unicode_width::UnicodeWidthChar;
        if self.workspace.busy() {
            return;
        }
        let Some(index) = self
            .workspace
            .composer
            .fields
            .iter()
            .position(|r| r.contains(ratatui::layout::Position::new(x, y)))
        else {
            return;
        };
        let rect = self.workspace.composer.fields[index];
        let c = &mut self.workspace.composer;
        c.field = [Field::Subject, Field::Body, Field::Review][index];
        if index == 2 {
            if y > rect.y && y < rect.bottom().saturating_sub(1) {
                c.selected =
                    (c.list_top + (y - rect.y - 1) as usize).min(c.staged.len().saturating_sub(1));
                self.composer_select();
            }
        } else if x > rect.x && y > rect.y && y < rect.bottom().saturating_sub(1) {
            let editor = if index == 0 {
                &mut c.subject
            } else {
                &mut c.body
            };
            let row = editor.top + (y - rect.y - 1) as usize;
            let target = editor.left + (x - rect.x - 1) as usize;
            let line = editor.text.split('\n').nth(row).unwrap_or("");
            let mut width = 0;
            let column = line
                .chars()
                .take_while(|c| {
                    width += c.width().unwrap_or(0);
                    width <= target
                })
                .count();
            editor.place(row, column);
        }
    }
    fn composer_scroll_error(&mut self, delta: i32) {
        if let Some(error) = &self.workspace.commit_error {
            let rect = self.workspace.composer.error_rect;
            let lines = ratatui::widgets::Paragraph::new(error.as_str())
                .wrap(ratatui::widgets::Wrap { trim: false })
                .line_count(rect.width.saturating_sub(2));
            let max = lines
                .saturating_sub(rect.height.saturating_sub(2) as usize)
                .min(u16::MAX as usize);
            self.workspace.composer.error_scroll = (i64::from(self.workspace.composer.error_scroll)
                + i64::from(delta))
            .clamp(0, max as i64) as u16;
        }
    }
    pub(crate) fn composer_scroll(&mut self, delta: i32, x: u16, y: u16) {
        if self.workspace.busy() {
            return;
        }
        if self
            .workspace
            .composer
            .error_rect
            .contains(ratatui::layout::Position::new(x, y))
        {
            self.composer_scroll_error(delta);
            return;
        }
        if self
            .workspace
            .composer
            .diff_rect
            .contains(ratatui::layout::Position::new(x, y))
        {
            let max = self
                .workspace
                .patch
                .lines
                .len()
                .saturating_sub(self.workspace.diff_height as usize);
            self.workspace.scroll = (i64::from(self.workspace.scroll) + i64::from(delta))
                .clamp(0, max.min(u16::MAX as usize) as i64)
                as u16;
        } else if self.workspace.composer.fields[2].contains(ratatui::layout::Position::new(x, y)) {
            let c = &mut self.workspace.composer;
            c.selected = (c.selected as i64 + i64::from(delta))
                .clamp(0, c.staged.len().saturating_sub(1) as i64)
                as usize;
            self.composer_select();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_moves_and_edits_unicode_without_corrupting_text() {
        let mut e = Editor::default();
        e.insert("日本語\nbody");
        e.key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE), true);
        e.key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), true);
        assert_eq!(e.text, "日本語body");
        e.key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), true);
        e.key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE), true);
        assert_eq!(e.text, "日本body");
        e.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), true);
        e.insert("理由");
        assert_eq!(e.text, "日本\n理由body");
        e.place(0, 1);
        e.insert("X");
        assert_eq!(e.text, "日X本\n理由body");
    }
    #[test]
    fn message_joins_subject_and_body_with_one_blank_line() {
        let mut c = Composer::default();
        c.subject.insert(" subject ");
        c.body.insert("\nfirst\n\nlast\n");
        assert_eq!(c.message(), "subject\n\nfirst\n\nlast");
    }
}
