# REQ-004: Performance metrics and health reporting

- **Status:** Documented, not implemented
- **Added:** 2026-11-05
- **Owner:** maintainer

## Rationale

There is no way to see whether the daemon is polling and sending heartbeats successfully without reading logs.

## Design

- Track counters: polls, task query failures, heartbeat failures, last successful heartbeat time.
- Expose them through a log line at a fixed interval. A socket or HTTP endpoint would need a new dependency and a new architecture section first.
- Affects: `main.rs` (poll loop), `activitywatch.rs`, `taskwarrior.rs`. See [../architecture.md](../architecture.md) for the poll-loop sequence.

## Acceptance criteria

- [ ] Counters are updated on every poll.
- [ ] A summary line is written at the configured interval.

## Code references

- Implemented in: (none yet)
