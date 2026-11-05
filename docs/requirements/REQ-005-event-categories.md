# REQ-005: Event categorization by task type or project

- **Status:** Documented, not implemented
- **Added:** 2026-11-05
- **Owner:** maintainer

## Rationale

ActivityWatch events currently carry the task title, project and tags but no category. Categories would let ActivityWatch group work time without parsing titles.

## Design

- Map a task's project or tags to a category name in config.
- Add a `category` key to the heartbeat data in `send_heartbeat` only when a mapping matches, consistent with the rule that empty values are omitted.
- Affects: `config.rs` (mapping), `activitywatch.rs` (heartbeat data). See [../modules.md](../modules.md).

## Acceptance criteria

- [ ] A matching project or tag adds `category` to the heartbeat data.
- [ ] No matching mapping leaves the heartbeat data unchanged.

## Code references

- Implemented in: (none yet)
