# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Formatting (required)

- Format all Rust code with `cargo fmt`. Do not hand-format code against rustfmt.
- Before finishing any change, run `cargo fmt --check` and make sure it exits 0.
- Do not add `rustfmt.toml` or change formatter settings without asking. Use rustfmt defaults.

## Verification before finishing

- `cargo clippy --all-targets` must not introduce new warnings in code you touch.
- `cargo test` must pass. Add unit tests for new logic in the same module, following the existing `#[cfg(test)]` pattern.

## Documentation-first workflow (required)

- Reference docs live in `docs/`: `architecture.md`, `modules.md`, `dependencies.md`, `requirements/`, and `STATUS.md`.
- Every new requirement is written or updated in `docs/requirements/` (copy `_template.md`) and committed before any code commit for that requirement.
- Code must never be ahead of docs. Temporary drift is allowed only when docs are ahead and the item is listed in `docs/STATUS.md`.
- A change that adds, removes, or renames a module, public function, struct, or crate must update `docs/modules.md` or `docs/dependencies.md` and their Mermaid diagrams in the same change.
- When code for a requirement lands, set its status in `docs/requirements/` and `docs/STATUS.md` in that same commit.
- Diagrams are Mermaid code blocks. Do not add image files.

## Workspace and providers

- The repo is a Cargo workspace. Run `cargo` commands from the repository root; they cover every crate.
- `aw-taskwarrior-core` must keep zero dependencies.
- The watcher (`crates/aw-taskwarrior`) must not import provider internals. It uses only `providers::build`, `TaskProvider`, and `ActiveTask`.
- A new provider needs a requirement in `docs/requirements/` first, then its own crate, a feature, and one `cfg` arm in `crates/aw-taskwarrior/src/providers.rs`.

## Project notes

- The CLI provider polls TaskWarrior with `task rc.hooks:off +ACTIVE export`. Keep `rc.hooks:off` so the query does not trigger user hooks.
- Empty or null values must be omitted from ActivityWatch heartbeat payloads. Do not send them.
- Config is optional. Every setting needs a built-in default in `crates/aw-taskwarrior/src/config.rs`, so the daemon runs with no config file.
