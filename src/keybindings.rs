//! Keybindings

#[cfg(windows)]
use crossterm::event::KeyEventKind;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::AppMode;

pub fn map_mode_key(key: KeyEvent, mode: &AppMode) -> Option<Action> {
    #[cfg(windows)]
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match mode {
        AppMode::GitMenu(_) => map_normal_mode(key),
        AppMode::Normal => map_normal_mode(key),
        AppMode::Help => map_help_mode(key),
        AppMode::Input { action, .. } => {
            if *action == crate::app::InputAction::Search {
                map_search_mode(key)
            } else if *action == crate::app::InputAction::CommitMessage {
                Some(Action::CommitInput(key))
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

/// Context is part of the binding: R renames a branch, r rewords a commit.
pub fn map_key_to_action(key: KeyEvent, app: &crate::app::App) -> Option<Action> {
    #[cfg(windows)]
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if !matches!(app.mode, AppMode::Normal) || app.workspace.help {
        return map_mode_key(key, &app.mode);
    }
    use crate::{
        workspace::Panel,
        workspace_tabs::{FilesTab, RefsTab},
    };
    let focus = app.workspace.focus;
    let modified = key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);
    if modified {
        return match (key.modifiers, key.code, focus) {
            (KeyModifiers::ALT, KeyCode::Char(' '), Panel::Graph) => Some(Action::CheckoutRevision),
            (KeyModifiers::CONTROL, KeyCode::Char('l'), Panel::Graph) => Some(Action::LogMenu),
            (KeyModifiers::CONTROL, KeyCode::Char('r'), Panel::Graph) => {
                Some(Action::ClearCopiedCommit)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('o'), Panel::Graph | Panel::CommitDetail) => {
                Some(Action::CopyHash)
            }
            _ => map_normal_mode(key),
        };
    }
    let local = app.workspace.tabs.refs == RefsTab::Local;
    let files = app.workspace.tabs.files == FilesTab::Files;
    let working = app.workspace.commit.is_none();
    match (focus, key.code) {
        (Panel::Files, KeyCode::Char('a')) if files && working => Some(Action::ToggleStageAll),
        (Panel::Files, KeyCode::Char('A')) if files && working => Some(Action::AmendHead),
        (Panel::Files, KeyCode::Char('c')) if files && working => Some(Action::CommitDialog),
        (Panel::Files, KeyCode::Char('d' | 'x')) if files && working => Some(Action::DeleteBranch),
        (Panel::Files, KeyCode::Char('e')) if files && working => Some(Action::EditFile),
        (Panel::Files, KeyCode::Char('r')) if files => Some(Action::Refresh),
        (Panel::Files, KeyCode::Char('s')) if files && working => Some(Action::StashSave),
        (Panel::Files, KeyCode::Char('f')) => Some(Action::Fetch),
        (Panel::Files, KeyCode::Char('-')) if files => Some(Action::CollapseAll),
        (Panel::Files, KeyCode::Char('=')) if files => Some(Action::ExpandAll),
        (Panel::Branches, KeyCode::Char('R')) if local => Some(Action::RenameBranch),
        (Panel::Branches, KeyCode::Char('R')) => Some(Action::Refresh),
        (Panel::Branches, KeyCode::Char('n')) if local => Some(Action::CreateBranch),
        (Panel::Branches, KeyCode::Char('d')) if local => Some(Action::DeleteBranch),
        (Panel::Branches, KeyCode::Char('M')) if app.workspace.tabs.refs != RefsTab::Tags => {
            Some(Action::Merge)
        }
        (Panel::Branches, KeyCode::Char('r')) if app.workspace.tabs.refs != RefsTab::Tags => {
            Some(Action::Rebase)
        }
        (Panel::Branches, KeyCode::Char('T')) if app.workspace.tabs.refs != RefsTab::Tags => {
            Some(Action::CreateTag)
        }
        (Panel::Branches, KeyCode::Char('g')) => Some(Action::ResetMenu),
        (Panel::Branches, KeyCode::Enter) if app.workspace.tabs.refs != RefsTab::Tags => {
            Some(Action::InspectBranch)
        }
        (Panel::Branches, KeyCode::Char('P')) if app.workspace.tabs.refs == RefsTab::Tags => None,
        (Panel::Graph, KeyCode::Char('r')) => Some(Action::RewordHead),
        (Panel::Graph, KeyCode::Char('A')) => Some(Action::AmendHead),
        (Panel::Graph, KeyCode::Char('g')) => Some(Action::ResetMenu),
        (Panel::Graph, KeyCode::Char('T')) => Some(Action::CreateTag),
        (Panel::Graph, KeyCode::Char('t')) => Some(Action::Revert),
        (Panel::Graph, KeyCode::Char('C')) => Some(Action::CopyCommit),
        (Panel::Graph, KeyCode::Char('V')) => Some(Action::PasteCommit),
        (Panel::Graph, KeyCode::Char(' ')) => Some(Action::CheckoutGraphBranch),
        (Panel::Graph, KeyCode::Char('n')) => Some(Action::CreateBranch),
        // These have other meanings in Lazygit; do not map them to global actions.
        (Panel::Graph, KeyCode::Char('R' | 'y')) => None,
        (Panel::Stashes, KeyCode::Char('g')) => Some(Action::StashPop),
        (Panel::Stashes, KeyCode::Char('d')) => Some(Action::DeleteBranch),
        (Panel::Diff, KeyCode::Left | KeyCode::Char('h')) => Some(Action::PrevHunk),
        (Panel::Diff, KeyCode::Right | KeyCode::Char('l')) => Some(Action::NextHunk),
        (Panel::Diff, KeyCode::Char('n')) => Some(Action::NextFile),
        (Panel::Diff, KeyCode::Char('N')) => Some(Action::PrevFile),
        (Panel::Diff, KeyCode::Tab) if working && app.workspace.stash_preview.is_none() => {
            Some(Action::ToggleDiffSide)
        }
        (Panel::Diff, KeyCode::Char('c')) if working && app.workspace.stash_preview.is_none() => {
            Some(Action::CommitDialog)
        }
        (Panel::Status, KeyCode::Char('f')) => Some(Action::Fetch),
        _ => map_normal_mode(key),
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
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => Some(Action::Quit),
            _ => None,
        };
    }
    match key.code {
        KeyCode::Char('1') => Some(Action::FocusPanel(Panel::Status)),
        KeyCode::Char('2') => Some(Action::FocusPanel(Panel::Files)),
        KeyCode::Char('3') => Some(Action::FocusPanel(Panel::Branches)),
        KeyCode::Char('4') => Some(Action::FocusPanel(Panel::Graph)),
        KeyCode::Char('5') => Some(Action::FocusPanel(Panel::Stashes)),
        KeyCode::Char('6') => Some(Action::FocusPanel(Panel::CommitDetail)),
        KeyCode::Char('0') => Some(Action::FocusPanel(Panel::Diff)),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::MoveUp),
        KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab => Some(Action::FocusPrevious),
        KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => Some(Action::FocusNext),
        KeyCode::Char('H') => Some(Action::ScrollLeft),
        KeyCode::Char('L') => Some(Action::ScrollRight),
        KeyCode::Char('<') | KeyCode::Home => Some(Action::GoToTop),
        KeyCode::Char('>') | KeyCode::End => Some(Action::GoToBottom),
        KeyCode::Char(',') | KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::Char('.') | KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Enter => Some(Action::Checkout),
        KeyCode::Char(' ') => Some(Action::StageToggle),
        KeyCode::Char(']') => Some(Action::NextHunk),
        KeyCode::Char('[') => Some(Action::PrevHunk),
        KeyCode::Char('@') => Some(Action::GitMenu),
        KeyCode::Char('m') => Some(Action::RecoveryMenu),
        KeyCode::Char('P') => Some(Action::Push),
        KeyCode::Char('p') => Some(Action::Pull),
        KeyCode::Char('/') => Some(Action::Search),
        KeyCode::Char('R') => Some(Action::Refresh),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Esc => Some(Action::Cancel),
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
            map_mode_key(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL),
                &AppMode::Normal
            ),
            None
        );
        assert_eq!(
            map_mode_key(
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                &AppMode::Normal
            ),
            Some(Action::StageToggle)
        );
        assert_eq!(
            map_mode_key(
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
                &AppMode::Normal
            ),
            Some(Action::Pull)
        );
        assert_eq!(
            map_mode_key(
                KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT),
                &AppMode::Normal
            ),
            Some(Action::Push)
        );
    }
}
