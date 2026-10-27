# lazykeifu

[![Crates.io](https://img.shields.io/crates/v/lazykeifu.svg)](https://crates.io/crates/lazykeifu)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/watabegg/lazykeifu/blob/main/LICENSE)

[日本語の README](https://github.com/watabegg/lazykeifu/blob/main/README.ja.md)

**lazykeifu** is a Git terminal user interface (TUI) written in Rust. It combines the commit graph visualization foundation of [keifu](https://github.com/trasta298/keifu) with keyboard-driven workflows inspired by [lazygit](https://github.com/jesseduffield/lazygit).

lazykeifu keeps the Git commit graph visible while you work, opening diff views only on demand. It is an independent project built upon keifu rather than a complete lazygit replacement.

![lazykeifu overview](https://raw.githubusercontent.com/watabegg/lazykeifu/main/docs/demo/overview.png)
*Real application session with a synthetic trail repository.*

## Features

- **Structured Interface:** Collapsible file tree with colored status indicators, dedicated tabs for Files / Worktrees / Submodules and Local / Remotes / Tags, contextual footer keybindings, and commit details showing per-file additions and deletions.
- **Everyday Git Operations:** Execute routine workflows such as checkout, branch creation/rename, tag creation, stashes, fetch/pull/push, merge/rebase, and cherry-pick/revert directly from the interface.

### On-Demand Diff View

Press `Enter` in the Files panel to open the diff view, and press `Esc` to close it.

![lazykeifu diff](https://raw.githubusercontent.com/watabegg/lazykeifu/main/docs/demo/diff.png)

- Syntax highlighting, line numbers, and word-level change indicators.
- Stage whole files or stage modified UTF-8 text hunks directly from the diff.

### Side-by-Side Commit Dialog

Press `Space` to stage files, then press `c` in the Files panel to open the commit dialog. If no files are staged, lazykeifu prompts for confirmation to stage all changes first.

![lazykeifu commit](https://raw.githubusercontent.com/watabegg/lazykeifu/main/docs/demo/commit.png)

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

The `lazykeifu` crate provides the `lzkf` binary.

### Prebuilt Binaries

Prebuilt archives are available on [GitHub Releases](https://github.com/watabegg/lazykeifu/releases). Official releases use the tag namespace `lazykeifu-v<VERSION>` (such as `lazykeifu-v0.1.6`). Existing tags with plain `v*` prefixes are preserved upstream history from `keifu` and do not represent releases of this project.

Prebuilt archives are provided for:
- Linux x86_64 (Ubuntu 22.04 / glibc 2.35+)
- Linux aarch64 (Ubuntu 24.04 / glibc 2.39+)
- macOS x86_64 and aarch64 (macOS 14+ baseline)
- Windows x86_64

Archives are named `lzkf-0.1.6-<TARGET>.tar.gz` (or `.zip` for Windows) and contain `LICENSE`, `LICENSE.lazygit`, `THIRD_PARTY.json`, and `README.md`. Checksums are provided in `SHA256SUMS` among the release assets.

**Requirements and Platform Notes:**
- Git must be installed and on your `PATH`. Prebuilt binaries do not require a Rust or C compiler.
- **Windows:** Editor launching requires `sh` from Git for Windows to be present on your `PATH`.
- **macOS:** Binaries are unsigned and not notarized.

To install, download the archive for your architecture, extract it, and place `lzkf` (or `lzkf.exe`) on your `PATH`. For example, on Linux and macOS:

```sh
mkdir -p ~/.local/bin
install -m755 lzkf ~/.local/bin/lzkf
```

Ensure `~/.local/bin` is in your `PATH`.

### Building from Source

Alternatively, install from source using Cargo or mise. Source builds require Git, Rust/Cargo, and native build tools including `make` and `Perl` (required to compile the bundled OpenSSL dependency).

Via Cargo:
```sh
cargo install lazykeifu --locked
```

Via mise (builds from source via Cargo; pin the version explicitly because age policies exclude fresh releases from `latest`):
```sh
mise use -g cargo:lazykeifu@0.1.6
```

### Usage

Run `lzkf` inside any Git repository:
```sh
cd path/to/repo
lzkf
```

### Reporting Issues

Report bugs on [GitHub Issues](https://github.com/watabegg/lazykeifu/issues). Please include your `lzkf` version, operating system, terminal emulator, and steps to reproduce.

## Localization & Documentation

lazykeifu supports 9 UI languages and automatically detects your terminal locale. You can also specify a language manually:

```sh
lzkf --language ja
```

- [Configuration Documentation](https://github.com/watabegg/lazykeifu/blob/main/docs/configuration.md)
- [Usage Guide (Japanese)](https://github.com/watabegg/lazykeifu/blob/main/docs/usage.md)

## Credits & License

Distributed under the [MIT License](https://github.com/watabegg/lazykeifu/blob/main/LICENSE).

lazykeifu builds upon the graph foundation and initial codebase of [keifu](https://github.com/trasta298/keifu). Interaction workflows are inspired by [lazygit](https://github.com/jesseduffield/lazygit), and UI translations reuse its translations. Original copyright notices are preserved; localized translations follow the [lazygit translation license](https://github.com/watabegg/lazykeifu/blob/main/locales/LICENSE.lazygit).
