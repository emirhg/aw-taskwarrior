# REQ-003: Structured logging with configurable levels

- **Status:** Documented, not implemented
- **Added:** 2026-11-05
- **Owner:** maintainer

## Rationale

The log has only INFO and ERROR, with free-form text. Levels and structure would make filtering easier.

## Design

- Replace `src/log.rs` helpers with a level-aware logger (error, warn, info, debug).
- Level set by a config key; default stays INFO.
- Output stays on stdout and stderr, so aw-qt can still read it (see [../architecture.md](../architecture.md) design decisions).
- Affects: `log.rs`, `config.rs`, and call sites in `main.rs` and `activitywatch.rs`. See [../modules.md](../modules.md).

## Acceptance criteria

- [ ] A `log_level` config key selects the minimum level.
- [ ] Default output format and streams are unchanged at INFO.

## Code references

- Implemented in: (none yet)
