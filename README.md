# pedigit

[![Crates.io](https://img.shields.io/crates/v/pedigit.svg)](https://crates.io/crates/pedigit)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/watabegg/pedigit/blob/main/LICENSE)

[日本語の README](https://github.com/watabegg/pedigit/blob/main/README.ja.md)

**pedigit** (pedigree + git) is a Git terminal user interface (TUI) written in Rust. It combines the commit graph visualization foundation of [keifu](https://github.com/trasta298/keifu) with keyboard-driven workflows inspired by [lazygit](https://github.com/jesseduffield/lazygit).

pedigit keeps the Git commit graph visible while you work, opening diff views only on demand. It is an independent project built upon keifu rather than a complete lazygit replacement.

![pedigit overview](https://raw.githubusercontent.com/watabegg/pedigit/main/docs/demo/overview.png)
*Real application session with a synthetic trail repository.*

## Features

- **Structured Interface:** Collapsible file tree with colored status indicators, dedicated tabs for Files / Worktrees / Submodules and Local / Remotes / Tags, contextual footer keybindings, and commit details showing per-file additions and deletions.
- **Everyday Git Operations:** Execute routine workflows such as checkout, branch creation/rename, tag creation, stashes, fetch/pull/push, merge/rebase, and cherry-pick/revert directly from the interface.

### On-Demand Diff View

Press `Enter` in the Files panel to open the diff view, and press `Esc` to close it.

![pedigit diff](https://raw.githubusercontent.com/watabegg/pedigit/main/docs/demo/diff.png)

- Syntax highlighting, line numbers, and word-level change indicators.
- Stage whole files or stage modified UTF-8 text hunks directly from the diff.

### Side-by-Side Commit Dialog

Press `Space` to stage files, then press `c` in the Files panel to open the commit dialog. If no files are staged, pedigit prompts for confirmation to stage all changes first.

![pedigit commit](https://raw.githubusercontent.com/watabegg/pedigit/main/docs/demo/commit.png)

- Inspect staged files and their diff side-by-side with your commit message editor.
- Press `Tab` to navigate between fields and the file list.
- Press `Enter` in the subject field or `Ctrl-s` to commit.
- Press `Esc` to dismiss the dialog while preserving your draft commit message during the same application session.

## Keybindings

| Key | Action |
| --- | --- |
| `2` | Switch to Files |
| `3` | Switch to Branches |
| `4` | Switch to Graph |
| `5` | Switch to Stashes |
| `6` | Switch to Commit Detail |
| `Tab` | Cycle panel focus |
| `[` / `]` | Switch tabs |
| `Space` | Stage / unstage file (in Files) |
| `Enter` | Open diff view (in Files / Graph) |
| `c` | Open commit dialog (in Files) |
| `?` | Show help |

## Installation

pedigit is distributed through crates.io and builds from source.

### Requirements

- Rust and Cargo
- Git
- Native C build toolchain (including `make` and `Perl` for bundled OpenSSL)

### Using cargo

```sh
cargo install pedigit --locked
```

### Using mise

```sh
mise use -g cargo:pedigit@latest
```

### Usage

Navigate to any Git repository and launch:

```sh
cd path/to/repo
pedigit
```

## Localization & Documentation

pedigit supports 9 UI languages and automatically detects your terminal locale. You can also specify a language manually:

```sh
pedigit --language ja
```

- [Configuration Documentation](https://github.com/watabegg/pedigit/blob/main/docs/configuration.md)
- [Usage Guide (Japanese)](https://github.com/watabegg/pedigit/blob/main/docs/usage.md)

## Credits & License

Distributed under the [MIT License](https://github.com/watabegg/pedigit/blob/main/LICENSE).

pedigit builds upon the graph foundation and initial codebase of [keifu](https://github.com/trasta298/keifu). Interaction workflows are inspired by [lazygit](https://github.com/jesseduffield/lazygit), and UI translations reuse its translations. Original copyright notices are preserved; localized translations follow the [lazygit translation license](https://github.com/watabegg/pedigit/blob/main/locales/LICENSE.lazygit).
