# REQ-001: `--testing` flag using a `_testing` bucket suffix

- **Status:** Documented, not implemented
- **Added:** 2026-11-05
- **Owner:** maintainer

## Rationale

Testing the aw-qt integration currently writes into the real bucket. A `--testing` flag would send heartbeats to a separate bucket so test data does not mix with real tracking.

## Design

- Add a `--testing` flag in `main.rs`. When set, `AwTaskWatcherClient::new` appends `_testing` to the bucket id.
- Affects: `main.rs`, `activitywatch.rs` (bucket id construction). See [../modules.md](../modules.md).
- With the flag, `config::load(true)` is used (REQ-009), so the `[aw-taskwarrior-testing]` and `[server-testing]` config sections apply.
- No new dependency; see [../dependencies.md](../dependencies.md). Argument parsing is shared with REQ-002.

## Acceptance criteria

- [ ] Without the flag, the bucket id is `aw-watcher-taskwarrior_{hostname}`.
- [ ] With the flag, the bucket id ends in `_testing`.

## Code references

- Implemented in: (none yet)
