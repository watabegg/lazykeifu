//! User action definitions

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    FocusPanel(crate::workspace::Panel),
    FocusPrevious,
    TreeLeft,
    TreeRight,
    ToggleDiffSide,
    Unavailable,
    // Navigation
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    GoToTop,
    GoToBottom,
    JumpToHead,
    NextBranch,
    PrevBranch,
    BranchLeft,
    BranchRight,

    // Git operations
    Checkout,
    CreateBranch,
    DeleteBranch,
    Fetch,
    Merge,
    Rebase,
    CherryPick,
    Revert,
    LoadMoreHistory,

    // Staging / commit / push
    StageToggle,
    StageAll,
    UnstageAll,
    CommitDialog,
    Push,
    Pull,
    StashSave,
    EditFile,
    ContinueOperation,
    AbortOperation,

    // Clipboard
    CopyHash,
    CopyBranch,

    // UI
    FocusNext,
    ToggleHelp,
    Search,
    Refresh,
    ToggleRemoteBranches,
    ToggleTags,
    Quit,

    // Dialogs
    Confirm,
    Cancel,
    InputChar(char),
    CommitInput(crossterm::event::KeyEvent),
    Paste(String),
    InputBackspace,

    // Search dropdown
    SearchSelectUp,
    SearchSelectDown,
    SearchSelectUpQuiet,   // Tab navigation (no graph jump)
    SearchSelectDownQuiet, // Tab navigation (no graph jump)

    // File diff
    EnterFileSelect,
    FileSelectUp,
    FileSelectDown,
    OpenFileDiff,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollToTop,
    ScrollToBottom,
    ScrollLeft,
    ScrollRight,
    ScrollToLineStart,
    NextFile,
    PrevFile,
    NextHunk,
    PrevHunk,
}
