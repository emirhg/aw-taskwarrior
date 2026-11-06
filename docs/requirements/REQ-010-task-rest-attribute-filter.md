# REQ-010: Server-side task filtering in task-rest

- **Status:** Documented, not implemented
- **Added:** 2026-11-06 (server side implemented in task-rest `501b9ff`, verified 2026-11-07)
- **Owner:** maintainer

## Rationale

`GET /` in task-rest returned every task: 581 tasks, about 1.6 MB, and the server ran a full `task export` on each request. The HTTP provider (REQ-007) polls every 5 seconds to find the active task, so every poll moved the whole list.

task-rest now accepts TaskWarrior-style attribute filters as query parameters (task-rest `501b9ff`, `task_rest/filters.py`). The provider can ask for only the active tasks.

## Design

- **Server (done, task-rest `501b9ff`):** `GET /` with filter parameters returns the tasks that match, as `task <filter>` does. Unknown attributes, invalid dates and unsafe values return `400` with the usual `{"message": {...}, }` body, before `task` runs. Parameters are ANDed. The full list is still returned without parameters.
- **Provider (to implement):** every list request sends `?status=pending&start.any=`. That is the same set as TaskWarrior's `+ACTIVE` (pending tasks with a start date), and the provider keeps its client-side selection rule as a safety net (REQ-007). The filter is sent unconditionally, with no setting. An older task-rest ignores the unknown parameters and returns the full list, so the result stays correct, only slower.
- If a server rejects the filter (`400`), the provider reports its message as `Unavailable`, as for any other `400`.

### Verified request and response (task-rest `b7386eb`, TaskWarrior 3.5.0)

| Request | Result |
|---------|--------|
| `GET /` (no parameters) | 13 tasks, including completed, deleted, recurring and waiting ones |
| `GET /?status=pending&start.any=` | 4 tasks: the pending tasks that have a start date. The same uuids as the client-side selection on the unfiltered list. |
| `GET /?foo=bar` | `400`, `Unknown filter "foo". Supported: ...` |

Saved as `crates/aw-taskwarrior-provider-http/tests/fixtures/task_list_active.json` (the filtered response).

## Acceptance criteria

- [x] (task-rest) `GET /?status=pending&start.any=` returns the same tasks as `task status:pending start.any:` on the same data (task-rest tests, verified against the live server).
- [ ] The HTTP provider sends the filter and returns the same `ActiveTask` as the unfiltered path, on the same data.
- [ ] A poll transfers only the active tasks, not the full list (checked against the fixture sizes: the filtered response is a few kilobytes).

## Code references

- Server side: task-rest `task_rest/filters.py`, `task_rest/resources.py` (`TaskListResource.get`), commit `501b9ff`.
- Provider side: (none yet; to be implemented in `crates/aw-taskwarrior-provider-http`, see REQ-007).
