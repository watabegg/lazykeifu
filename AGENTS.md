# pedigit development notes

pedigit is a Rust TUI (ratatui + crossterm) that visualizes Git commit graphs.

## Build / test

```bash
cargo build
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
```

## Debugging the TUI yourself

pedigit has a remote-control debug server (`--debug-listen`), so you can drive
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
- `origin` is `watabegg/pedigit`; `upstream` is `trasta298/keifu`.
- Preserve upstream copyright notices and graph regression tests.
- The executable/crate is `pedigit`; use `PEDIGIT_LOG` and the `pedigit` config directory.
- Release automation is parked in `.github/workflows/release.yaml.disabled`; Cargo publishing is disabled until distribution is ready. Do not use the inherited release skill without adapting it first.
