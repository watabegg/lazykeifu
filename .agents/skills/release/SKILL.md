---
name: release
description: Publish pedigit to crates.io and verify Cargo and mise installation
---

# Release pedigit

- Distribution: crates.io, with `mise use -g cargo:pedigit@latest` using its Cargo backend. GitHub binary automation stays disabled; do not modify upstream Homebrew taps.
- Follow the user's authorized release scope. Select the version from actual changes; update `Cargo.toml` and `Cargo.lock` together.
- Run `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --all-targets --locked -- -D warnings` for code changes. Verify the packaged source with `cargo publish --dry-run --locked`.
- Check `cargo package --list --locked`: source, embedded locales, README, and both MIT notices must be included. Screenshots stay on GitHub.
- Review and commit only the release's changes, then push to `origin` (`watabegg/pedigit`). Preserve upstream history and tags; never push all inherited tags as a release step.
- Publish with `cargo publish --locked`. If credentials are missing, ask the user to run `cargo login`; never request or print the token.
- Verify `cargo install pedigit --version VERSION --locked` and `pedigit --version`. Verify `mise exec cargo:pedigit@VERSION -- pedigit --version` using isolated mise directories when testing.
- Report the actual published version and installation results. Do not claim a GitHub binary release or Homebrew availability.
