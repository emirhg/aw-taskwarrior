# REQ-013: HTTP provider hardening against task-rest

- **Status:** Implemented
- **Added:** 2026-11-08
- **Owner:** maintainer

## Rationale

A review of the task-rest OpenAPI document (`GET /docs/openapi.json`) against the HTTP provider (REQ-007, REQ-010, REQ-012) found no missing feature for watching the active task. Three small gaps remain: a typo in the `[providers.http]` table is silently ignored, the `/auth` failure paths have no tests, and a task with an unparseable `start` is selected as active without a test saying so.

Tasks always have a `description` in task-rest and TaskWarrior, so a missing `description` stays a parse error. It is not tolerated.

## Design

### Unknown settings are rejected

`build` in `crates/aw-taskwarrior-provider-http/src/lib.rs` accepts only `base_url`, `username`, `password` and `timeout_seconds`. Any other key fails with `InvalidConfig`, the same way the watcher's own section rejects unknown keys (`deny_unknown_fields`):

```text
aw-taskwarrior.providers.http.timeut_seconds: unknown key (allowed: base_url, username, password, timeout_seconds)
```

### Lone unparseable `start`

A pending task whose `start` cannot be parsed as RFC 2822 is still an active task (REQ-010 selection is unchanged). It sorts after every task with a parseable start, so it wins only when no other active task has a valid start. This is kept deliberately: the server already filtered on `start.any=`, so a task with a start is active by definition.

### Error paths of `GET /auth`

| Condition | Result |
|-----------|--------|
| `200` with a body that is not JSON or has no `token` | `InvalidResponse("auth response: ...")` |
| Any other non-2xx status (not 401, not 404) | `Unavailable("authentication failed with HTTP <n>")` |
| No answer before `timeout_seconds` | `Timeout` |

The token and password are never included in any of these messages.

## Acceptance criteria

- [x] An unknown key under `[providers.http]` fails at startup and names the key and the allowed keys.
- [x] A pending task with an unparseable `start` is selected only when no task has a valid start (test).
- [x] A `200` from `/auth` with a non-JSON body gives `InvalidResponse` (loopback mock test).
- [x] A `500` from `/auth` gives "authentication failed with HTTP 500" (loopback mock test).
- [x] A `/auth` request that times out gives `Timeout` (loopback mock test).
- [x] The ignored live test passes against a running task-rest (manual run, 2026-11-08, against `http://127.0.0.1:5000` with a started task).

## Code references

- Implemented in: `crates/aw-taskwarrior-provider-http/src/lib.rs` (`build`, `reject_unknown_keys`, `SETTING_KEYS`, `authenticate`, `select_active`; tests in the same module).
