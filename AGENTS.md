# lazykeifu development notes

lazykeifu is a Rust TUI (ratatui + crossterm) that visualizes Git commit graphs.

## Build / test

```bash
cargo build
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```

## Debugging the TUI yourself

lazykeifu has a remote-control debug server (`--debug-listen`), so you can drive
the real app and see its screen without a human. Use the **debug-tui** skill
(`.claude/skills/debug-tui/SKILL.md`) for the full workflow and its gotchas;
the wire protocol is documented in docs/debugging.md. Always reproduce
TUI-affecting bugs through that interface (drive → dump → assert) before and
after a fix.

## Architecture quick map

- `src/app.rs` — application state machine (`AppMode`), async diff loading
  via mpsc + threads, staging/commit/push, focus model
- `src/ui/` — ratatui widgets; `ui::draw` records pane rects into
  `App::layout` for mouse hit-testing
- `src/mouse.rs` — position-based mouse routing (uses `App::layout`)
- `src/keybindings.rs` — key → `Action` mapping per mode
- `src/git/` — git2-based operations; fetch/push shell out to `git` for auth
- `src/debug_server.rs` — NDJSON-over-TCP remote control described above

## Fork development

- Read `docs/development-plan.md` before non-trivial implementation.
- `origin` is `watabegg/lazykeifu`; `upstream` is `trasta298/keifu`.
- Preserve upstream copyright notices and graph regression tests.
- The crate is `lazykeifu` and the executable is `lzkf`; use `LAZYKEIFU_LOG` and the `lazykeifu` config directory.
- Distribution uses crates.io and the mise Cargo backend. Follow `.agents/skills/release/SKILL.md`. GitHub binary releases use `.github/workflows/release.yaml` and `lazykeifu-vVERSION` tags; plain `v*` tags belong to upstream keifu.
