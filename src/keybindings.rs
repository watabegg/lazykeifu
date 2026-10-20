//! Keybindings

#[cfg(windows)]
use crossterm::event::KeyEventKind;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::AppMode;

pub fn map_key_to_action(key: KeyEvent, mode: &AppMode) -> Option<Action> {
    #[cfg(windows)]
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match mode {
        AppMode::Normal => map_normal_mode(key),
        AppMode::Help => map_help_mode(key),
        AppMode::Input { action, .. } => {
            if *action == crate::app::InputAction::Search {
                map_search_mode(key)
            } else {
                map_input_mode(key)
            }
        }
        AppMode::Confirm { .. } => map_confirm_mode(key),
        AppMode::Error { .. } => map_error_mode(key),
        AppMode::FileSelect { .. } => map_file_select_mode(key),
        AppMode::FileDiff { .. } => map_file_diff_mode(key),
    }
}

fn map_normal_mode(key: KeyEvent) -> Option<Action> {
    use crate::workspace::Panel;
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER)
    {
        return match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => Some(Action::PageDown),
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => Some(Action::PageUp),
            _ => None,
        };
    }
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => Some(Action::PageDown),
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => Some(Action::PageUp),
        (_, KeyCode::Char('1')) => Some(Action::FocusPanel(Panel::Status)),
        (_, KeyCode::Char('2')) => Some(Action::FocusPanel(Panel::Files)),
        (_, KeyCode::Char('3')) => Some(Action::FocusPanel(Panel::Branches)),
        (_, KeyCode::Char('4')) => Some(Action::FocusPanel(Panel::Graph)),
        (_, KeyCode::Char('5')) => Some(Action::FocusPanel(Panel::Stashes)),
        (_, KeyCode::Char('0')) => Some(Action::FocusPanel(Panel::Diff)),
        (_, KeyCode::Char('j') | KeyCode::Down) => Some(Action::MoveDown),
        (_, KeyCode::Char('k') | KeyCode::Up) => Some(Action::MoveUp),
        (_, KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab) => Some(Action::FocusPrevious),
        (_, KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab) => Some(Action::FocusNext),
        (_, KeyCode::Char('H')) => Some(Action::ScrollLeft),
        (_, KeyCode::Char('L')) => Some(Action::ScrollRight),
        (_, KeyCode::Char('g') | KeyCode::Char('<') | KeyCode::Home) => Some(Action::GoToTop),
        (_, KeyCode::Char('G') | KeyCode::Char('>') | KeyCode::End) => Some(Action::GoToBottom),
        (_, KeyCode::Enter) => Some(Action::Checkout),
        (_, KeyCode::Char(' ')) => Some(Action::StageToggle),
        (_, KeyCode::Char(']')) => Some(Action::NextHunk),
        (_, KeyCode::Char('[')) => Some(Action::PrevHunk),
        (_, KeyCode::Char('v')) => Some(Action::ToggleDiffSide),
        (_, KeyCode::Char('a')) => Some(Action::StageAll),
        (_, KeyCode::Char('u')) => Some(Action::UnstageAll),
        (_, KeyCode::Char('c')) => Some(Action::CommitDialog),
        (_, KeyCode::Char('M')) => Some(Action::Merge),
        (_, KeyCode::Char('r')) => Some(Action::Rebase),
        (_, KeyCode::Char('b')) => Some(Action::CreateBranch),
        (_, KeyCode::Char('d')) => Some(Action::DeleteBranch),
        (_, KeyCode::Char('f')) => Some(Action::Fetch),
        (_, KeyCode::Char('P')) => Some(Action::Push),
        (_, KeyCode::Char('p')) => Some(Action::Pull),
        (_, KeyCode::Char('e')) => Some(Action::EditFile),
        (_, KeyCode::Char('C')) => Some(Action::ContinueOperation),
        (_, KeyCode::Char('A')) => Some(Action::AbortOperation),
        (_, KeyCode::Char('s')) => Some(Action::StashSave),
        (_, KeyCode::Char('/')) => Some(Action::Search),
        (_, KeyCode::Char('R')) => Some(Action::Refresh),
        (_, KeyCode::Char('o')) => Some(Action::ToggleRemoteBranches),
        (_, KeyCode::Char('t')) => Some(Action::ToggleTags),
        (_, KeyCode::Char('?')) => Some(Action::ToggleHelp),
        (_, KeyCode::Char('q')) => Some(Action::Quit),
        (_, KeyCode::Esc) => Some(Action::Cancel),
        _ => None,
    }
}

fn map_help_mode(key: KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        (_, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) => Some(Action::ToggleHelp),
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            Some(Action::ScrollDown)
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            Some(Action::ScrollUp)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => Some(Action::ScrollPageDown),
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => Some(Action::ScrollPageUp),
        (KeyModifiers::NONE, KeyCode::PageDown) => Some(Action::PageDown),
        (KeyModifiers::NONE, KeyCode::PageUp) => Some(Action::PageUp),
        (KeyModifiers::NONE, KeyCode::Char('g')) | (KeyModifiers::NONE, KeyCode::Home) => {
            Some(Action::ScrollToTop)
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) | (KeyModifiers::NONE, KeyCode::End) => {
            Some(Action::ScrollToBottom)
        }
        _ => None,
    }
}

fn map_input_mode(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Enter => Some(Action::Confirm),
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Backspace => Some(Action::InputBackspace),
        KeyCode::Char(c) => Some(Action::InputChar(c)),
        _ => None,
    }
}

fn map_search_mode(key: KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        // Navigation in dropdown (Tab doesn't move graph)
        (KeyModifiers::NONE, KeyCode::Up) => Some(Action::SearchSelectUp),
        (KeyModifiers::NONE, KeyCode::Down) => Some(Action::SearchSelectDown),
        (KeyModifiers::CONTROL, KeyCode::Char('k')) => Some(Action::SearchSelectUp),
        (KeyModifiers::CONTROL, KeyCode::Char('j')) => Some(Action::SearchSelectDown),
        (KeyModifiers::NONE, KeyCode::Tab) => Some(Action::SearchSelectDownQuiet),
        (KeyModifiers::SHIFT, KeyCode::BackTab) => Some(Action::SearchSelectUpQuiet),
        // Standard input actions
        (_, KeyCode::Enter) => Some(Action::Confirm),
        (_, KeyCode::Esc) => Some(Action::Cancel),
        (_, KeyCode::Backspace) | (_, KeyCode::Delete) => Some(Action::InputBackspace),
        (_, KeyCode::Char(c)) => Some(Action::InputChar(c)),
        _ => None,
    }
}

fn map_confirm_mode(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Enter => Some(Action::Confirm),
        KeyCode::Char('n') | KeyCode::Esc => Some(Action::Cancel),
        _ => None,
    }
}

fn map_error_mode(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => Some(Action::Cancel),
        _ => None,
    }
}

fn map_file_select_mode(key: KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            Some(Action::FileSelectDown)
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            Some(Action::FileSelectUp)
        }
        (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::OpenFileDiff),
        // Staging (uncommitted changes only)
        (KeyModifiers::NONE, KeyCode::Char('s')) => Some(Action::StageToggle),
        (KeyModifiers::NONE, KeyCode::Char('a')) => Some(Action::StageAll),
        (KeyModifiers::NONE, KeyCode::Char('u')) => Some(Action::UnstageAll),
        (KeyModifiers::NONE, KeyCode::Char('c')) => Some(Action::CommitDialog),
        (KeyModifiers::NONE, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
            Some(Action::Cancel)
        }
        _ => None,
    }
}

fn map_file_diff_mode(key: KeyEvent) -> Option<Action> {
    match (key.modifiers, key.code) {
        // Line scroll
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            Some(Action::ScrollDown)
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            Some(Action::ScrollUp)
        }
        // Half-page scroll
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => Some(Action::ScrollPageDown),
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => Some(Action::ScrollPageUp),
        // Full-page scroll
        (KeyModifiers::CONTROL, KeyCode::Char('f')) | (KeyModifiers::NONE, KeyCode::PageDown) => {
            Some(Action::PageDown)
        }
        (KeyModifiers::CONTROL, KeyCode::Char('b')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
            Some(Action::PageUp)
        }
        // Top/bottom
        (KeyModifiers::NONE, KeyCode::Char('g')) | (KeyModifiers::NONE, KeyCode::Home) => {
            Some(Action::ScrollToTop)
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) | (KeyModifiers::NONE, KeyCode::End) => {
            Some(Action::ScrollToBottom)
        }
        // Horizontal scroll
        (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
            Some(Action::ScrollLeft)
        }
        (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) => {
            Some(Action::ScrollRight)
        }
        (KeyModifiers::NONE, KeyCode::Char('0')) => Some(Action::ScrollToLineStart),
        // Hunk jump
        (_, KeyCode::Char(']')) => Some(Action::NextHunk),
        (_, KeyCode::Char('[')) => Some(Action::PrevHunk),
        // File navigation
        (KeyModifiers::NONE, KeyCode::Char('n')) => Some(Action::NextFile),
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => Some(Action::PrevFile),
        // Exit
        (KeyModifiers::NONE, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('q')) => {
            Some(Action::Cancel)
        }
        _ => None,
    }
}

#[cfg(test)]
mod workspace_tests {
    use super::*;
    #[test]
    fn modified_space_cannot_stage_and_push_is_uppercase_only() {
        assert_eq!(
            map_key_to_action(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL),
                &AppMode::Normal
            ),
            None
        );
        assert_eq!(
            map_key_to_action(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                &AppMode::Normal
            ),
            Some(Action::StageToggle)
        );
        assert_eq!(
            map_key_to_action(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &AppMode::Normal
            ),
            Some(Action::Pull)
        );
        assert_eq!(
            map_key_to_action(
                KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT),
                &AppMode::Normal
            ),
            Some(Action::Push)
        );
    }
}
