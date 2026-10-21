---
name: release
description: Publish lazykeifu to crates.io and verify Cargo and mise installation
---

# Release lazykeifu

- Distribution: crates.io, with `mise use -g cargo:lazykeifu@VERSION` using its Cargo backend. GitHub binary releases use `.github/workflows/release.yaml`; do not modify upstream Homebrew taps.
- Follow the user's authorized release scope. Select the version from actual changes; update `Cargo.toml` and `Cargo.lock` together.
- Run `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --all-targets --locked -- -D warnings` for code changes. Verify the packaged source with `cargo publish --dry-run --locked`.
- Check `cargo package --list --locked`: source, embedded locales, README, and both MIT notices must be included. Screenshots stay on GitHub.
- Review and commit only the release's changes, then push to `origin` (`watabegg/lazykeifu`). Preserve upstream history and tags; never push all inherited tags as a release step.
- Publish with `cargo publish --locked`. If credentials are missing, ask the user to run `cargo login`; never request or print the token.
- Verify `cargo install lazykeifu --version VERSION --locked` and `lzkf --version`. Verify `mise exec cargo:lazykeifu@VERSION -- lzkf --version` using isolated mise directories when testing.
- Report the actual published version and installation results. Link the actual GitHub Release after all assets pass verification. Homebrew is not provided.

- Pin the README mise command to the published version: mise may hide fresh releases from `@latest` due to its minimum release age. Update that pin with each release.

- Refresh `THIRD_PARTY.json` with cargo-bundle-licenses 4.2.0, preserving manually supplied upstream license texts with `--previous THIRD_PARTY.json`; inspect missing-license warnings.
- Run the Release workflow on main for a build rehearsal. After CI and all five native binaries pass, create only `lazykeifu-vVERSION` at the release commit and push that tag. Never overwrite upstream `v*` tags.
- The tag workflow reruns CI, builds and smoke-tests native binaries, includes notices, verifies checksums, and publishes the GitHub Release. It does not publish to crates.io; use the separate Cargo step above. If the tag push does not trigger a run, dispatch Release with `--ref lazykeifu-vVERSION`.
- Download the published host archive, verify SHA256SUMS, and run its `lzkf --version` before reporting completion.

- Keep `bundled_source_notices` and `supplemental_sources` in THIRD_PARTY.json when regenerating it; refresh embedded native library notices from the matching locked crate sources.
