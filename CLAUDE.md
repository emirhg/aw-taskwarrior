# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Formatting (required)

- Format all Rust code with `cargo fmt`. Do not hand-format code against rustfmt.
- Before finishing any change, run `cargo fmt --check` and make sure it exits 0.
- Do not add `rustfmt.toml` or change formatter settings without asking. Use rustfmt defaults.

## Verification before finishing

- `cargo clippy --all-targets` must not introduce new warnings in code you touch.
- `cargo test` must pass. Add unit tests for new logic in the same module, following the existing `#[cfg(test)]` pattern.

## Project notes

- The daemon polls TaskWarrior with `task rc.hooks:off +ACTIVE export`. Keep `rc.hooks:off` so the query does not trigger user hooks.
- Empty or null values must be omitted from ActivityWatch heartbeat payloads. Do not send them.
- Config is optional. Every setting needs a built-in default in `src/config.rs`, so the daemon runs with no config file.
